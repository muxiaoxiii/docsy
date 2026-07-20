import { PUBLIC_CAUSE_ACTIONS_2025 } from './causeActions2025.js'
import { PUBLIC_COURT_NAMES } from './courtNames.js'

const COURT_NAME_SUFFIX_RE = /(?:人民法院|知识产权法院|金融法院|互联网法院|海事法院|军事法院|铁路运输法院)$/
const CASE_NUMBER_RE =
  /^[（(]\s*\d{4}\s*[）)]\s*[\u4e00-\u9fa5A-Za-z0-9]{1,12}(?:民|行|知|执|赔|破|清|申|再|终|初|保|诉前|民终|民初|行初|行终)[\u4e00-\u9fa5A-Za-z0-9-]*号$/
const DATE_TEXT_RE =
  /^((?:\d{2,4}|[一二三四五六七八九零〇]{2,4}|[_＿]{2,4})?\s*年\s*(?:\d{1,2}|[一二三四五六七八九十]{1,3}|[_＿]{1,2})?\s*月\s*(?:\d{1,2}|[一二三四五六七八九十]{1,3}|[_＿]{1,2})?\s*日?|(?:\d{4})[-/.年](?:\d{1,2})[-/.月](?:\d{1,2})日?)$/
const DATE_FRAGMENT_RE =
  /^((?:\d{1,4}|[一二三四五六七八九零〇十]{1,4}|[_＿]{1,4})?(?:年)?(?:\d{0,2}|[一二三四五六七八九十]{0,3}|[_＿]{0,2})?(?:月)?(?:\d{0,2}|[一二三四五六七八九十]{0,3}|[_＿]{0,2})?(?:日)?|年|月|日|年月|年月日)$/
const ORGANIZATION_SUFFIX_RE =
  /(公司|有限公司|股份有限公司|集团|事务所|委员会|管理局|知识产权局|银行|大学|研究院|中心)$/

export const PUBLIC_ROLE_PREFIX_RULES = [
  { text: '原告', targetName: '原告', targetLabel: '原告', targetType: 'party_list', targetSemanticKey: '当事人' },
  { text: '被告', targetName: '被告', targetLabel: '被告', targetType: 'party_list', targetSemanticKey: '当事人' },
  {
    text: '第三人',
    targetName: '第三人',
    targetLabel: '第三人',
    targetType: 'party_list',
    targetSemanticKey: '当事人',
  },
  {
    text: '申请人',
    targetName: '申请人',
    targetLabel: '申请人',
    targetType: 'party_list',
    targetSemanticKey: '当事人',
  },
  {
    text: '被申请人',
    targetName: '被申请人',
    targetLabel: '被申请人',
    targetType: 'party_list',
    targetSemanticKey: '当事人',
  },
  {
    text: '上诉人',
    targetName: '上诉人',
    targetLabel: '上诉人',
    targetType: 'party_list',
    targetSemanticKey: '当事人',
  },
  {
    text: '被上诉人',
    targetName: '被上诉人',
    targetLabel: '被上诉人',
    targetType: 'party_list',
    targetSemanticKey: '当事人',
  },
  {
    text: '法定代表人',
    targetName: '法定代表人',
    targetLabel: '法定代表人',
    targetType: 'text',
    targetSemanticKey: '法定代表人',
  },
  { text: '负责人', targetName: '负责人', targetLabel: '负责人', targetType: 'text', targetSemanticKey: '负责人' },
  { text: '委托代理人', targetName: '代理人', targetLabel: '代理人', targetType: 'party_list' },
  { text: '承办法官', targetName: '承办法官', targetLabel: '承办法官', targetType: 'text' },
  { text: '审判长', targetName: '审判长', targetLabel: '审判长', targetType: 'text' },
  { text: '书记员', targetName: '书记员', targetLabel: '书记员', targetType: 'text' },
  { text: '住所地', targetName: '地址', targetLabel: '地址', targetType: 'text' },
  { text: '注册地', targetName: '注册地', targetLabel: '注册地', targetType: 'text' },
  { text: '经营地', targetName: '经营地', targetLabel: '经营地', targetType: 'text' },
  { text: '统一社会信用代码', targetName: '统一社会信用代码', targetLabel: '统一社会信用代码', targetType: 'text' },
  { text: '身份证号码', targetName: '身份证号', targetLabel: '身份证号', targetType: 'text' },
  { text: '护照号码', targetName: '护照号', targetLabel: '护照号', targetType: 'text' },
]

export const PUBLIC_SUFFIX_RULES = [
  { text: '律师', targetName: '律师', targetLabel: '律师', targetType: 'party_list' },
  { text: '实习律师', targetName: '律师', targetLabel: '律师', targetType: 'party_list' },
  { text: '代理人', targetName: '代理人', targetLabel: '代理人', targetType: 'text' },
  { text: '诉讼代理人', targetName: '代理人', targetLabel: '代理人', targetType: 'text' },
]

export { PUBLIC_COURT_NAMES } from './courtNames.js'

const SUPPLEMENTAL_CAUSE_ACTIONS = ['发明专利无效行政纠纷', '实用新型专利无效行政纠纷', '外观设计专利无效行政纠纷']

export const PUBLIC_CAUSE_ACTIONS = Array.from(new Set([...PUBLIC_CAUSE_ACTIONS_2025, ...SUPPLEMENTAL_CAUSE_ACTIONS]))

export const PUBLIC_LITIGATION_STAGES = [
  '一审阶段',
  '二审阶段',
  '再审阶段',
  '保全阶段',
  '执行阶段',
  '无效阶段',
  '复审阶段',
  '仲裁阶段',
]

export function normalizeRuleText(text) {
  return String(text || '')
    .replace(/\s+/g, '')
    .trim()
}

export function normalizeSuggestionSearchText(text) {
  return normalizeRuleText(text)
    .replace(/[^\p{L}\p{N}]/gu, '')
    .toLowerCase()
}

export function looksLikeCourtName(text) {
  const value = normalizeRuleText(text).replace(/[：:，,。；;]$/, '')
  return PUBLIC_COURT_NAMES.includes(value) || COURT_NAME_SUFFIX_RE.test(value)
}

export function looksLikeCaseNumber(text) {
  return CASE_NUMBER_RE.test(normalizeRuleText(text))
}

export function looksLikeDateText(text) {
  const value = normalizeRuleText(text)
  if (!value) return false
  if (/^(年|月|日)$/.test(value)) return true
  return DATE_TEXT_RE.test(value)
}

export function looksLikeDatePrefix(text) {
  const value = normalizeRuleText(text)
  if (!value) return false
  if (looksLikeDateText(value)) return true
  return DATE_FRAGMENT_RE.test(value) && /[年月日\d一二三四五六七八九十零〇_＿]/.test(value)
}

export function looksLikeCauseAction(text) {
  const value = normalizeRuleText(text)
    .replace(/[，,。；;：:]$/, '')
    .replace(/一案$/, '')
  if (PUBLIC_CAUSE_ACTIONS.includes(value)) return true
  if (value.length < 4 || value.length > 32) return false
  if (ORGANIZATION_SUFFIX_RE.test(value)) return false
  return (
    /(纠纷|案件)$/.test(value) && /(合同|侵害|专利|商标|著作权|竞争|劳动|行政|公司|股权|损害|保全|执行)/.test(value)
  )
}

export function looksLikeLawFirm(text) {
  return /(?:律师事务所|事务所|律所)$/.test(normalizeRuleText(text))
}

export function looksLikeLitigationStage(text) {
  const value = normalizeRuleText(text)
  if (PUBLIC_LITIGATION_STAGES.includes(value)) return true
  return /^(一审|二审|再审|保全|执行|无效|复审|仲裁)(阶段|程序)?$/.test(value)
}

export function looksLikePrefixMark(text) {
  const info = prefixStructureInfo(text)
  return Boolean(info.rule) || /[：:]$/.test(info.roleText)
}

export function looksLikeSuffixMark(text) {
  const value = normalizeRuleText(text)
  return Boolean(suffixTargetRule(value)) || /^[）)]$/.test(value)
}

export function prefixTargetRule(text) {
  return prefixStructureInfo(text).rule
}

export function prefixStructureInfo(text) {
  const raw = normalizeRuleText(text)
  const connectorMatch = raw.match(/^(?:以及|或者|[，,、;；和与及\s])+/)
  const connectorText = connectorMatch?.[0] || ''
  const roleText = raw.slice(connectorText.length)
  const rule = PUBLIC_ROLE_PREFIX_RULES.find((item) => item.text === roleText) || null
  return {
    connectorText,
    roleText,
    removeText: raw,
    rule,
  }
}

export function suffixTargetRule(text) {
  const value = normalizeRuleText(text)
  return PUBLIC_SUFFIX_RULES.find((item) => item.text === value) || null
}

export function inferTemplateField({ text, context = '', checkboxLike = false, index = 0 }) {
  const value = normalizeRuleText(text)
  if (checkboxLike) return buildInference('checkbox', `勾选${index + 1}`, text)
  if (looksLikeDateText(value)) return buildInference('date', '日期', '日期')
  if (looksLikeCourtName(value)) return buildInference('text', '法院', '法院')
  if (looksLikeCaseNumber(value)) return buildInference('text', '案号', '案号')
  if (looksLikeCauseAction(value)) return buildInference('text', '案由', '案由')
  if (looksLikeLitigationStage(value)) return buildInference('text', '诉讼阶段', '诉讼阶段')
  if (looksLikeLawFirm(value)) return buildInference('text', '律所名称', '律所名称')

  const suffixIdentityRole = inferSuffixIdentityRole(value, context)
  if (suffixIdentityRole) {
    return buildInference(
      suffixIdentityRole.type,
      suffixIdentityRole.name,
      suffixIdentityRole.label,
      suffixIdentityRole.semanticKey,
    )
  }

  const partyRole = inferPartyRole(value, context)
  if (partyRole) return buildInference('party_list', partyRole.name, partyRole.label, partyRole.semanticKey)

  const contextRole = inferContextRole(value, context)
  if (contextRole) return buildInference(contextRole.type, contextRole.name, contextRole.label)

  return buildInference('text', `字段${index + 1}`, text)
}

function buildInference(type, name, label, semanticKey = name) {
  return {
    type,
    name,
    label: String(label || name || ''),
    semanticKey,
    optionalWhenEmpty: false,
    optionalScope: 'position',
    optionalPrefix: '',
    optionalSuffix: '',
  }
}

function inferPartyRole(value, context) {
  const normalizedContext = normalizeRuleText(context)
  const valueIndex = normalizedContext.indexOf(value)
  if (valueIndex < 0) return null
  const rolePatterns = [
    ...PUBLIC_ROLE_PREFIX_RULES.map((item) => ({
      role: item.text,
      name: item.targetName,
      label: item.targetLabel,
      semanticKey: item.targetSemanticKey || item.targetName,
    })),
  ]
  let nearest = null
  for (const item of rolePatterns) {
    const roleIndex = normalizedContext.lastIndexOf(item.role, valueIndex)
    if (roleIndex >= 0 && roleIndex < valueIndex && valueIndex - roleIndex <= 120) {
      if (!nearest || roleIndex > nearest.roleIndex) nearest = { ...item, roleIndex }
    }
  }
  return nearest
}

function inferSuffixIdentityRole(value, context) {
  const normalizedContext = normalizeRuleText(context)
  if (!value || value.length > 8) return null
  const suffixRules = PUBLIC_SUFFIX_RULES.filter((rule) => rule.targetType === 'party_list')
  for (const rule of suffixRules) {
    if (normalizedContext.includes(`${value}${rule.text}`)) {
      return {
        type: rule.targetType,
        name: rule.targetName,
        label: rule.targetLabel,
        semanticKey: rule.targetSemanticKey || rule.targetName,
      }
    }
  }
  if (/指派|出庭|承办/.test(normalizedContext)) {
    const valueIndex = normalizedContext.indexOf(value)
    if (valueIndex > 0) {
      const before = normalizedContext.slice(Math.max(0, valueIndex - 10), valueIndex)
      if (/指派|出庭|承办/.test(before)) {
        return { type: 'party_list', name: '律师', label: '律师', semanticKey: '律师' }
      }
    }
  }
  return null
}

function inferContextRole(value, context) {
  const { before, after, all } = contextAroundValue(value, context)

  if ((/案号|案卷号|文书号/.test(all) && isReasonableShortValue(value)) || /[（(]\d{4}[）)]/.test(before + value)) {
    return { type: 'text', name: '案号', label: '案号' }
  }
  if (/案由/.test(all) && isReasonableShortValue(value)) {
    return { type: 'text', name: '案由', label: '案由' }
  }
  if ((/[致送呈报至][:：]?$/.test(before) || /^[:：]?/.test(after)) && /法院/.test(value + after.slice(0, 8))) {
    return { type: 'text', name: '法院', label: '法院' }
  }
  if (/住所|地址|送达地址|联系地址/.test(all) && value.length >= 2) {
    return { type: 'text', name: '地址', label: '地址' }
  }
  if (
    looksLikeLitigationStage(value) ||
    (/(一审|二审|再审|保全|执行|无效|复审|仲裁)/.test(value) && /阶段|程序|案件/.test(all))
  ) {
    return { type: 'text', name: '诉讼阶段', label: '诉讼阶段' }
  }
  return null
}

function contextAroundValue(value, context) {
  const normalizedContext = normalizeRuleText(context)
  const valueIndex = normalizedContext.indexOf(value)
  if (!value || valueIndex < 0) return { before: '', after: '', all: normalizedContext }
  return {
    before: normalizedContext.slice(Math.max(0, valueIndex - 40), valueIndex),
    after: normalizedContext.slice(valueIndex + value.length, valueIndex + value.length + 40),
    all: normalizedContext,
  }
}

function isReasonableShortValue(value) {
  return Boolean(value && value.length >= 1 && value.length <= 40)
}
