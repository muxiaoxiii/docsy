import { expandSplitNameTokens } from './splitFileName.js'

const CN_DIGITS = ['', '一', '二', '三', '四', '五', '六', '七', '八', '九']

export function createEvidenceFile(path) {
  const name = fileName(path)
  return {
    path,
    name,
    header: stripPdf(name),
    footer: null,
    headerEdited: false,
    footerEdited: false,
    pages: 0,
    pageStart: 1,
    pageEnd: 0,
    outputPath: '',
    detectionSummary: '',
    detectionCandidates: [],
    existingHeaderText: '',
    existingFooterText: '',
    existingHeaderArtifact: false,
    existingFooterArtifact: false,
    statusText: '等待',
    statusType: 'info',
  }
}

export function assignPageRanges(files) {
  let start = 1
  return files.map((file) => {
    file.pageStart = start
    file.pageEnd = file.pages ? start + file.pages - 1 : start - 1
    start += file.pages || 0
    return file
  })
}

export function totalPages(files) {
  return files.reduce((sum, file) => sum + (file.pages || 0), 0)
}

export function pageRangeText(file) {
  if (!file.pages) return '-'
  const end = file.pageEnd || file.pageStart + file.pages - 1
  return `${file.pageStart}-${end}`
}

export function buildHeaderText(file, index, rules) {
  if (rules.headerMode === 'none') return ''
  let base = ''
  if (rules.headerMode === 'per_file') base = file.header ?? stripPdf(file.name)
  else if (rules.headerMode === 'custom') base = rules.headerText || stripPdf(file.name)
  else if (rules.headerMode === 'seq') base = `证据${index + 1}`
  else if (rules.headerMode === 'seq_cn') base = `证据${toChineseNumber(index + 1)}`
  else if (rules.headerMode === 'prefix_seq') base = `${rules.headerText || ''}证据${index + 1}`
  else base = stripPdf(file.name)
  return decorateHeaderText(base, file, index, rules)
}

export function canWriteHeader(file) {
  return !(file?.existingHeaderText && !file?.existingHeaderArtifact)
}

export function canWriteFooter(file) {
  return !(file?.existingFooterText && !file?.existingFooterArtifact)
}

function decorateHeaderText(base, file, index, rules) {
  const name = stripPdf(file?.name || '')
  const contextText = String(base || '').replaceAll('[name]', name)
  const prefix = expandSplitNameTokens(rules.headerPrefix || '', index, rules.headerDateValue || '')
  const suffix = expandSplitNameTokens(rules.headerSuffix || '', index, rules.headerDateValue || '')
  const body = expandSplitNameTokens(contextText, index, rules.headerDateValue || '')
  return `${prefix}${body}${suffix}`.trim()
}

export function expandPlaceholders(template, page, total) {
  return String(template || '')
    .replaceAll('{page}', String(page))
    .replaceAll('{total}', String(total))
    .replaceAll('{range}', `${page}/${total}`)
}

export function buildHeaderFooterItems(files, rules, outputDir = '') {
  const rangedFiles = assignPageRanges(files)
  const total = totalPages(rangedFiles)
  return rangedFiles.map((file, index) => {
    const header = buildHeaderText(file, index, rules)
    const outputPath = buildOverlayOutputPath(file.path, outputDir)
    const continuousFooter = rules.footerContinuous !== false
    const pageStart = continuousFooter ? file.pageStart : 1
    const jobTotalPages = continuousFooter ? total : file.pages || 1
    const footerText = file.footer ?? rules.footerText
    file.outputPath = outputPath
    return {
      inputPath: file.path,
      outputPath,
      pageStart,
      totalPages: jobTotalPages,
      normalizeA4: rules.normalizeA4,
      a4Orientation: rules.a4Orientation,
      rasterDpi: rules.rasterDpi,
      cleanup: {
        headerEnabled: rules.cleanupHeaderEnabled,
        footerEnabled: rules.cleanupFooterEnabled,
        headerHeightMm: rules.cleanupHeaderHeightMm,
        footerHeightMm: rules.cleanupFooterHeightMm,
      },
      header: canWriteHeader(file) && header ? {
        text: header,
        fontSize: rules.headerFontSize,
        marginMm: rules.headerMarginMm,
        align: rules.headerAlign,
        offsetXMm: rules.headerOffsetXMm || 0,
        color: rules.headerColor || '#000000',
      } : null,
      footer: canWriteFooter(file) && rules.footerEnabled && footerText ? {
        text: footerText,
        fontSize: rules.footerFontSize,
        marginMm: rules.footerMarginMm,
        align: rules.footerAlign,
        offsetXMm: rules.footerOffsetXMm || 0,
        color: rules.footerColor || '#000000',
      } : null,
    }
  })
}

export function buildEvidencePdfRulePayload(files, rules, outputDir = '') {
  const rangedFiles = assignPageRanges(files)
  const items = buildHeaderFooterItems(rangedFiles, rules, outputDir)
  const total = totalPages(rangedFiles)
  const mergeOutputPath = buildMergeOutputPath(rangedFiles, outputDir, rules.mergeFileName)
  const outputMode = rules.outputMode || (rules.mergeAfterProcessing ? 'files_and_merge' : 'files_only')
  return {
    session: {
      items: rangedFiles.map((file, index) => ({
        id: file.id || file.path,
        sourcePath: file.path,
        displayName: file.name,
        evidenceLabel: buildHeaderText(file, index, rules),
        order: index + 1,
        pageCount: file.pages || 0,
        pageStart: file.pageStart,
        pageEnd: file.pageEnd,
        outputPath: file.outputPath || items[index]?.outputPath || '',
        status: file.statusText || 'ready',
      })),
      totalPages: total,
      headerRule: {
        mode: rules.headerMode,
        text: rules.headerText,
        prefix: rules.headerPrefix || '',
        suffix: rules.headerSuffix || '',
        align: rules.headerAlign,
        fontSize: rules.headerFontSize,
        marginMm: rules.headerMarginMm,
        offsetXmm: rules.headerOffsetXMm || 0,
        color: rules.headerColor || '#000000',
      },
      footerRule: {
        enabled: rules.footerEnabled,
        text: rules.footerText,
        continuous: rules.footerContinuous !== false,
        align: rules.footerAlign,
        fontSize: rules.footerFontSize,
        marginMm: rules.footerMarginMm,
        offsetXmm: rules.footerOffsetXMm || 0,
        color: rules.footerColor || '#000000',
      },
      cleanupRule: {
        headerEnabled: rules.cleanupHeaderEnabled,
        footerEnabled: rules.cleanupFooterEnabled,
        headerHeightMm: rules.cleanupHeaderHeightMm,
        footerHeightMm: rules.cleanupFooterHeightMm,
      },
      pageFormatRule: {
        normalizeA4: rules.normalizeA4,
        a4Orientation: rules.a4Orientation,
        rasterDpi: rules.rasterDpi,
      },
      annotationRule: {
        removeAnnotations: Boolean(rules.removeAnnotations),
        kinds: rules.annotationKinds || [],
      },
      outputRule: {
        outputDir,
        outputMode,
        mergeAfterProcessing: outputMode !== 'files_only',
        mergeOutputPath,
      },
    },
    items,
    merge: {
      enabled: outputMode !== 'files_only',
      outputPath: mergeOutputPath,
      outputMode,
    },
  }
}

export function buildOverlayOutputPath(inputPath, outputDir = '') {
  const name = fileName(inputPath)
  const stem = stripPdf(name)
  const dir = outputDir || `${parentDir(inputPath)}/_docsy_pdf_processed`
  return `${dir}/${stem}_processed.pdf`
}

export function buildMergeOutputPath(files, outputDir = '', fileName = '') {
  const first = files[0]
  const dir = buildOutputDir(files, outputDir)
  const name = ensurePdfFileName(fileName || 'merged_evidence.pdf')
  return `${dir}/${name}`
}

export function buildOutputDir(files, outputDir = '') {
  if (outputDir) return outputDir
  const first = files[0]
  return `${parentDir(first?.path || '.')}/_docsy_pdf_processed`
}

function ensurePdfFileName(name) {
  const value = String(name || '').trim() || 'merged_evidence.pdf'
  return /\.pdf$/i.test(value) ? value : `${value}.pdf`
}

export function fileName(path) {
  return String(path || '').split(/[\\/]/).pop() || path
}

export function parentDir(path) {
  const value = String(path || '')
  const idx = Math.max(value.lastIndexOf('/'), value.lastIndexOf('\\'))
  return idx >= 0 ? value.slice(0, idx) : '.'
}

export function stripPdf(name) {
  return String(name || '').replace(/\.pdf$/i, '')
}

export function toChineseNumber(value) {
  const num = Number(value)
  if (!Number.isInteger(num) || num <= 0) return String(value)
  if (num < 10) return CN_DIGITS[num]
  if (num === 10) return '十'
  if (num < 20) return `十${CN_DIGITS[num % 10]}`
  if (num < 100) {
    const tens = Math.floor(num / 10)
    const ones = num % 10
    return `${CN_DIGITS[tens]}十${CN_DIGITS[ones]}`
  }
  return String(value)
}
