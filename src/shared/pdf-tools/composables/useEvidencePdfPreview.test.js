import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfPreview } from './useEvidencePdfPreview.js'
import {
  buildHeaderFooterItems,
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

function makePreview({ file, index = 0, rules }) {
  return useEvidencePdfPreview({
    selectedOverlayFile: ref(file),
    selectedOverlayIndex: ref(index),
    previewPage: ref(1),
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
    footerInsertEnabled: ref(false),
    footerEnabled: ref(false),
    footerContinuous: ref(true),
    selectedFooterTextGroup: ref(null),
    totalOverlayPages: ref(3),
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
