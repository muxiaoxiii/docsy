import { ElMessage } from 'element-plus'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { candidateTargetRange } from './useEvidencePdfSession.js'

const DETECTION_SCAN_MAX_PAGES = 20
const PAGE_NUMBER_MIN_BOTTOM_RATIO = 0.9
const PAGE_NUMBER_MIN_CONFIDENCE = 0.75
const ROMAN_PAGE_SCORE_PENALTY = -0.25
const NON_PAGE_FOOTER_MIN_CONFIDENCE = 0.65

export function candidateKey(candidate) {
  if (!candidate) return ''
  if (candidate.candidateKey) return candidate.candidateKey
  const bbox = candidate.bbox || {}
  const range = candidate.pageRange || {}
  return [
    candidate.region || 'footer',
    candidate.normalizedText || candidate.text || '',
    range.start || bbox.page || 1,
    range.end || range.start || bbox.page || 1,
    Math.round(Number(bbox.x0 || 0)),
    Math.round(Number(bbox.y0 || 0)),
    Math.round(Number(bbox.x1 || 0)),
    Math.round(Number(bbox.y1 || 0)),
  ].join('|')
}

export function useEvidencePdfDetection({
  overlayRows,
  detectingAllHeaderFooter,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
}) {
  async function detectAllHeaderFooter(options = {}) {
    const silent = Boolean(options.silent)
    if (!overlayRows.value.length || detectingAllHeaderFooter.value) return
    detectingAllHeaderFooter.value = true
    let success = 0
    let failed = 0
    try {
      for (const file of overlayRows.value) {
        file.statusText = '检测中'
        file.statusType = 'warning'
        const result = await detectFileHeaderFooter(file)
        if (result.ok) {
          try {
            applyDetectionResultToFile(file, result.data || {})
            const status = fileExistingStatus(file)
            file.statusText = status.text
            file.statusType = status.type
            success += 1
          } catch {
            file.statusText = '检测失败'
            file.statusType = 'danger'
            failed += 1
          }
        } else {
          file.statusText = '检测失败'
          file.statusType = 'danger'
          failed += 1
        }
      }
      if (!silent) {
        failed
          ? ElMessage.warning(`已检测 ${success} 个，失败 ${failed} 个`)
          : ElMessage.success(`已检测 ${success} 个 PDF`)
      }
    } finally {
      detectingAllHeaderFooter.value = false
    }
  }

  async function detectFileHeaderFooter(file) {
    return tauriCallSafe('detect_pdf_header_footer', {
      args: {
        inputPath: file.path,
        maxPages: DETECTION_SCAN_MAX_PAGES,
        headerZoneMm: cleanupHeaderHeightMm.value,
        footerZoneMm: cleanupFooterHeightMm.value,
      },
    })
  }

  function applyDetectionResultToFile(file, data) {
    const header = data.headerCandidates?.[0]
    const footerCandidates = data.footerCandidates || []
    const pageNumber = bestReliablePageNumberCandidate(
      footerCandidates,
      file.pages || data.pages?.length || data.pagesAnalyzed || 1,
    )
    const footer = footerCandidates.find(isStrongNonPageFooterCandidate) || null
    const candidates = [...(data.headerCandidates || []).slice(0, 6), ...(data.footerCandidates || []).slice(0, 6)]
    const parts = []
    if (data.artifact?.hasHeader) parts.push(`发现结构化页眉 ${data.artifact.headerCount} 处`)
    if (data.artifact?.hasFooter) parts.push(`发现结构化页脚 ${data.artifact.footerCount} 处`)
    if (header) parts.push(`页眉候选：${header.text}`)
    if (footer) parts.push(`页脚候选：${footer.text}`)
    if (pageNumber) parts.push(`页码候选：${pageNumber.text}`)
    if (candidates.length) parts.push(`候选 ${candidates.length} 个`)
    file.existingHeaderText = header?.text || ''
    file.existingFooterText = footer?.text || footer?.normalizedText || ''
    file.existingPageNumberText = pageNumber?.text || pageNumber?.normalizedText || ''
    file.existingHeaderTargetText = header?.text || ''
    file.existingFooterTargetText = footer?.text || footer?.normalizedText || ''
    file.existingPageNumberTargetText = pageNumber?.text || pageNumber?.normalizedText || ''
    file.existingHeaderNormalizedText = header?.normalizedText || header?.text || ''
    file.existingFooterNormalizedText = footer?.normalizedText || footer?.text || ''
    file.existingPageNumberNormalizedText = pageNumber?.normalizedText || pageNumber?.text || ''
    file.existingHeaderBBox = header?.bbox || null
    file.existingFooterBBox = footer?.bbox || null
    file.existingPageNumberBBox = pageNumber?.bbox || null
    file.existingHeaderFontSize = header?.fontSize || null
    file.existingFooterFontSize = footer?.fontSize || null
    file.existingPageNumberFontSize = pageNumber?.fontSize || null
    file.footerCandidateChoices = footerCandidates.map((candidate) => ({
      ...candidate,
      candidateKey: candidateKey(candidate),
    }))
    file.existingFooterCandidateKey = footer ? candidateKey(footer) : ''
    file.existingPageNumberCandidateKey = pageNumber ? candidateKey(pageNumber) : ''
    file.ignoredFooterCandidateKeys = []
    const headerTargetRange = candidateTargetRange(header, file.pages)
    const footerTargetRange = candidateTargetRange(footer, file.pages)
    const pageNumberTargetRange = candidateTargetRange(pageNumber, file.pages)
    file.existingHeaderPageStart = headerTargetRange.start
    file.existingHeaderPageEnd = headerTargetRange.end
    file.existingFooterPageStart = footerTargetRange.start
    file.existingFooterPageEnd = footerTargetRange.end
    file.existingPageNumberPageStart = pageNumberTargetRange.start
    file.existingPageNumberPageEnd = pageNumberTargetRange.end
    file.existingHeaderArtifact = Boolean(data.artifact?.hasHeader)
    file.existingFooterArtifact = Boolean(data.artifact?.hasFooter)
    file.existingHeaderEdited = false
    file.existingFooterEdited = false
    file.existingPageNumberEdited = false
    if (!file.existingHeaderText || file.existingHeaderArtifact) file.convertPlainHeader = false
    if (!file.existingFooterText || file.existingFooterArtifact) file.convertPlainFooter = false
    if (!file.existingPageNumberText) file.convertPlainPageNumber = false
    if (!hasExistingHeader(file)) file.removeExistingHeader = false
    if (!hasExistingFooter(file)) file.removeExistingFooter = false
    if (!hasExistingPageNumber(file)) file.removeExistingPageNumber = false
    file.detectionSummary = parts.length ? parts.join('；') : '未发现稳定的文本型页眉页脚候选'
    file.detectionCandidates = candidates
  }

  function isPageNumberCandidate(candidate) {
    const normalized = String(candidate?.normalizedText || '')
    return Boolean(
      candidate?.labels?.includes?.('page-number') ||
      normalized.includes('{page}') ||
      normalized.includes('{total}') ||
      normalized.includes('{roman-page}'),
    )
  }

  function bestPageNumberCandidate(candidates = []) {
    return (
      candidates
        .filter(isPageNumberCandidate)
        .sort((left, right) => pageNumberCandidateScore(right) - pageNumberCandidateScore(left))[0] || null
    )
  }

  function bestReliablePageNumberCandidate(candidates = [], totalPages = 1) {
    return bestPageNumberCandidate(
      candidates.filter((candidate) => isReliablePageNumberCandidate(candidate, totalPages)),
    )
  }

  function isReliablePageNumberCandidate(candidate, totalPages = 1) {
    if (!isPageNumberCandidate(candidate)) return false
    const normalized = String(candidate?.normalizedText || '')
    const count = Number(candidate?.count || 0)
    const pageRange = candidate?.pageRange || {}
    const rangeLength = Math.max(0, Number(pageRange.end || 0) - Number(pageRange.start || 0) + 1)
    if (Number(totalPages || 1) <= 1) return true
    if (normalized.includes('{total}')) return true
    if (count >= 2 || rangeLength >= 2) return true
    const bbox = candidate?.bbox || {}
    const pageHeight = Number(bbox.height || 0)
    const yBottomRatio = pageHeight ? Number(bbox.y1 || 0) / pageHeight : 0
    const isArabic = normalized === '{page}' || normalized.includes('{page}')
    return (
      isArabic &&
      yBottomRatio >= PAGE_NUMBER_MIN_BOTTOM_RATIO &&
      Number(candidate?.confidence || 0) >= PAGE_NUMBER_MIN_CONFIDENCE
    )
  }

  function pageNumberCandidateScore(candidate) {
    const bbox = candidate?.bbox || {}
    const pageHeight = Number(bbox.height || 0)
    const yBottomRatio = pageHeight ? Number(bbox.y1 || 0) / pageHeight : 0
    const count = Number(candidate?.count || 0)
    const normalized = String(candidate?.normalizedText || '')
    const pageRange = candidate?.pageRange || {}
    const rangeLength = Math.max(0, Number(pageRange.end || 0) - Number(pageRange.start || 0) + 1)
    const totalFormatBonus = normalized.includes('{total}') ? 2 : 0
    const arabicBonus = normalized === '{page}' || normalized.includes('{page}/') ? 1 : 0
    const romanPenalty = normalized.includes('{roman-page}') ? ROMAN_PAGE_SCORE_PENALTY : 0
    return (
      yBottomRatio * 100 +
      Math.min(count, 50) * 5 +
      Math.min(rangeLength, 50) * 3 +
      Number(Boolean(candidate?.repeating)) * 8 +
      totalFormatBonus +
      arabicBonus +
      romanPenalty +
      Number(candidate?.confidence || 0)
    )
  }

  function isStrongNonPageFooterCandidate(candidate) {
    if (!candidate || isPageNumberCandidate(candidate)) return false
    if (candidate.repeating || Number(candidate.count || 0) >= 2) return true
    if (candidate.labels?.length) return true
    return Number(candidate.confidence || 0) >= NON_PAGE_FOOTER_MIN_CONFIDENCE
  }

  function footerCandidateMeta(candidate) {
    const range = candidate?.pageRange || {}
    const start = range.start || candidate?.bbox?.page || 1
    const end = range.end || start
    const count = candidate?.count ? `，${candidate.count} 次` : ''
    const label = isPageNumberCandidate(candidate) ? '页码型' : '文本型'
    return `${label}，第 ${start}${end !== start ? `-${end}` : ''} 页${count}`
  }

  function footerCandidateRole(candidate, selectedOverlayFile) {
    const key = candidateKey(candidate)
    const file = selectedOverlayFile.value
    if (!file) return ''
    if (file.ignoredFooterCandidateKeys?.includes?.(key)) return 'ignore'
    if (key && key === file.existingFooterCandidateKey) return 'footer'
    if (key && key === file.existingPageNumberCandidateKey) return 'pageNumber'
    return ''
  }

  function footerCandidatesNeedReview(file) {
    const candidates = file?.footerCandidateChoices || []
    if (candidates.length <= 1) return false
    return candidates.some((candidate) => footerCandidateRoleForFile(file, candidate) === '')
  }

  function footerCandidateRoleForFile(file, candidate) {
    const key = candidateKey(candidate)
    if (!file || !key) return ''
    if (file.ignoredFooterCandidateKeys?.includes?.(key)) return 'ignore'
    if (key === file.existingFooterCandidateKey) return 'footer'
    if (key === file.existingPageNumberCandidateKey) return 'pageNumber'
    return ''
  }

  function footerCandidateRoleText(candidate, selectedOverlayFile) {
    const role = footerCandidateRole(candidate, selectedOverlayFile)
    if (role === 'footer') return '当前页脚'
    if (role === 'pageNumber') return '当前页码'
    if (role === 'ignore') return '已忽略'
    return '未确认'
  }

  function footerCandidateRoleType(candidate, selectedOverlayFile) {
    const role = footerCandidateRole(candidate, selectedOverlayFile)
    if (role === 'footer' || role === 'pageNumber') return 'success'
    if (role === 'ignore') return 'info'
    return 'warning'
  }

  function previewFooterCandidate(candidate, selectedOverlayFile, previewMaxPage, previewPage, truePreview) {
    if (!candidate || !selectedOverlayFile.value) return
    const start = Number(candidate.pageRange?.start || candidate.bbox?.page || 1)
    previewPage.value = Math.min(previewMaxPage.value, Math.max(1, start))
    truePreview.value = null
  }

  function assignFooterCandidate(
    candidate,
    role,
    selectedOverlayFile,
    selectedFooterCandidateKey,
    truePreview,
    refreshPreview,
  ) {
    const file = selectedOverlayFile.value
    if (!file || !candidate) return
    const key = candidateKey(candidate)
    if (role === 'ignore') {
      file.ignoredFooterCandidateKeys = [...new Set([...(file.ignoredFooterCandidateKeys || []), key])]
      if (key === file.existingFooterCandidateKey) clearExistingFooter(file)
      if (key === file.existingPageNumberCandidateKey) clearExistingPageNumber(file)
    } else {
      file.ignoredFooterCandidateKeys = (file.ignoredFooterCandidateKeys || []).filter((item) => item !== key)
      if (role === 'footer') {
        if (key === file.existingPageNumberCandidateKey) clearExistingPageNumber(file)
        applyCandidateToExistingFooter(file, candidate)
      }
      if (role === 'pageNumber') {
        if (key === file.existingFooterCandidateKey) clearExistingFooter(file)
        applyCandidateToExistingPageNumber(file, candidate)
      }
    }
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
    selectedFooterCandidateKey.value = key
    truePreview.value = null
    refreshPreview()
  }

  function applyCandidateToExistingFooter(file, candidate) {
    const range = candidateTargetRange(candidate, file.pages)
    file.existingFooterText = candidate.text || candidate.normalizedText || ''
    file.existingFooterTargetText = candidate.text || candidate.normalizedText || ''
    file.existingFooterNormalizedText = candidate.normalizedText || candidate.text || ''
    file.existingFooterBBox = candidate.bbox || null
    file.existingFooterFontSize = candidate.fontSize || null
    file.existingFooterPageStart = range.start
    file.existingFooterPageEnd = range.end
    file.existingFooterCandidateKey = candidateKey(candidate)
    file.existingFooterEdited = false
    file.removeExistingFooter = false
  }

  function applyCandidateToExistingPageNumber(file, candidate) {
    const range = candidateTargetRange(candidate, file.pages)
    file.existingPageNumberText = candidate.text || candidate.normalizedText || ''
    file.existingPageNumberTargetText = candidate.text || candidate.normalizedText || ''
    file.existingPageNumberNormalizedText = candidate.normalizedText || candidate.text || ''
    file.existingPageNumberBBox = candidate.bbox || null
    file.existingPageNumberFontSize = candidate.fontSize || null
    file.existingPageNumberPageStart = range.start
    file.existingPageNumberPageEnd = range.end
    file.existingPageNumberCandidateKey = candidateKey(candidate)
    file.existingPageNumberEdited = false
    file.removeExistingPageNumber = false
  }

  function clearExistingFooter(file) {
    file.existingFooterText = ''
    file.existingFooterTargetText = ''
    file.existingFooterNormalizedText = ''
    file.existingFooterBBox = null
    file.existingFooterFontSize = null
    file.existingFooterPageStart = 1
    file.existingFooterPageEnd = 0
    file.existingFooterCandidateKey = ''
    file.existingFooterEdited = false
    file.convertPlainFooter = false
    file.removeExistingFooter = false
  }

  function clearExistingPageNumber(file) {
    file.existingPageNumberText = ''
    file.existingPageNumberTargetText = ''
    file.existingPageNumberNormalizedText = ''
    file.existingPageNumberBBox = null
    file.existingPageNumberFontSize = null
    file.existingPageNumberPageStart = 1
    file.existingPageNumberPageEnd = 0
    file.existingPageNumberCandidateKey = ''
    file.existingPageNumberEdited = false
    file.convertPlainPageNumber = false
    file.removeExistingPageNumber = false
  }

  function fileExistingStatus(file) {
    if (file.removeExistingHeader || file.removeExistingFooter || file.removeExistingPageNumber) {
      return { text: '删除待处理', type: 'warning' }
    }
    if (file.existingHeaderEdited || file.existingFooterEdited || file.existingPageNumberEdited) {
      return { text: '旧内容已编辑', type: 'warning' }
    }
    if (footerCandidatesNeedReview(file)) {
      return { text: '页脚需确认', type: 'warning' }
    }
    if (file.convertPlainHeader || file.convertPlainFooter || file.convertPlainPageNumber) {
      return { text: '转换待处理', type: 'warning' }
    }
    if (file.existingHeaderArtifact || file.existingFooterArtifact) {
      return { text: '现有可编辑', type: 'warning' }
    }
    if (file.existingHeaderText || file.existingFooterText) {
      return { text: '普通文本可转换', type: 'warning' }
    }
    return { text: '无旧页眉页码', type: 'success' }
  }

  function hasExistingHeader(row) {
    return Boolean(row?.existingHeaderText || row?.existingHeaderArtifact)
  }

  function hasExistingFooter(row) {
    return Boolean(row?.existingFooterText || row?.existingFooterArtifact)
  }

  function hasExistingPageNumber(row) {
    return Boolean(row?.existingPageNumberText || row?.existingPageNumberTargetText)
  }

  return {
    detectAllHeaderFooter,
    detectFileHeaderFooter,
    applyDetectionResultToFile,
    isPageNumberCandidate,
    bestPageNumberCandidate,
    bestReliablePageNumberCandidate,
    pageNumberCandidateScore,
    isStrongNonPageFooterCandidate,
    candidateKey,
    footerCandidateMeta,
    footerCandidateRole,
    footerCandidatesNeedReview,
    footerCandidateRoleForFile,
    footerCandidateRoleText,
    footerCandidateRoleType,
    previewFooterCandidate,
    assignFooterCandidate,
    applyCandidateToExistingFooter,
    applyCandidateToExistingPageNumber,
    clearExistingFooter,
    clearExistingPageNumber,
    fileExistingStatus,
    hasExistingHeader,
    hasExistingFooter,
    hasExistingPageNumber,
  }
}
