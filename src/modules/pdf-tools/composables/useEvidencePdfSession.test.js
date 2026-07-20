import { describe, expect, it } from 'vitest'
import {
  assignPageRanges,
  buildEvidencePdfRulePayload,
  buildHeaderFooterItems,
  buildHeaderText,
  buildMergeOutputPath,
  buildOutputDir,
  canWriteFooter,
  canWriteHeader,
  candidateTargetRange,
  createEvidenceFile,
  expandPlaceholders,
  naturalCompare,
  pageRangeText,
  sortByNatural,
  toChineseNumber,
  totalPages,
  updatePageRanges,
} from './useEvidencePdfSession.js'

const baseRules = {
  normalizeA4: false,
  a4Orientation: 'auto',
  rasterDpi: 200,
  cleanupHeaderEnabled: true,
  cleanupFooterEnabled: true,
  cleanupHeaderHeightMm: 18,
  cleanupFooterHeightMm: 18,
  headerMode: 'per_file',
  headerText: '',
  headerAlign: 'right',
  headerFontSize: 10,
  headerMarginMm: 10,
  footerEnabled: true,
  footerText: '{page}/{total}',
  footerAlign: 'center',
  footerFontSize: 9,
  footerMarginMm: 10,
  removeAnnotations: false,
  annotationKinds: [],
  outputMode: 'files_only',
}

const noInsertionRules = {
  ...baseRules,
  headerMode: 'none',
  footerEnabled: false,
}

describe('Evidence PDF session helpers', () => {
  it('assigns continuous page ranges without mutating source files', () => {
    const files = [
      { ...createEvidenceFile('/tmp/证据1.pdf'), pages: 3 },
      { ...createEvidenceFile('/tmp/证据2.pdf'), pages: 2 },
    ]

    const ranged = assignPageRanges(files)

    expect(ranged[0].pageStart).toBe(1)
    expect(ranged[0].pageEnd).toBe(3)
    expect(ranged[1].pageStart).toBe(4)
    expect(ranged[1].pageEnd).toBe(6 - 1)
    expect(files[1].pageStart).toBe(1)
    expect(files[1].pageEnd).toBe(0)
    expect(pageRangeText(ranged[1])).toBe('4-5')
    expect(totalPages(files)).toBe(5)
  })

  it('updates page ranges in place for editable UI rows', () => {
    const files = [
      { ...createEvidenceFile('/tmp/证据1.pdf'), pages: 3 },
      { ...createEvidenceFile('/tmp/证据2.pdf'), pages: 2 },
    ]

    const updated = updatePageRanges(files)

    expect(updated[0]).toBe(files[0])
    expect(files[0].pageStart).toBe(1)
    expect(files[0].pageEnd).toBe(3)
    expect(files[1].pageStart).toBe(4)
    expect(files[1].pageEnd).toBe(5)
  })

  it('builds per-file headers and merged total page footer jobs', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, header: '证据1 合同' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, header: '证据2 付款' },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items).toHaveLength(2)
    expect(items[0].header.text).toBe('证据1 合同')
    expect(items[0].pageStart).toBe(1)
    expect(items[0].totalPages).toBe(6)
    expect(items[1].pageStart).toBe(3)
    expect(items[1].footer.text).toBe('{page}/{total}')
    expect(items[1].outputPath).toBe('/out/付款_processed.pdf')
  })

  it('keeps an intentionally blank per-file header', () => {
    const file = { ...createEvidenceFile('/case/合同.pdf'), header: '' }

    expect(buildHeaderText(file, 0, baseRules)).toBe('')
  })

  it('sorts Chinese file labels in natural numeric order', () => {
    const values = ['证据11-1.pdf', '证据2.pdf', '证据1.pdf', '证据10.pdf']

    expect([...values].sort(naturalCompare)).toEqual(['证据1.pdf', '证据2.pdf', '证据10.pdf', '证据11-1.pdf'])
  })

  it('sorts rows by natural values while preserving ties', () => {
    const rows = [{ name: '1-10' }, { name: '1-2' }, { name: '11-20' }, { name: '1-2' }]

    expect(sortByNatural(rows, (row) => row.name).map((row) => row.name)).toEqual(['1-2', '1-2', '1-10', '11-20'])
    expect(sortByNatural(rows, (row) => row.name, 'descending').map((row) => row.name)).toEqual([
      '11-20',
      '1-10',
      '1-2',
      '1-2',
    ])
  })

  it('supports per-file footer overrides including blank values', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, footer: '第{page}页' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, footer: '' },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].footer.text).toBe('第{page}页')
    expect(items[1].footer).toBeNull()
  })

  it('keeps detected old text separate from new header and footer insertion', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        header: 'new header',
        footer: 'new footer',
        existingHeaderText: 'old plain header',
        existingFooterText: 'old plain footer',
        existingHeaderArtifact: false,
        existingFooterArtifact: false,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(canWriteHeader(files[0])).toBe(true)
    expect(canWriteFooter(files[0])).toBe(true)
    expect(items[0].cleanup.plainHeaderTargets).toHaveLength(0)
    expect(items[0].cleanup.plainFooterTargets).toHaveLength(0)
    expect(items[0].header.text).toBe('new header')
    expect(items[0].footer.text).toBe('new footer')
  })

  it('rejects header and footer write checks without an input path', () => {
    expect(canWriteHeader(null)).toBe(false)
    expect(canWriteFooter({})).toBe(false)
  })

  it('converts confirmed plain text headers and footers into cleanup targets plus independent overlays', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 3,
        existingHeaderText: 'new header',
        existingHeaderTargetText: 'old plain header',
        existingHeaderNormalizedText: 'oldplainheader',
        existingHeaderPageStart: 1,
        existingHeaderPageEnd: 3,
        existingFooterText: 'new footer',
        existingFooterTargetText: '1/3',
        existingFooterNormalizedText: '{page}/{total}',
        existingFooterPageStart: 1,
        existingFooterPageEnd: 3,
        convertPlainHeader: true,
        convertPlainFooter: true,
      },
    ]

    const items = buildHeaderFooterItems(files, noInsertionRules, '/out')

    expect(canWriteHeader(files[0])).toBe(true)
    expect(canWriteFooter(files[0])).toBe(true)
    expect(items[0].header).toBeNull()
    expect(items[0].footer).toBeNull()
    expect(items[0].extraOverlays.map((item) => item.text)).toEqual(['new header', 'new footer'])
    expect(items[0].extraOverlays[0]).toMatchObject({ pageStart: 1, pageEnd: 3 })
    expect(items[0].extraOverlays[1]).toMatchObject({ pageStart: 1, pageEnd: 3 })
    expect(items[0].cleanup.plainHeaderTargets[0]).toMatchObject({
      text: 'old plain header',
      normalizedText: 'oldplainheader',
      pageStart: 1,
      pageEnd: 3,
    })
    expect(items[0].cleanup.plainFooterTargets[0]).toMatchObject({
      text: '1/3',
      normalizedText: '{page}/{total}',
      pageStart: 1,
      pageEnd: 3,
    })
  })

  it('uses edited plain text header as replacement while deleting the original detected target', () => {
    const files = [
      {
        ...createEvidenceFile('/case/测试页眉3.pdf'),
        pages: 1,
        existingHeaderText: '新测试页眉3',
        existingHeaderTargetText: '测试页眉3',
        existingHeaderNormalizedText: '测试页眉3',
        existingHeaderBBox: {
          x0: 505,
          y0: 15,
          x1: 578,
          y1: 35,
          page: 1,
          width: 595,
          height: 842,
        },
        convertPlainHeader: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].cleanup.plainHeaderTargets[0]).toMatchObject({
      text: '测试页眉3',
      normalizedText: '测试页眉3',
      pageStart: 1,
      pageEnd: 1,
    })
    expect(items[0].cleanup.plainHeaderTargets[0].bbox).toMatchObject({ x0: 505, y0: 15 })
    expect(items[0].header.text).toBe('测试页眉3')
    expect(items[0].extraOverlays[0].text).toBe('新测试页眉3')
    expect(items[0].extraOverlays[0].align).toBe('right')
    expect(items[0].extraOverlays[0].fontSize).toBeCloseTo(11.6, 2)
    expect(items[0].extraOverlays[0].marginMm).toBeCloseTo(12.347, 2)
  })

  it('keeps existing page number separate from existing footer text when editing old page numbers', () => {
    const files = [
      {
        ...createEvidenceFile('/case/测试页眉3.pdf'),
        pages: 1,
        existingFooterText: '页脚说明',
        existingFooterTargetText: '页脚说明',
        existingFooterNormalizedText: '页脚说明',
        existingFooterBBox: {
          x0: 50,
          y0: 800,
          x1: 120,
          y1: 820,
          page: 1,
          width: 595,
          height: 842,
        },
        existingPageNumberText: '第1页',
        existingPageNumberTargetText: '1',
        existingPageNumberNormalizedText: '{page}',
        existingPageNumberBBox: {
          x0: 535,
          y0: 806,
          x1: 545,
          y1: 820,
          page: 1,
          width: 595,
          height: 842,
        },
        convertPlainPageNumber: true,
      },
    ]

    const items = buildHeaderFooterItems(files, noInsertionRules, '/out')

    expect(items[0].cleanup.plainFooterTargets).toHaveLength(1)
    expect(items[0].cleanup.plainFooterTargets[0]).toMatchObject({
      text: '1',
      normalizedText: '{page}',
    })
    expect(items[0].extraOverlays).toHaveLength(1)
    expect(items[0].extraOverlays[0]).toMatchObject({
      region: 'footer',
      text: '第1页',
      align: 'right',
      fontSize: 10.08,
    })
  })

  it('can delete a confirmed plain text header without inserting a replacement', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        header: '',
        existingHeaderText: 'old plain header',
        existingHeaderPageStart: 1,
        existingHeaderPageEnd: 2,
        convertPlainHeader: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].cleanup.plainHeaderTargets).toHaveLength(1)
    expect(items[0].header).toBeNull()
  })

  it('can explicitly delete existing standard headers without inserting replacements', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        existingHeaderText: 'old standard header',
        existingHeaderArtifact: true,
        removeExistingHeader: true,
      },
    ]

    const items = buildHeaderFooterItems(
      files,
      {
        ...baseRules,
        headerMode: 'none',
        footerEnabled: false,
        cleanupHeaderEnabled: false,
        cleanupFooterEnabled: false,
      },
      '/out',
    )

    expect(items[0].cleanup.headerEnabled).toBe(true)
    expect(items[0].cleanup.forceDeleteHeader).toBe(true)
    expect(items[0].header).toBeNull()
  })

  it('can delete existing standard headers before inserting replacements', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        header: 'new header',
        existingHeaderText: 'old standard header',
        existingHeaderArtifact: true,
        removeExistingHeader: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].cleanup.forceDeleteHeader).toBe(true)
    expect(items[0].header.text).toBe('new header')
  })

  it('keeps overlay task for standard artifact header and footer edits', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        header: 'new header',
        footer: 'new footer',
        existingHeaderText: 'old standard header',
        existingFooterText: 'old standard footer',
        existingHeaderArtifact: true,
        existingFooterArtifact: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(canWriteHeader(files[0])).toBe(true)
    expect(canWriteFooter(files[0])).toBe(true)
    expect(items[0].cleanup.headerEnabled).toBe(false)
    expect(items[0].cleanup.footerEnabled).toBe(false)
    expect(items[0].header.text).toBe('new header')
    expect(items[0].footer.text).toBe('new footer')
  })

  it('edits standard existing header/footer separately from newly inserted text', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 2,
        header: 'new header',
        footer: 'new footer',
        existingHeaderText: 'edited old header',
        existingFooterText: 'edited old footer',
        existingHeaderTargetText: 'old standard header',
        existingFooterTargetText: 'old standard footer',
        existingHeaderArtifact: true,
        existingFooterArtifact: true,
        existingHeaderEdited: true,
        existingFooterEdited: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].cleanup.headerEnabled).toBe(true)
    expect(items[0].cleanup.footerEnabled).toBe(true)
    expect(items[0].cleanup.headerReplacement.text).toBe('edited old header')
    expect(items[0].cleanup.footerReplacement.text).toBe('edited old footer')
    expect(items[0].header.text).toBe('new header')
    expect(items[0].footer.text).toBe('new footer')
  })

  it('expands repeating detected header/footer targets to the full file', () => {
    expect(
      candidateTargetRange(
        {
          pageRange: { start: 1, end: 20 },
          repeating: true,
        },
        159,
      ),
    ).toEqual({ start: 1, end: 159 })

    expect(
      candidateTargetRange(
        {
          pageRange: { start: 3, end: 5 },
          repeating: false,
        },
        159,
      ),
    ).toEqual({ start: 3, end: 5 })
  })

  it('can number footers per file instead of continuously', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, header: '证据1 合同' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, header: '证据2 付款' },
    ]

    const items = buildHeaderFooterItems(
      files,
      {
        ...baseRules,
        footerContinuous: false,
      },
      '/out',
    )

    expect(items[0].pageStart).toBe(1)
    expect(items[0].totalPages).toBe(2)
    expect(items[1].pageStart).toBe(1)
    expect(items[1].totalPages).toBe(4)
  })

  it('builds a business-level evidence PDF rules payload', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, header: '证据1 合同' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, header: '证据2 付款' },
    ]

    const payload = buildEvidencePdfRulePayload(files, baseRules, '/out')

    expect(payload.items).toHaveLength(2)
    expect(payload.session.totalPages).toBe(6)
    expect(payload.session.items[0].evidenceLabel).toBe('证据1 合同')
    expect(payload.session.items[1].pageStart).toBe(3)
    expect(payload.session.headerRule.mode).toBe('per_file')
    expect(payload.session.outputRule.outputDir).toBe('/out')
    expect(payload.session.outputRule.outputMode).toBe('files_only')
    expect(payload.merge.enabled).toBe(false)
    expect(payload.session.annotationRule.removeAnnotations).toBe(false)
  })

  it('supports merge-only output mode in the business payload', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, header: '证据1 合同' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, header: '证据2 付款' },
    ]

    const payload = buildEvidencePdfRulePayload(
      files,
      {
        ...baseRules,
        outputMode: 'merge_only',
        mergeFileName: '提交证据.pdf',
      },
      '/out',
    )

    expect(payload.merge.enabled).toBe(true)
    expect(payload.merge.outputMode).toBe('merge_only')
    expect(payload.merge.outputPath).toBe('/out/提交证据.pdf')
    expect(payload.session.outputRule.mergeAfterProcessing).toBe(true)
  })

  it('builds merge output path for processed evidence packages', () => {
    const files = [{ ...createEvidenceFile('/case/合同.pdf'), pages: 2 }]

    expect(buildMergeOutputPath(files, '/out', '证据包')).toBe('/out/证据包.pdf')
    expect(buildMergeOutputPath(files, '', 'final.pdf')).toBe('/case/_docsy_pdf_processed/final.pdf')
    expect(buildOutputDir(files, '')).toBe('/case/_docsy_pdf_processed')
  })

  it('keeps common header naming modes deterministic', () => {
    const file = createEvidenceFile('/case/鉴定意见.pdf')

    expect(buildHeaderText(file, 0, { ...baseRules, headerMode: 'filename' })).toBe('鉴定意见')
    expect(buildHeaderText(file, 0, { ...baseRules, headerMode: 'custom', headerText: '' })).toBe('')
    expect(buildHeaderText(file, 0, { ...baseRules, headerMode: 'custom', headerText: '固定说明' })).toBe('固定说明')
    expect(buildHeaderText(file, 2, { ...baseRules, headerMode: 'custom', headerText: '证据[##]-[文件名]' })).toBe(
      '证据03-鉴定意见',
    )
    expect(buildHeaderText(file, 10, { ...baseRules, headerMode: 'custom', headerText: '证据[中文序号]' })).toBe(
      '证据十一',
    )
    expect(buildHeaderText(file, 2, { ...baseRules, headerMode: 'filename', headerPrefix: '证据[##]-' })).toBe(
      '证据03-鉴定意见',
    )
    expect(buildHeaderText(file, 2, { ...baseRules, headerMode: 'prefix_seq', headerText: '原告' })).toBe('原告证据3')
  })

  it('expands continuous page number placeholders', () => {
    expect(expandPlaceholders('第{page}页，共{total}页', 7, 31)).toBe('第7页，共31页')
    expect(expandPlaceholders('{range}', 7, 31)).toBe('7/31')
  })

  it('formats Chinese numbers used by evidence headers', () => {
    expect(toChineseNumber(1)).toBe('一')
    expect(toChineseNumber(10)).toBe('十')
    expect(toChineseNumber(11)).toBe('十一')
    expect(toChineseNumber(23)).toBe('二十三')
  })
})
