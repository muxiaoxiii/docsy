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
    existingPageNumberText: '',
    existingHeaderTargetText: '',
    existingFooterTargetText: '',
    existingPageNumberTargetText: '',
    existingHeaderNormalizedText: '',
    existingFooterNormalizedText: '',
    existingPageNumberNormalizedText: '',
    existingHeaderBBox: null,
    existingFooterBBox: null,
    existingPageNumberBBox: null,
    existingHeaderPageStart: 1,
    existingHeaderPageEnd: 0,
    existingFooterPageStart: 1,
    existingFooterPageEnd: 0,
    existingPageNumberPageStart: 1,
    existingPageNumberPageEnd: 0,
    existingHeaderArtifact: false,
    existingFooterArtifact: false,
    existingHeaderEdited: false,
    existingFooterEdited: false,
    existingPageNumberEdited: false,
    convertPlainHeader: false,
    convertPlainFooter: false,
    convertPlainPageNumber: false,
    removeExistingHeader: false,
    removeExistingFooter: false,
    removeExistingPageNumber: false,
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
  if (file?.headerEdited) {
    return decorateHeaderText(file.header ?? '', file, index, rules)
  }
  if (rules.headerMode === 'none') return ''
  let base = ''
  if (rules.headerMode === 'per_file') base = file.header ?? stripPdf(file.name)
  else if (rules.headerMode === 'custom') base = rules.headerText || ''
  else if (rules.headerMode === 'template') base = rules.headerText || ''
  else if (rules.headerMode === 'seq') base = `证据${index + 1}`
  else if (rules.headerMode === 'seq_cn') base = `证据${toChineseNumber(index + 1)}`
  else if (rules.headerMode === 'prefix_seq') base = `${rules.headerText || ''}证据${index + 1}`
  else base = stripPdf(file.name)
  return decorateHeaderText(base, file, index, rules)
}

export function canWriteHeader(file) {
  return true
}

export function canWriteFooter(file) {
  return true
}

export function candidateTargetRange(candidate, pages = 0) {
  if (!candidate) return { start: 1, end: 0 }
  const start = candidate.pageRange?.start || 1
  const end = candidate.repeating && pages
    ? pages
    : candidate.pageRange?.end || pages || start
  return { start, end: Math.max(start, end) }
}

const NATURAL_COLLATOR = new Intl.Collator('zh-Hans-CN', {
  numeric: true,
  sensitivity: 'base',
})

export function naturalCompare(left, right) {
  const leftEmpty = left === null || left === undefined || left === ''
  const rightEmpty = right === null || right === undefined || right === ''
  if (leftEmpty && rightEmpty) return 0
  if (leftEmpty) return 1
  if (rightEmpty) return -1
  if (typeof left === 'number' && typeof right === 'number') {
    return left - right
  }
  return NATURAL_COLLATOR.compare(String(left), String(right))
}

export function sortByNatural(items, valueGetter, order = 'ascending') {
  const direction = order === 'descending' ? -1 : 1
  return [...items]
    .map((item, index) => ({ item, index, value: valueGetter(item, index) }))
    .sort((left, right) => {
      const result = naturalCompare(left.value, right.value)
      return result === 0 ? left.index - right.index : result * direction
    })
    .map(({ item }) => item)
}

function decorateHeaderText(base, file, index, rules) {
  const name = stripPdf(file?.name || '')
  const contextText = String(base || '')
    .replaceAll('[name]', name)
    .replaceAll('[文件名]', name)
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
    const existingHeaderReplacement = standardArtifactReplacementConfig(file, 'header', rules)
    const existingFooterReplacement = standardArtifactReplacementConfig(file, 'footer', rules)
    const extraOverlays = convertedExistingOverlays(file, rules)
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
        headerEnabled: Boolean(file.removeExistingHeader || existingHeaderReplacement),
        footerEnabled: Boolean(file.removeExistingFooter || file.removeExistingPageNumber || existingFooterReplacement),
        forceDeleteHeader: Boolean(file.removeExistingHeader),
        forceDeleteFooter: Boolean(file.removeExistingFooter),
        headerHeightMm: rules.cleanupHeaderHeightMm,
        footerHeightMm: rules.cleanupFooterHeightMm,
        plainHeaderTargets: buildPlainTextTargets(file, 'header'),
        plainFooterTargets: buildPlainTextTargets(file, 'footer'),
        headerReplacement: existingHeaderReplacement,
        footerReplacement: existingFooterReplacement,
      },
      header: header ? overlayConfigForFile(file, 'header', header, rules) : null,
      footer: rules.footerEnabled && footerText ? overlayConfigForFile(file, 'footer', footerText, rules) : null,
      extraOverlays,
    }
  })
}

function buildPlainTextTargets(file, region) {
  if (region === 'header') return [plainTextTarget(file, 'header')].filter(Boolean)
  return [
    plainTextTarget(file, 'footer'),
    plainTextTarget(file, 'pageNumber'),
  ].filter(Boolean)
}

function plainTextTarget(file, region) {
  const enabled = region === 'header'
    ? file.convertPlainHeader || file.removeExistingHeader
    : region === 'footer'
      ? file.convertPlainFooter || file.removeExistingFooter
      : file.convertPlainPageNumber || file.removeExistingPageNumber
  if (!enabled) return null
  const text = existingTargetText(file, region)
  if (!text) return null
  const normalizedText = existingNormalizedText(file, region)
  const pageStart = existingPageStart(file, region)
  const pageEnd = existingPageEnd(file, region)
  return {
    text,
    normalizedText: normalizedText || text,
    pageStart: pageStart || 1,
    pageEnd: pageEnd || file.pages || 1,
    bbox: existingBBox(file, region),
  }
}

function overlayConfigForFile(file, region, text, rules) {
  const isHeader = region === 'header'
  const bbox = existingBBox(file, region)
  const useDetectedPlacement = region === 'header'
    ? file.convertPlainHeader
    : region === 'footer'
      ? file.convertPlainFooter
      : file.convertPlainPageNumber
  const base = isHeader
    ? {
        region: 'header',
        text,
        fontFamily: rules.headerFontFamily || 'auto',
        fontSize: rules.headerFontSize,
        marginMm: rules.headerMarginMm,
        align: rules.headerAlign,
        offsetXMm: rules.headerOffsetXMm || 0,
        color: rules.headerColor || '#000000',
      }
    : {
        region: 'footer',
        text,
        fontFamily: rules.footerFontFamily || 'auto',
        fontSize: rules.footerFontSize,
        marginMm: rules.footerMarginMm,
        align: rules.footerAlign,
        offsetXMm: rules.footerOffsetXMm || 0,
        color: rules.footerColor || '#000000',
      }
  if (!useDetectedPlacement || !bbox || !bbox.width || !bbox.height) return base
  const centerX = (Number(bbox.x0) + Number(bbox.x1)) / 2
  const pageWidth = Number(bbox.width)
  const pageHeight = Number(bbox.height)
  const align = centerX < pageWidth * 0.36 ? 'left' : centerX > pageWidth * 0.64 ? 'right' : 'center'
  const anchorX = align === 'left' ? Number(bbox.x0) : align === 'right' ? Number(bbox.x1) : centerX
  const baseX = align === 'left' ? 0 : align === 'right' ? pageWidth : pageWidth / 2
  const yTop = Number(bbox.y0)
  const yBottom = Number(bbox.y1)
  const fontSize = Math.max(6, Math.min(24, yBottom - yTop || base.fontSize))
  return {
    ...base,
    region: isHeader ? 'header' : 'footer',
    fontSize,
    align,
    offsetXMm: ptToMm(anchorX - baseX),
    marginMm: isHeader ? ptToMm(yBottom) : ptToMm(pageHeight - yBottom),
  }
}

function convertedExistingOverlays(file, rules) {
  const overlays = []
  if (file.convertPlainHeader && !file.removeExistingHeader) {
    const text = String(file.existingHeaderText || '').trim()
    if (text) overlays.push(overlayConfigForFile(file, 'header', text, rules))
  }
  if (file.convertPlainFooter && !file.removeExistingFooter) {
    const text = String(file.existingFooterText || '').trim()
    if (text) overlays.push(overlayConfigForFile(file, 'footer', text, rules))
  }
  if (file.convertPlainPageNumber && !file.removeExistingPageNumber) {
    const text = String(file.existingPageNumberText || '').trim()
    if (text) overlays.push(overlayConfigForFile(file, 'pageNumber', text, rules))
  }
  return overlays
}

function existingTargetText(file, region) {
  if (region === 'header') return file.existingHeaderTargetText || file.existingHeaderText
  if (region === 'footer') return file.existingFooterTargetText || file.existingFooterText
  return file.existingPageNumberTargetText || file.existingPageNumberText
}

function existingNormalizedText(file, region) {
  if (region === 'header') return file.existingHeaderNormalizedText
  if (region === 'footer') return file.existingFooterNormalizedText
  return file.existingPageNumberNormalizedText
}

function existingBBox(file, region) {
  if (region === 'header') return file.existingHeaderBBox
  if (region === 'footer') return file.existingFooterBBox
  return file.existingPageNumberBBox
}

function existingPageStart(file, region) {
  if (region === 'header') return file.existingHeaderPageStart
  if (region === 'footer') return file.existingFooterPageStart
  return file.existingPageNumberPageStart
}

function existingPageEnd(file, region) {
  if (region === 'header') return file.existingHeaderPageEnd
  if (region === 'footer') return file.existingFooterPageEnd
  return file.existingPageNumberPageEnd
}

function ptToMm(value) {
  return Number(value || 0) * 25.4 / 72
}

function standardArtifactReplacementConfig(file, region, rules) {
  const isHeader = region === 'header'
  const artifact = isHeader ? file.existingHeaderArtifact : file.existingFooterArtifact
  const edited = isHeader ? file.existingHeaderEdited : file.existingFooterEdited
  const removed = isHeader ? file.removeExistingHeader : file.removeExistingFooter
  const text = isHeader ? file.existingHeaderText : file.existingFooterText
  if (!artifact || !edited || removed || !String(text || '').trim()) return null
  return isHeader
    ? {
        text,
        fontSize: rules.headerFontSize,
        fontFamily: rules.headerFontFamily || 'auto',
        marginMm: rules.headerMarginMm,
        align: rules.headerAlign,
        offsetXMm: rules.headerOffsetXMm || 0,
        color: rules.headerColor || '#000000',
      }
    : {
        text,
        fontSize: rules.footerFontSize,
        fontFamily: rules.footerFontFamily || 'auto',
        marginMm: rules.footerMarginMm,
        align: rules.footerAlign,
        offsetXMm: rules.footerOffsetXMm || 0,
        color: rules.footerColor || '#000000',
      }
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
        fontFamily: rules.headerFontFamily || 'auto',
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
        fontFamily: rules.footerFontFamily || 'auto',
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
