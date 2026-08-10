import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfPreview } from './useEvidencePdfPreview.js'
import {
  buildHeaderFooterItems,
  createDefaultFooterTextGroup,
  createDefaultHeaderGroup,
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
    previewData: ref(null),
    truePreview: ref(null),
    truePreviewLoading: ref(false),
    previewMaxPage: ref(1),
    mergedImportPlan: ref(null),
    overlayRows: ref([file]),
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

  it('continuous vs per-file page numbering follows footerContinuous', () => {
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
    expect(perFile.previewFooterText.value).toBe('2/3')
  })
})
