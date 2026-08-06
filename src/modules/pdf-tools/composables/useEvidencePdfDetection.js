import { ElMessage } from 'element-plus'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { candidateTargetRange } from './useEvidencePdfSession.js'
import { candidateIdentity, detectedElementFromCandidate, mergeExistingElements } from './existingPdfElements.js'

const DETECTION_SCAN_MAX_PAGES = 20
const ROMAN_PAGE_SCORE_PENALTY = -0.25

export function headerFooterDetectionZoneMm(value) {
  return Math.max(25, Math.min(60, Number(value || 0) || 25))
}

export function candidateKey(candidate) {
  return candidateIdentity(candidate)
}

export function useEvidencePdfDetection({
  overlayRows,
  detectingAllHeaderFooter,
  detectionProgressText,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
}) {
  async function detectAllHeaderFooter(options = {}) {
    const silent = Boolean(options.silent)
    if (!overlayRows.value.length || detectingAllHeaderFooter.value) return
    detectingAllHeaderFooter.value = true
    detectionProgressText.value = ''
    let success = 0
    let failed = 0
    const total = overlayRows.value.length
    const results = []
    const startTime = Date.now()
    // Elapsed timer — updates the progress text every second via setInterval
    const timerId = setInterval(() => {
      const elapsed = Math.floor((Date.now() - startTime) / 1000)
      const m = String(Math.floor(elapsed / 60)).padStart(2, '0')
      const s = String(elapsed % 60).padStart(2, '0')
      detectionProgressText.value = `正在检测 ${results.length}/${total} 个文件  ${m}:${s}`
    }, 1000)
    try {
      // Detect all files — NO per-file UI updates, only the timer above
      for (let i = 0; i < total; i++) {
        const file = overlayRows.value[i]
        const result = await detectFileHeaderFooter(file)
        results.push({ file, result })
      }
      // Final progress update
      clearInterval(timerId)
      const elapsed = Math.floor((Date.now() - startTime) / 1000)
      detectionProgressText.value = `检测完成 ${results.length}/${total} 个文件  ${elapsed}s`
      // Phase 2: apply all results at once (single re-render cycle)
      for (const { file, result } of results) {
        if (result.ok) {
          try {
            applyDetectionResultToFile(file, result.data || {})
            const status = fileExistingStatus(file)
            file.statusText = status.text
            file.statusType = status.type
            file.statusDetail = ''
            success += 1
          } catch (applyErr) {
            file.statusText = '检测结果处理失败'
            file.statusType = 'danger'
            file.statusDetail = applyErr?.message || '数据解析错误'
            failed += 1
          }
        } else {
          file.statusText = '检测失败'
          file.statusType = 'danger'
          file.statusDetail = result.error || '未知错误'
          failed += 1
        }
      }
      if (!silent) {
        failed
          ? ElMessage.warning(`已检测 ${success} 个，失败 ${failed} 个`)
          : ElMessage.success(`已检测 ${success} 个 PDF`)
      }
    } finally {
      clearInterval(timerId)
      detectingAllHeaderFooter.value = false
      detectionProgressText.value = ''
    }
  }

  async function detectFileHeaderFooter(file) {
    return tauriCallSafe('detect_pdf_header_footer', {
      args: {
        inputPath: file.path,
        maxPages: DETECTION_SCAN_MAX_PAGES,
        headerZoneMm: headerFooterDetectionZoneMm(cleanupHeaderHeightMm.value),
        footerZoneMm: headerFooterDetectionZoneMm(cleanupFooterHeightMm.value),
      },
    })
  }

  function applyDetectionResultToFile(file, data) {
    const totalPages = file.pages || data.pages?.length || data.pagesAnalyzed || 1
    const headerCandidates = data.headerCandidates || []
    const footerCandidates = data.footerCandidates || []
    const pageNumberCandidates = [...headerCandidates, ...footerCandidates].filter(isPageNumberCandidate)
    const header = bestReliableHeaderCandidate(
      headerCandidates.filter((candidate) => !isPageNumberCandidate(candidate)),
      totalPages,
    )
    const pageNumber = bestReliablePageNumberCandidate(pageNumberCandidates, totalPages)
    const footer = footerCandidates.find(isStrongNonPageFooterCandidate) || null
    const candidates = [...headerCandidates.slice(0, 12), ...footerCandidates.slice(0, 12)]
    const detectedElements = [
      ...headerCandidates
        .filter((candidate) => !isPageNumberCandidate(candidate))
        .filter((candidate) => bestReliableHeaderCandidate([candidate], totalPages))
        .map((candidate, index) => detectedElementFromCandidate(candidate, 'header', index)),
      ...footerCandidates
        .filter((candidate) => !isPageNumberCandidate(candidate) && isStrongNonPageFooterCandidate(candidate))
        .map((candidate, index) => detectedElementFromCandidate(candidate, 'footerText', index)),
      ...pageNumberCandidates.map((candidate, index) => detectedElementFromCandidate(candidate, 'pageNumber', index)),
    ]
    file.existingElements = mergeExistingElements(file.existingElements || [], detectedElements)
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
    file.existingPageNumberSequenceForm = pageNumber?.sequenceForm || ''
    file.existingPageNumberHasTotal = Boolean(pageNumber?.hasTotal)
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
    file.ignoredFooterCandidateKeys = file.existingElements
      .filter((element) => element.decision === 'ignore')
      .map((element) => element.id)
    const headerTargetRange = candidateTargetRange(header, file.pages)
    const footerTargetRange = candidateTargetRange(footer, file.pages)
    const pageNumberTargetRange = candidateTargetRange(pageNumber, file.pages)
    file.existingHeaderPageStart = headerTargetRange.start
    file.existingHeaderPageEnd = headerTargetRange.end
    file.existingFooterPageStart = footerTargetRange.start
    file.existingFooterPageEnd = footerTargetRange.end
    file.existingPageNumberPageStart = pageNumberTargetRange.start
    file.existingPageNumberPageEnd = pageNumberTargetRange.end
    // A document-level Artifact summary cannot prove that a text candidate is
    // inside that marked-content range. Only object-linked candidates may use
    // the lossless Artifact edit path.
    file.existingHeaderArtifact = header?.source === 'artifact'
    file.existingFooterArtifact = footer?.source === 'artifact'
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

  function bestReliableHeaderCandidate(candidates = [], totalPages = 1) {
    if (Number(totalPages || 1) <= 1) return candidates[0] || null
    return (
      candidates.find(
        (candidate) =>
          candidate?.source === 'artifact' ||
          (candidate?.repeating && candidate?.positionStable !== false && Number(candidate?.count || 0) >= 2),
      ) || null
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
    const count = Number(candidate?.count || 0)
    const pageRange = candidate?.pageRange || {}
    const rangeLength = Math.max(0, Number(pageRange.end || 0) - Number(pageRange.start || 0) + 1)
    if (candidate?.source === 'artifact') return true
    if (Number(totalPages || 1) <= 1) return false
    return (
      count >= 2 &&
      rangeLength >= 2 &&
      candidate?.repeating === true &&
      candidate?.positionStable !== false &&
      candidate?.sequenceStable !== false
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
    return (
      candidate?.source === 'artifact' ||
      (candidate?.repeating && candidate?.positionStable !== false && Number(candidate?.count || 0) >= 2)
    )
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
    // Only check existingElements (strong candidates that entered the confirmation dialog)
    const elements = file?.existingElements || []
    const footerElements = elements.filter(e => e.kind === 'footerText' || e.kind === 'pageNumber')
    if (footerElements.length === 0) return false
    // Need review if any footer/pageNumber element has no decision yet
    return footerElements.some(e => !e.decision)
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
    const element = (file.existingElements || []).find((item) => item.id.includes(`|${key}|`))
    if (role === 'ignore') {
      if (element) element.decision = 'ignore'
      file.ignoredFooterCandidateKeys = [...new Set([...(file.ignoredFooterCandidateKeys || []), key])]
      if (key === file.existingFooterCandidateKey) clearExistingFooter(file)
      if (key === file.existingPageNumberCandidateKey) clearExistingPageNumber(file)
    } else {
      if (element) {
        element.decision = 'keep'
        element.kind = role === 'footer' ? 'footerText' : 'pageNumber'
      }
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
    // Check if all existing elements have been confirmed (decision: keep/ignore)
    const elements = file.existingElements || []
    if (elements.length > 0) {
      const allConfirmed = elements.every(
        (el) => el.decision === 'keep' || el.decision === 'ignore',
      )
      if (allConfirmed) {
        return { text: '已确认', type: 'success' }
      }
      const anyPending = elements.some(
        (el) => el.decision == null || el.decision === undefined,
      )
      if (anyPending) {
        return { text: '待确认', type: 'warning' }
      }
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
    bestReliableHeaderCandidate,
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
