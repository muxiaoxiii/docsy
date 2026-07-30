import { describe, expect, it } from 'vitest'
import {
  inferTemplateField,
  looksLikePrefixMark,
  looksLikeCaseNumber,
  looksLikeCauseAction,
  looksLikeCourtName,
  looksLikeDatePrefix,
  looksLikeDateText,
  looksLikeLitigationStage,
  looksLikeLawFirm,
  looksLikeSuffixMark,
  normalizeSuggestionSearchText,
  prefixStructureInfo,
  prefixTargetRule,
  PUBLIC_CAUSE_ACTIONS,
  suffixTargetRule,
} from './publicRules.js'

describe('template public rules', () => {
  it('recognizes court names by seed list and suffix pattern', () => {
    expect(looksLikeCourtName('苏州市中级人民法院')).toBe(true)
    expect(looksLikeCourtName('北京知识产权法院')).toBe(true)
    expect(looksLikeCourtName('浦项股份有限公司')).toBe(false)
  })

  it('recognizes regulated cause action text', () => {
    expect(looksLikeCauseAction('侵害发明专利权纠纷')).toBe(true)
    expect(looksLikeCauseAction('发明专利无效行政纠纷')).toBe(true)
    expect(looksLikeCauseAction('发明专利无效行政纠纷一案')).toBe(true)
    expect(looksLikeCauseAction('浦项股份有限公司')).toBe(false)
  })

  it('loads searchable 2025 cause action data', () => {
    expect(PUBLIC_CAUSE_ACTIONS.length).toBeGreaterThan(900)
    expect(PUBLIC_CAUSE_ACTIONS).toContain('发明专利权权属纠纷')
    expect(PUBLIC_CAUSE_ACTIONS).toContain('侵害发明专利权纠纷')
    expect(
      PUBLIC_CAUSE_ACTIONS.filter((item) => normalizeSuggestionSearchText(item).includes('专利权')).length,
    ).toBeGreaterThan(10)
    expect(normalizeSuggestionSearchText('发明专利权权属、侵权纠纷')).toContain('专利权')
  })

  it('recognizes Chinese case numbers', () => {
    expect(looksLikeCaseNumber('（2026）京73行初6803号')).toBe(true)
    expect(looksLikeCaseNumber('(2026)京73行初6803号')).toBe(true)
    expect(looksLikeCaseNumber('2026年4月23日')).toBe(false)
  })

  it('recognizes complete and blank Chinese dates', () => {
    expect(looksLikeDateText('2026年4月23日')).toBe(true)
    expect(looksLikeDateText('2026年  月  日')).toBe(true)
    expect(looksLikeDateText('年  月  日')).toBe(true)
    expect(looksLikeDateText('____年__月__日')).toBe(true)
    expect(looksLikeDatePrefix('2026年')).toBe(true)
    expect(looksLikeDatePrefix('年月日')).toBe(true)
  })

  it('recognizes litigation stages', () => {
    expect(looksLikeLitigationStage('一审阶段')).toBe(true)
    expect(looksLikeLitigationStage('二审阶段')).toBe(true)
    expect(looksLikeLitigationStage('再审阶段')).toBe(true)
    expect(looksLikeLitigationStage('保全')).toBe(true)
    expect(looksLikeLitigationStage('执行阶段')).toBe(true)
    expect(inferTemplateField({ text: '一审阶段' }).name).toBe('诉讼阶段')
  })

  it('infers common legal fields from text and context', () => {
    expect(inferTemplateField({ text: '苏州市中级人民法院' }).name).toBe('法院')
    expect(inferTemplateField({ text: '侵害发明专利权纠纷' }).name).toBe('案由')
    expect(inferTemplateField({ text: '发明专利无效行政纠纷一案' }).name).toBe('案由')
    expect(inferTemplateField({ text: '（2026）京73行初6803号' }).name).toBe('案号')
    expect(inferTemplateField({ text: '2026年  月  日' }).type).toBe('date')
    expect(looksLikeLawFirm('北京志霖律所')).toBe(true)
    expect(inferTemplateField({ text: '北京志霖律所' }).name).toBe('律所名称')
  })

  it('infers party lists from surrounding role words', () => {
    const context = '原告浦项股份有限公司与被告上海蔚来汽车有限公司，第三人A公司之间侵害发明专利权纠纷一案'

    expect(inferTemplateField({ text: '浦项股份有限公司', context }).name).toBe('原告')
    expect(inferTemplateField({ text: '上海蔚来汽车有限公司', context }).name).toBe('被告')
    expect(inferTemplateField({ text: 'A公司', context }).name).toBe('第三人')
    expect(inferTemplateField({ text: '浦项股份有限公司', context }).semanticKey).toBe('当事人')
    expect(inferTemplateField({ text: '上海蔚来汽车有限公司', context }).semanticKey).toBe('当事人')
    expect(inferTemplateField({ text: 'A公司', context }).semanticKey).toBe('当事人')
  })

  it('keeps lawyer names out of the nearest party list', () => {
    const context = '第三人E公司委托本所锑律师、铁实习律师为一审阶段的诉讼代理人'

    expect(inferTemplateField({ text: '锑', context }).name).toBe('律师')
    expect(inferTemplateField({ text: '铁', context }).name).toBe('律师')
  })

  it('keeps common role prefixes and lawyer suffixes as structure rules', () => {
    expect(looksLikePrefixMark('原告')).toBe(true)
    expect(looksLikePrefixMark('，第三人')).toBe(true)
    expect(prefixTargetRule('，第三人')?.targetName).toBe('第三人')
    expect(prefixTargetRule('，第三人')?.targetSemanticKey).toBe('当事人')
    expect(prefixStructureInfo('，第三人')).toMatchObject({
      connectorText: '，',
      roleText: '第三人',
      removeText: '，第三人',
    })
    expect(prefixStructureInfo('与被告')).toMatchObject({
      connectorText: '与',
      roleText: '被告',
      removeText: '与被告',
    })
    expect(prefixStructureInfo('以及委托代理人')).toMatchObject({
      connectorText: '以及',
      roleText: '委托代理人',
      removeText: '以及委托代理人',
    })
    expect(prefixTargetRule('住所地')?.targetName).toBe('地址')
    expect(prefixTargetRule('统一社会信用代码')?.targetName).toBe('统一社会信用代码')
    expect(looksLikeSuffixMark('律师')).toBe(true)
    expect(looksLikeSuffixMark('实习律师')).toBe(true)
    expect(suffixTargetRule('实习律师')?.targetName).toBe('律师')
  })

  it('uses paragraph context for legal field names', () => {
    expect(inferTemplateField({ text: '京73行初6803号', context: '案号：（2026）京73行初6803号' }).name).toBe('案号')
    expect(inferTemplateField({ text: '朝阳区', context: '送达地址：北京市朝阳区某路1号' }).name).toBe('地址')
    expect(inferTemplateField({ text: '吕晗', context: '委托吕晗律师为一审阶段的诉讼代理人' }).name).toBe('律师')
    expect(inferTemplateField({ text: '吕晗', context: '本所指派吕晗出庭' }).name).toBe('律师')
    expect(inferTemplateField({ text: '一审', context: '委托吕晗律师为一审阶段的诉讼代理人' }).name).toBe('诉讼阶段')
  })
})
