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
  createEvidenceFile,
  expandPlaceholders,
  pageRangeText,
  toChineseNumber,
  totalPages,
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

describe('Evidence PDF session helpers', () => {
  it('assigns continuous page ranges across separate PDFs', () => {
    const files = [
      { ...createEvidenceFile('/tmp/证据1.pdf'), pages: 3 },
      { ...createEvidenceFile('/tmp/证据2.pdf'), pages: 2 },
    ]

    assignPageRanges(files)

    expect(files[0].pageStart).toBe(1)
    expect(files[0].pageEnd).toBe(3)
    expect(files[1].pageStart).toBe(4)
    expect(files[1].pageEnd).toBe(6 - 1)
    expect(pageRangeText(files[1])).toBe('4-5')
    expect(totalPages(files)).toBe(5)
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

  it('supports per-file footer overrides including blank values', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, footer: '第{page}页' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, footer: '' },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(items[0].footer.text).toBe('第{page}页')
    expect(items[1].footer).toBeNull()
  })

  it('does not overlay detected non-standard header or footer text', () => {
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

    expect(canWriteHeader(files[0])).toBe(false)
    expect(canWriteFooter(files[0])).toBe(false)
    expect(items[0].header).toBeNull()
    expect(items[0].footer).toBeNull()
  })

  it('converts confirmed plain text headers and footers into cleanup targets plus overlay', () => {
    const files = [
      {
        ...createEvidenceFile('/case/合同.pdf'),
        pages: 3,
        header: 'new header',
        footer: 'new footer',
        existingHeaderText: 'old plain header',
        existingHeaderNormalizedText: 'oldplainheader',
        existingHeaderPageStart: 1,
        existingHeaderPageEnd: 3,
        existingFooterText: '1/3',
        existingFooterNormalizedText: '{page}/{total}',
        existingFooterPageStart: 1,
        existingFooterPageEnd: 3,
        convertPlainHeader: true,
        convertPlainFooter: true,
      },
    ]

    const items = buildHeaderFooterItems(files, baseRules, '/out')

    expect(canWriteHeader(files[0])).toBe(true)
    expect(canWriteFooter(files[0])).toBe(true)
    expect(items[0].header.text).toBe('new header')
    expect(items[0].footer.text).toBe('new footer')
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
    expect(items[0].header.text).toBe('new header')
    expect(items[0].footer.text).toBe('new footer')
  })

  it('can number footers per file instead of continuously', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2, header: '证据1 合同' },
      { ...createEvidenceFile('/case/付款.pdf'), pages: 4, header: '证据2 付款' },
    ]

    const items = buildHeaderFooterItems(files, {
      ...baseRules,
      footerContinuous: false,
    }, '/out')

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

    const payload = buildEvidencePdfRulePayload(files, {
      ...baseRules,
      outputMode: 'merge_only',
      mergeFileName: '提交证据.pdf',
    }, '/out')

    expect(payload.merge.enabled).toBe(true)
    expect(payload.merge.outputMode).toBe('merge_only')
    expect(payload.merge.outputPath).toBe('/out/提交证据.pdf')
    expect(payload.session.outputRule.mergeAfterProcessing).toBe(true)
  })

  it('builds merge output path for processed evidence packages', () => {
    const files = [
      { ...createEvidenceFile('/case/合同.pdf'), pages: 2 },
    ]

    expect(buildMergeOutputPath(files, '/out', '证据包')).toBe('/out/证据包.pdf')
    expect(buildMergeOutputPath(files, '', 'final.pdf')).toBe('/case/_docsy_pdf_processed/final.pdf')
    expect(buildOutputDir(files, '')).toBe('/case/_docsy_pdf_processed')
  })

  it('keeps common header naming modes deterministic', () => {
    const file = createEvidenceFile('/case/鉴定意见.pdf')

    expect(buildHeaderText(file, 0, { ...baseRules, headerMode: 'filename' })).toBe('鉴定意见')
    expect(buildHeaderText(file, 1, { ...baseRules, headerMode: 'seq' })).toBe('证据2')
    expect(buildHeaderText(file, 10, { ...baseRules, headerMode: 'seq_cn' })).toBe('证据十一')
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
