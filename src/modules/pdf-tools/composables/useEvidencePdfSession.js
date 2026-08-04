import { expandSplitNameTokens } from './splitFileName.js'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { toChineseNumber } from '../../../core/numberFormat.js'
import { ptToMm } from '../../../core/unitConversion.js'
import { pageNumberOverlaysForFile } from './pdfPageNumberRules.js'

export { fileName, parentDir, stripPdf, toChineseNumber }

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
    existingFooterCandidateKey: '',
    existingPageNumberCandidateKey: '',
    footerCandidateChoices: [],
    ignoredFooterCandidateKeys: [],
    existingHeaderBBox: null,
    existingFooterBBox: null,
    existingPageNumberBBox: null,
    existingHeaderFontSize: null,
    existingFooterFontSize: null,
    existingPageNumberFontSize: null,
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
    const pages = Number(file.pages || 0)
    const pageStart = start
    const pageEnd = pages ? start + pages - 1 : start - 1
    start += pages
    return {
      ...file,
      pageStart,
      pageEnd,
    }
  })
}

export function updatePageRanges(files) {
  let start = 1
  return files.map((file) => {
    file.pageStart = start
    file.pageEnd = file.pages ? start + Number(file.pages || 0) - 1 : start - 1
    start += Number(file.pages || 0)
    return file
  })
}

export function totalPages(files) {
  return files.reduce((sum, file) => sum + (file.pages || 0), 0)
}

export function pageRangeText(file, sequence) {
  if (!file.pages) return '-'
  if (sequence === 'per-file') return `1-${file.pages}`
  const end = file.pageEnd || file.pageStart + file.pages - 1
  return `${file.pageStart}-${end}`
}

export function buildHeaderText(file, index, rules) {
  if (file?.headerEdited) {
    return decorateHeaderText(file.header ?? '', file, index, rules)
  }
  if (rules.headerMode === 'none') return ''
  const base = headerBaseText(file, index, rules)
  return decorateHeaderText(base, file, index, rules)
}

export function buildHeaderTextForGroup(file, index, group, rules) {
  if (group.mode === 'none') return ''
  const base = headerBaseTextForGroup(file, index, group, rules)
  return decorateHeaderTextForGroup(base, file, index, group, rules)
}

function headerBaseTextForGroup(file, index, group, rules) {
  if (group.mode === 'per_file') return file.header ?? stripPdf(file.name)
  if (group.mode === 'custom' || group.mode === 'template') return group.text || ''
  if (group.mode === 'seq') return `证据${index + 1}`
  if (group.mode === 'seq_cn') return `证据${toChineseNumber(index + 1)}`
  if (group.mode === 'prefix_seq') return `${group.text || ''}证据${index + 1}`
  return stripPdf(file.name)
}

function decorateHeaderTextForGroup(base, file, index, group, rules) {
  const name = stripPdf(file?.name || '')
  const contextText = String(base || '')
    .replaceAll('[name]', name)
    .replaceAll('[文件名]', name)
  const prefix = expandSplitNameTokens(group.prefix || '', index, rules.headerDateValue || '')
  const suffix = expandSplitNameTokens(group.suffix || '', index, rules.headerDateValue || '')
  const body = expandSplitNameTokens(contextText, index, rules.headerDateValue || '')
  return `${prefix}${body}${suffix}`.trim()
}

export function overlayConfigForGroup(file, region, text, group) {
  return {
    region: 'header',
    artifactKind: 'HeaderText',
    text,
    fontFamily: group.fontFamily || 'auto',
    fontSize: group.fontSize,
    marginMm: group.marginMm,
    align: group.align,
    offsetXMm: group.offsetXMm || 0,
    color: group.color || '#000000',
  }
}

function headerBaseText(file, index, rules) {
  if (rules.headerMode === 'per_file') return file.header ?? stripPdf(file.name)
  if (rules.headerMode === 'custom' || rules.headerMode === 'template') return rules.headerText || ''
  if (rules.headerMode === 'seq') return `证据${index + 1}`
  if (rules.headerMode === 'seq_cn') return `证据${toChineseNumber(index + 1)}`
  if (rules.headerMode === 'prefix_seq') return `${rules.headerText || ''}证据${index + 1}`
  return stripPdf(file.name)
}

export function canWriteHeader(file) {
  return Boolean(file?.path)
}

export function canWriteFooter(file) {
  return Boolean(file?.path)
}

export function candidateTargetRange(candidate, pages = 0) {
  if (!candidate) return { start: 1, end: 0 }
  const start = candidate.pageRange?.start || 1
  const end = candidate.repeating && pages ? pages : candidate.pageRange?.end || pages || start
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
    const legacyFooterMode = rules.footerInsertEnabled === undefined && rules.pageNumberEnabled === undefined
    const headerInsertEnabled = rules.headerInsertEnabled !== false
    const footerInsertEnabled = rules.footerInsertEnabled !== false
    const header = headerInsertEnabled ? buildHeaderText(file, index, rules) : ''
    const outputPath = buildOverlayOutputPath(file.path, outputDir)
    const pageNumberEnabled = rules.pageNumberEnabled ?? rules.footerEnabled
    const pageNumberSequence =
      rules.pageNumberSequence || (rules.footerContinuous === false ? 'per-file' : 'continuous')
    const continuousPageNumber = pageNumberSequence !== 'per-file'
    const pageStart = continuousPageNumber ? file.pageStart : 1
    const jobTotalPages = continuousPageNumber ? total : file.pages || 1
    const existingHeaderReplacement = standardArtifactReplacementConfig(file, 'header', rules)
    const existingFooterReplacement = standardArtifactReplacementConfig(file, 'footer', rules)
    const pageNumberOverlays = legacyFooterMode
      ? []
      : pageNumberOverlaysForFile(file, {
          enabled: pageNumberEnabled,
          totalPages: total,
          sequence: pageNumberSequence,
          template: rules.pageNumberTemplate || rules.footerText || '{page}/{total}',
          style: rules.pageNumberStyle || 'arabic',
          region: rules.pageNumberRegion || 'footer',
          align: rules.pageNumberAlign || rules.footerAlign,
          fontSize: rules.pageNumberFontSize || rules.footerFontSize,
          fontFamily: rules.pageNumberFontFamily || rules.footerFontFamily,
          marginMm: rules.pageNumberMarginMm || rules.footerMarginMm,
          offsetXMm: rules.pageNumberOffsetXMm ?? rules.footerOffsetXMm,
          color: rules.pageNumberColor || rules.footerColor,
          overrides: rules.pageNumberOverrides || [],
        })
    const extraOverlays = [...convertedExistingOverlays(file, rules), ...pageNumberOverlays]
    // Add header groups overlays (skip first group since it's already the main header)
    const headerGroups = rules.headerGroups || []
    if (headerInsertEnabled && headerGroups.length > 1) {
      for (let gi = 1; gi < headerGroups.length; gi++) {
        const g = headerGroups[gi]
        if (!g.enabled) continue
        const groupText = buildHeaderTextForGroup(file, index, g, rules)
        if (groupText) {
          extraOverlays.push(overlayConfigForGroup(file, 'header', groupText, g))
        }
      }
    }
    // Add footer text group overlays (skip first group since it's already the main footer text)
    const footerTextGroups = rules.footerTextGroups || []
    if (footerInsertEnabled && footerTextGroups.length > 1) {
      for (let gi = 1; gi < footerTextGroups.length; gi++) {
        const g = footerTextGroups[gi]
        if (!g.enabled) continue
        if (g.text) {
          extraOverlays.push(footerTextOverlayConfigForGroup(g.text, g))
        }
      }
    }
    // Add page number group overlays (skip first group since it's already the main page number)
    const pageNumberGroups = rules.pageNumberGroups || []
    if (pageNumberEnabled && pageNumberGroups.length > 1) {
      for (let gi = 1; gi < pageNumberGroups.length; gi++) {
        const g = pageNumberGroups[gi]
        if (!g.enabled) continue
        const pnSequence = g.sequence || pageNumberSequence
        const pnContinuous = pnSequence !== 'per-file'
        const pnStart = pnContinuous ? file.pageStart : 1
        const pnTotal = pnContinuous ? total : file.pages || 1
        const pnOverlays = pageNumberOverlaysForFile(file, {
          enabled: true,
          totalPages: pnTotal,
          sequence: pnSequence,
          template: g.template || '{page}/{total}',
          style: g.style || 'arabic',
          region: g.region || 'footer',
          align: g.align || 'center',
          fontSize: g.fontSize || 9,
          fontFamily: g.fontFamily || 'auto',
          marginMm: g.marginMm || 10,
          offsetXMm: g.offsetXMm || 0,
          color: g.color || '#000000',
          overrides: rules.pageNumberOverrides || [],
        })
        extraOverlays.push(...pnOverlays)
      }
    }
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
        headerEnabled: Boolean(hasArtifactDecision(file, ['header'], 'delete') || existingHeaderReplacement),
        footerEnabled: Boolean(
          hasArtifactDecision(file, ['footerText', 'pageNumber'], 'delete') || existingFooterReplacement,
        ),
        forceDeleteHeader: Boolean(hasArtifactDecision(file, ['header'], 'delete')),
        forceDeleteFooter: Boolean(hasArtifactDecision(file, ['footerText', 'pageNumber'], 'delete')),
        headerHeightMm: rules.cleanupHeaderHeightMm,
        footerHeightMm: rules.cleanupFooterHeightMm,
        plainHeaderTargets: buildPlainTextTargets(file, 'header'),
        plainFooterTargets: buildPlainTextTargets(file, 'footer'),
        headerReplacement: existingHeaderReplacement,
        footerReplacement: existingFooterReplacement,
      },
      header: header ? overlayConfigForFile(file, 'header', header, rules) : null,
      footer:
        legacyFooterMode && rules.footerEnabled && (file.footer ?? rules.footerText)
          ? (footerInsertEnabled ? overlayConfigForFile(file, 'footer', file.footer ?? rules.footerText, rules) : null)
          : footerInsertEnabled && rules.footerTextContent
            ? footerTextOverlayConfig(rules.footerTextContent, rules)
            : null,
      extraOverlays,
    }
  })
}

function footerTextOverlayConfig(text, rules) {
  return {
    text,
    region: 'footer',
    artifactKind: 'FooterText',
    fontSize: rules.footerTextFontSize || 9,
    fontFamily: rules.footerTextFontFamily || 'auto',
    marginMm: rules.footerTextMarginMm || 10,
    align: rules.footerTextAlign || 'left',
    offsetXMm: rules.footerTextOffsetXMm || 0,
    color: rules.footerTextColor || '#000000',
  }
}

function footerTextOverlayConfigForGroup(text, group) {
  return {
    text,
    region: 'footer',
    artifactKind: 'FooterText',
    fontSize: group.fontSize || 9,
    fontFamily: group.fontFamily || 'auto',
    marginMm: group.marginMm || 10,
    align: group.align || 'left',
    offsetXMm: group.offsetXMm || 0,
    color: group.color || '#000000',
  }
}

function buildPlainTextTargets(file, region) {
  if (Array.isArray(file.existingElements) && file.existingElements.length) {
    const kinds = region === 'header' ? ['header'] : ['footerText', 'pageNumber']
    return file.existingElements
      .filter(
        (element) =>
          kinds.includes(element.kind) &&
          element.source !== 'artifact' &&
          ['delete', 'edit'].includes(element.decision),
      )
      .map((element) => ({
        text: element.detectedText,
        normalizedText: element.normalizedText || element.detectedText,
        pageStart: element.pageStart || 1,
        pageEnd: element.pageEnd || file.pages || 1,
        bbox: element.bbox || null,
      }))
      .filter((target) => target.text)
  }
  if (region === 'header') return [plainTextTarget(file, 'header')].filter(Boolean)
  return [plainTextTarget(file, 'footer'), plainTextTarget(file, 'pageNumber')].filter(Boolean)
}

function plainTextTarget(file, region) {
  const enabled =
    region === 'header'
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
  const useDetectedPlacement =
    region === 'header'
      ? file.convertPlainHeader
      : region === 'footer'
        ? file.convertPlainFooter
        : file.convertPlainPageNumber
  const base = isHeader
    ? {
        region: 'header',
        artifactKind: 'HeaderText',
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
        artifactKind: region === 'pageNumber' ? 'PageNumber' : 'FooterText',
        text,
        fontFamily: rules.footerFontFamily || 'auto',
        fontSize: rules.footerFontSize,
        marginMm: rules.footerMarginMm,
        align: rules.footerAlign,
        offsetXMm: rules.footerOffsetXMm || 0,
        color: rules.footerColor || '#000000',
      }
  const scopedBase = useDetectedPlacement
    ? {
        ...base,
        pageStart: existingPageStart(file, region) || 1,
        pageEnd: existingPageEnd(file, region) || file.pages || 1,
      }
    : base
  if (!useDetectedPlacement || !bbox || !bbox.width || !bbox.height) return scopedBase
  const centerX = (Number(bbox.x0) + Number(bbox.x1)) / 2
  const pageWidth = Number(bbox.width)
  const pageHeight = Number(bbox.height)
  const align = centerX < pageWidth * 0.36 ? 'left' : centerX > pageWidth * 0.64 ? 'right' : 'center'
  const anchorX = align === 'left' ? Number(bbox.x0) : align === 'right' ? Number(bbox.x1) : centerX
  const baseX = align === 'left' ? 0 : align === 'right' ? pageWidth : pageWidth / 2
  const yBottom = Number(bbox.y1)
  const fontSize = existingFontSize(file, region) || inferDetectedFontSize(bbox, base.fontSize, text)
  return {
    ...scopedBase,
    region: isHeader ? 'header' : 'footer',
    fontSize,
    align,
    offsetXMm: ptToMm(anchorX - baseX),
    marginMm: isHeader ? ptToMm(yBottom) : ptToMm(pageHeight - yBottom),
  }
}

function inferDetectedFontSize(bbox, fallback, text = '') {
  const height = Math.abs(Number(bbox?.y1 || 0) - Number(bbox?.y0 || 0))
  if (!Number.isFinite(height) || height <= 0) return fallback
  const cjk = /[\u3400-\u9fff\uf900-\ufaff]/.test(String(text || ''))
  const ratio = cjk && height > 16 ? 0.58 : 0.72
  const inferred = height * ratio
  return Math.max(6, Math.min(18, Number(inferred.toFixed(2))))
}

function convertedExistingOverlays(file, rules) {
  if (Array.isArray(file.existingElements) && file.existingElements.length) {
    return file.existingElements
      .filter((element) => element.source !== 'artifact' && element.decision === 'edit')
      .map((element) => overlayConfigForDetectedElement(element, rules))
      .filter(Boolean)
  }
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

function hasArtifactDecision(file, kinds, decision) {
  if (!Array.isArray(file.existingElements) || !file.existingElements.length) {
    if (decision !== 'delete') return false
    if (kinds.includes('header')) return Boolean(file.existingHeaderArtifact && file.removeExistingHeader)
    return Boolean(
      (file.existingFooterArtifact && file.removeExistingFooter) ||
      (file.existingPageNumberArtifact && file.removeExistingPageNumber),
    )
  }
  return (file.existingElements || []).some(
    (element) => kinds.includes(element.kind) && element.source === 'artifact' && element.decision === decision,
  )
}

function overlayConfigForDetectedElement(element, rules) {
  const isHeader = element.kind === 'header'
  const bbox = element.bbox || null
  const pageWidth = Number(bbox?.width || 0)
  const pageHeight = Number(bbox?.height || 0)
  const centerX = bbox ? (Number(bbox.x0) + Number(bbox.x1)) / 2 : 0
  const align = !bbox
    ? isHeader
      ? rules.headerAlign
      : rules.footerAlign
    : centerX < pageWidth * 0.36
      ? 'left'
      : centerX > pageWidth * 0.64
        ? 'right'
        : 'center'
  const anchorX = !bbox ? 0 : align === 'left' ? Number(bbox.x0) : align === 'right' ? Number(bbox.x1) : centerX
  const baseX = align === 'left' ? 0 : align === 'right' ? pageWidth : pageWidth / 2
  const fallbackFontSize = isHeader ? rules.headerFontSize : rules.footerFontSize
  return {
    text: String(element.editedText || element.detectedText || ''),
    region: isHeader ? 'header' : 'footer',
    artifactKind: isHeader ? 'HeaderText' : element.kind === 'pageNumber' ? 'PageNumber' : 'FooterText',
    pageStart: element.pageStart || 1,
    pageEnd: element.pageEnd || 1,
    fontFamily: isHeader ? rules.headerFontFamily || 'auto' : rules.footerFontFamily || 'auto',
    fontSize: Number(element.fontSize || inferDetectedFontSize(bbox, fallbackFontSize, element.editedText)),
    align,
    offsetXMm: bbox ? ptToMm(anchorX - baseX) : 0,
    marginMm: bbox
      ? isHeader
        ? ptToMm(pageHeight - Number(bbox.y1 || 0))
        : ptToMm(Number(bbox.y0 || 0))
      : isHeader
        ? rules.headerMarginMm
        : rules.footerMarginMm,
    color: isHeader ? rules.headerColor || '#000000' : rules.footerColor || '#000000',
  }
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

function existingFontSize(file, region) {
  const value =
    region === 'header'
      ? file.existingHeaderFontSize
      : region === 'footer'
        ? file.existingFooterFontSize
        : file.existingPageNumberFontSize
  const size = Number(value || 0)
  return Number.isFinite(size) && size > 0 ? Math.max(6, Math.min(18, size)) : null
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
        artifactKind: 'HeaderText',
        fontSize: rules.headerFontSize,
        fontFamily: rules.headerFontFamily || 'auto',
        marginMm: rules.headerMarginMm,
        align: rules.headerAlign,
        offsetXMm: rules.headerOffsetXMm || 0,
        color: rules.headerColor || '#000000',
      }
    : {
        text,
        artifactKind: 'FooterText',
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
        groups: rules.headerGroups || [],
      },
      footerRule: {
        enabled: Boolean(rules.footerInsertEnabled),
        text: rules.footerTextContent || '',
        align: rules.footerTextAlign || 'left',
        fontSize: rules.footerTextFontSize || 9,
        fontFamily: rules.footerTextFontFamily || 'auto',
        marginMm: rules.footerTextMarginMm || 10,
        offsetXmm: rules.footerTextOffsetXMm || 0,
        color: rules.footerTextColor || '#000000',
        groups: rules.footerTextGroups || [],
      },
      pageNumberRule: {
        enabled: rules.pageNumberEnabled ?? rules.footerEnabled,
        template: rules.pageNumberTemplate || rules.footerText || '{page}/{total}',
        style: rules.pageNumberStyle || 'arabic',
        sequence: rules.pageNumberSequence || (rules.footerContinuous === false ? 'per-file' : 'continuous'),
        region: rules.pageNumberRegion || 'footer',
        align: rules.pageNumberAlign || rules.footerAlign,
        fontSize: rules.pageNumberFontSize || rules.footerFontSize,
        fontFamily: rules.pageNumberFontFamily || rules.footerFontFamily || 'auto',
        marginMm: rules.pageNumberMarginMm || rules.footerMarginMm,
        offsetXmm: rules.pageNumberOffsetXMm ?? rules.footerOffsetXMm ?? 0,
        color: rules.pageNumberColor || rules.footerColor || '#000000',
        overrides: rules.pageNumberOverrides || [],
        groups: rules.pageNumberGroups || [],
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
