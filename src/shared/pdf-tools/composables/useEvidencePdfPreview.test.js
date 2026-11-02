import { describe, expect, it } from 'vitest'
import { reactive, ref } from 'vue'
import { useEvidencePdfPreview } from './useEvidencePdfPreview.js'
import {
  buildHeaderFooterItems,
  createDefaultFooterTextGroup,
  createDefaultHeaderGroup,
  createDefaultPageNumberGroup,
} from './useEvidencePdfSession.js'

function makeFile(name) {
  return {
    path: `/docs/${name}.pdf`,
    name: `${name}.pdf`,
    pages: 3,
    pageStart: 1,
    headerGroups: [createDefaultHeaderGroup()],
    footerTextGroups: [],
    pageNumberGroups: [],
    selectedHeaderGroupId: 'h1',
  }
}

function makePreview({ file, index = 0, rules, footer = {} }) {
  return useEvidencePdfPreview({
    selectedOverlayFile: ref(file),
    selectedOverlayIndex: ref(index),
    previewPage: ref(footer.page ?? 1),
    previewReloadKey: ref(0),
    previewData: ref(footer.previewData ?? null),
    truePreview: ref(null),
    truePreviewLoading: ref(false),
    previewMaxPage: ref(1),
    mergedImportPlan: ref(null),
    overlayRows: ref([...Array.from({ length: index }, (_, i) => makeFile(`前序${i}`)), file]),
    currentRules: ref(rules),
    overlayOutputDir: ref(''),
    insertHeaderFooterEnabled: ref(true),
    headerInsertEnabled: ref(true),
    headerMode: ref(rules.headerMode),
    footerInsertEnabled: ref(footer.insertEnabled ?? false),
    footerEnabled: ref(false),
    footerContinuous: ref(footer.continuous ?? true),
    selectedFooterTextGroup: ref(footer.group ?? null),
    totalOverlayPages: ref(footer.totalPages ?? 3),
    headerAlign: ref('right'),
    headerMarginMm: ref(10),
    headerFontSize: ref(10),
    headerFontFamily: ref('auto'),
    headerOffsetXMm: ref(0),
    headerColor: ref('#000000'),
    footerAlign: ref('left'),
    footerMarginMm: ref(10),
    footerFontSize: ref(9),
    footerFontFamily: ref('auto'),
    footerOffsetXMm: ref(0),
    footerColor: ref('#000000'),
    pageNumberStyle: ref('arabic'),
    removeAnnotations: ref(false),
    annotationKinds: ref([]),
    cleanupHeaderHeightMm: ref(18),
    cleanupFooterHeightMm: ref(18),
    selectedFooterCandidateKey: ref(''),
  })
}

function globalRules(group) {
  return {
    _globalApply: true,
    _globalHeaderGroup: group,
    headerMode: group.mode,
    headerInsertEnabled: true,
  }
}

describe('previewHeaderText matches the generated header', () => {
  it.each([true, false])('uses the live evidence start with global apply %s', (globalApply) => {
    const files = [makeFile('证据甲'), makeFile('证据乙')]
    const group = { ...createDefaultHeaderGroup(), mode: 'per_file' }
    files.forEach((file) => { file.headerGroups = [group] })
    const defaults = reactive({ evidenceStart: 21, pageStart: 1 })
    const rules = { ...globalRules(group), _globalApply: globalApply, numberingDefaults: defaults }
    if (!globalApply) delete rules._globalHeaderGroup
    const preview = makePreview({ file: files[1], index: 1, rules })
    expect(preview.previewHeaderText.value).toBe('证据22')
    expect(preview.previewHeaderText.value).toBe(buildHeaderFooterItems(files, rules)[1].header.text)
    defaults.evidenceStart = 41
    expect(preview.previewHeaderText.value).toBe('证据42')
    expect(preview.previewHeaderText.value).toBe(buildHeaderFooterItems(files, rules)[1].header.text)
  })

  it('uses a rule-specific evidence start instead of the global default', () => {
    const file = makeFile('证据甲')
    const group = { ...createDefaultHeaderGroup(), mode: 'per_file', numbering: { source: 'custom', evidenceStart: 75 } }
    const rules = { ...globalRules(group), numberingDefaults: { evidenceStart: 21, pageStart: 1 } }
    const preview = makePreview({ file, rules })
    expect(preview.previewHeaderText.value).toBe('证据75')
    expect(preview.previewHeaderText.value).toBe(buildHeaderFooterItems([file], rules)[0].header.text)
  })

  it('global apply + fixed text: preview shows the fixed text, not the file name or the detected old header', () => {
    const file = makeFile('合同扫描件')
    // A previously detected existing header must not leak into the preview.
    file.existingHeaderText = '证据１'
    const group = { ...createDefaultHeaderGroup(), mode: 'custom', text: '内部机密' }
    const rules = globalRules(group)
    const preview = makePreview({ file, rules })
    const actual = buildHeaderFooterItems([file], rules)[0].header?.text
    expect(actual).toBe('内部机密')
    expect(preview.previewHeaderText.value).toBe(actual)
  })

  it('global apply + per-file evidence list: preview shows the sequenced label', () => {
    const files = [makeFile('原告证据一'), makeFile('原告证据二')]
    const group = { ...createDefaultHeaderGroup(), mode: 'per_file' }
    const rules = globalRules(group)
    const items = buildHeaderFooterItems(files, rules)
    files.forEach((file, index) => {
      const preview = makePreview({ file, index, rules })
      expect(preview.previewHeaderText.value).toBe(items[index].header?.text)
      expect(preview.previewHeaderText.value).toBe(`证据${index + 1}`)
    })
  })

  it('per-file mode with global apply OFF: preview uses the file’s own group', () => {
    const file = makeFile('鉴定意见')
    file.headerGroups[0].mode = 'custom'
    file.headerGroups[0].text = '证据[##]'
    const rules = { headerMode: undefined, headerInsertEnabled: true }
    const preview = makePreview({ file, index: 2, rules })
    expect(preview.previewHeaderText.value).toBe('证据03')
  })
})

function globalFooterRules(group) {
  return {
    _globalApply: true,
    _globalFooterTextGroup: group,
    footerInsertEnabled: true,
    footerTextContent: group.text,
  }
}

describe('previewFooterText matches the generated footer', () => {
  it('global apply + rule text: preview shows the rule text with placeholders expanded, not the detected old footer', () => {
    const file = makeFile('合同扫描件')
    // A previously detected existing footer must not leak into the preview.
    file.existingFooterText = '旧页脚-第1页'
    const group = { ...createDefaultFooterTextGroup(), text: '第{page}页，共{total}页' }
    const rules = globalFooterRules(group)
    const preview = makePreview({
      file,
      rules,
      footer: { insertEnabled: true, group },
    })
    const actual = buildHeaderFooterItems([file], rules)[0].footer?.text
    // The job keeps {page}/{total} for the backend to expand per page.
    expect(actual).toBe('第{page}页，共{total}页')
    expect(preview.previewFooterText.value).toBe('第1页，共3页')
  })

  it('per-file mode with global apply OFF: preview uses the file’s own footer group', () => {
    const file = makeFile('鉴定意见')
    file.existingFooterText = '检测到的旧页脚'
    file.footerTextGroups = [{ ...createDefaultFooterTextGroup(), text: '[name] 第{page}页' }]
    file.selectedFooterTextGroupId = 'ft1'
    const rules = { footerInsertEnabled: true }
    const preview = makePreview({
      file,
      rules,
      footer: { insertEnabled: true, group: file.footerTextGroups[0] },
    })
    const actual = buildHeaderFooterItems([file], rules)[0].footer?.text
    expect(actual).toBe('鉴定意见 第{page}页')
    expect(preview.previewFooterText.value).toBe('鉴定意见 第1页')
  })

  it('footer text placeholders follow the global page coordinate used by export', () => {
    const files = [makeFile('原告证据一'), makeFile('原告证据二')]
    files[1].pageStart = 4
    const group = { ...createDefaultFooterTextGroup(), text: '{page}/{total}' }
    const rules = globalFooterRules(group)
    const continuous = makePreview({
      file: files[1],
      index: 1,
      rules,
      footer: { insertEnabled: true, group, page: 2, continuous: true, totalPages: 6 },
    })
    expect(continuous.previewFooterText.value).toBe('5/6')
    const perFile = makePreview({
      file: files[1],
      index: 1,
      rules,
      footer: { insertEnabled: true, group, page: 2, continuous: false, totalPages: 6 },
    })
    expect(perFile.previewFooterText.value).toBe('5/6')
  })
})

describe('previewPageInfo 页尺寸回退链', () => {
  const rules = { headerMode: 'custom', headerInsertEnabled: true }

  it('previewData 有真实页尺寸时优先使用（异形页）', () => {
    const file = makeFile('宽页')
    file.existingHeaderBBox = { x0: 10, y0: 10, x1: 200, y1: 30, width: 1000, height: 500 }
    const preview = makePreview({
      file,
      rules,
      footer: { previewData: { widthPt: 1200, heightPt: 600 } },
    })
    expect(preview.previewPageInfo.value).toEqual({ widthPt: 1200, heightPt: 600 })
    // overlay 字号按真实页宽缩放：10pt / 1200pt * 100 ≈ 0.8333cqw
    expect(preview.previewHeaderStyle.value.fontSize).toBe('max(8px, 0.8333cqw)')
  })

  it('previewData 缺失时回退到文件检测 bbox 携带的页尺寸', () => {
    const file = makeFile('检测页')
    file.existingHeaderBBox = { x0: 10, y0: 10, x1: 200, y1: 30, width: 1000, height: 500 }
    const preview = makePreview({ file, rules })
    expect(preview.previewPageInfo.value).toEqual({ widthPt: 1000, heightPt: 500 })
    expect(preview.previewHeaderStyle.value.fontSize).toBe('max(8px, 1cqw)')
  })

  it('两者都没有时回退 null，由 textOverlayStyle 兜底 A4', () => {
    const file = makeFile('无信息')
    const preview = makePreview({ file, rules })
    expect(preview.previewPageInfo.value).toBeNull()
    expect(preview.previewHeaderStyle.value.fontSize).toBe(`max(8px, ${Number(((10 / 595.28) * 100).toFixed(4))}cqw)`)
  })
})


describe('existing element deletion preview', () => {
  it('follows element decisions and keeps rotated header and page number inside the visible page', () => {
    const file = reactive(makeFile('旋转页'))
    file.pages = 27
    file.removeExistingHeader = false
    file.removeExistingPageNumber = false
    file.existingElements = [
      { id: 'old-header', kind: 'header', decision: 'delete', pageStart: 1, pageEnd: 27,
        bbox: { x0: 530, x1: 592, y0: 13, y1: 38, width: 842.4, height: 595.44 } },
      { id: 'old-number', kind: 'pageNumber', decision: 'delete', pageStart: 1, pageEnd: 27,
        bbox: { x0: 281, x1: 315, y0: 813, y1: 838, width: 842.4, height: 595.44 } },
    ]
    const preview = makePreview({ file, rules: {}, footer: { previewData: { widthPt: 595.44, heightPt: 842.4 } } })
    const markers = preview.deletionPreviewMarkers.value
    expect(markers).toHaveLength(2)
    expect(parseFloat(markers[0].style.left)).toBeGreaterThan(89)
    expect(parseFloat(markers[0].style.width)).toBeCloseTo(62 / 595.44 * 100)
    expect(parseFloat(markers[1].style.top)).toBeGreaterThan(96)
    expect(parseFloat(markers[1].style.top)).toBeLessThan(100)
    expect(markers[0].captionBelow).toBe(true)
    expect(markers[1].captionBelow).toBe(false)
    file.existingElements[0].decision = 'keep'
    expect(preview.deletionPreviewMarkers.value.map(marker => marker.key)).toEqual(['old-number'])
  })

  it('respects each element range and resolves an open end to the file length', () => {
    const file = makeFile('页段')
    file.existingElements = [
      { id: 'a', kind: 'header', decision: 'edit', pageStart: 2, pageEnd: 0 },
      { id: 'b', kind: 'footerText', decision: 'delete', pageStart: 1, pageEnd: 1 },
      { id: 'c', kind: 'pageNumber', decision: 'ignore', pageStart: 1, pageEnd: 3 },
    ]
    const preview = makePreview({ file, rules: {}, footer: { page: 3 } })
    expect(preview.deletionPreviewMarkers.value.map(marker => marker.key)).toEqual(['a'])
  })
})

describe('header overrides and page exceptions in the live preview', () => {
  it('previews per-file page numbers in the header even when header insertion is off', () => {
    const file = makeFile('第二份')
    const pn = { ...createDefaultPageNumberGroup(), region: 'header', template: '{page}', numbering: { pageStart: 1, sequence: 'per-file', totalMode: 'file' } }
    const rules = { _globalApply: true, headerInsertEnabled: false, pageNumberEnabled: true, pageNumberGroups: [pn], _globalPageNumberGroup: pn }
    const preview = makePreview({ file, index: 1, rules })
    expect(preview.convertedExistingPreviewOverlays.value.map(item => item.text)).toContain('1')
  })
  it('previews an edited old header while new header insertion is off', () => {
    const file = makeFile('原件')
    file.convertPlainHeader = true
    file.existingHeaderText = '修改旧页眉'
    const preview = makePreview({ file, rules: { headerInsertEnabled: false, footerInsertEnabled: false, pageNumberEnabled: false } })
    expect(preview.convertedExistingPreviewOverlays.value.map(item => item.text)).toContain('修改旧页眉')
  })
  it('shows only the applicable page segment and does not duplicate the basic header', () => {
    const file = makeFile('证据')
    const group = { ...createDefaultHeaderGroup(), mode: 'custom', text: '普通', fileHeaderOverrides: { [file.path]: '逐文件页眉' } }
    const rules = { ...globalRules(group), headerGroups: [group], footerInsertEnabled: false, pageNumberEnabled: false,
      insertExceptions: [{ kinds: ['header'], scope: { type: 'file', start: 2, end: 2 }, overrides: { headerText: '第二页例外' } }] }
    const preview = makePreview({ file, rules, footer: { page: 2 } })
    expect(preview.previewHeaderText.value).toBe('')
    expect(preview.convertedExistingPreviewOverlays.value.filter(item => item.region === 'header').map(item => item.text)).toEqual(['第二页例外'])
    const first = makePreview({ file, rules, footer: { page: 1 } })
    expect(first.previewHeaderText.value).toBe('')
    expect(first.convertedExistingPreviewOverlays.value.filter(item => item.region === 'header').map(item => item.text)).toEqual(['逐文件页眉'])
  })
})
