import { computed } from 'vue'
import { ElMessage } from 'element-plus'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { logWarn } from '../../../services/appLogger.js'
import { bboxOverlayStyle, textOverlayStyle } from './pdfPreviewCoordinates.js'
import { candidateKey } from './useEvidencePdfDetection.js'
import { buildHeaderFooterItems, expandPlaceholders, buildHeaderTextForGroup, selectedGroupFor } from './useEvidencePdfSession.js'
import { renderPageNumberTemplate } from './pdfPageNumberRules.js'

const TRUE_PREVIEW_DPI = 120
const PREVIEW_CHAR_WIDTH = {
  cjk: 1,
  whitespace: 0.28,
  latin: 0.56,
  other: 0.5,
}

export function useEvidencePdfPreview({
  selectedOverlayFile,
  selectedOverlayIndex,
  previewPage,
  previewReloadKey,
  previewData,
  truePreview,
  truePreviewLoading,
  previewMaxPage,
  mergedImportPlan,
  overlayRows,
  currentRules,
  overlayOutputDir,
  insertHeaderFooterEnabled,
  headerInsertEnabled,
  headerMode,
  footerInsertEnabled,
  footerEnabled,
  footerContinuous,
  selectedFooterTextGroup,
  totalOverlayPages,
  headerAlign,
  headerMarginMm,
  headerFontSize,
  headerFontFamily,
  headerOffsetXMm,
  headerColor,
  footerAlign: _footerAlign,
  footerMarginMm: _footerMarginMm,
  footerFontSize,
  footerFontFamily: _footerFontFamily,
  footerOffsetXMm: _footerOffsetXMm,
  footerColor: _footerColor,
  pageNumberStyle,
  removeAnnotations,
  annotationKinds,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
  selectedFooterCandidateKey,
}) {
  let truePreviewRequestSeq = 0
  const showRulePreviewOverlays = computed(() => !mergedImportPlan.value)

  const previewHeaderText = computed(() => {
    if (!insertHeaderFooterEnabled.value || !headerInsertEnabled.value || !selectedOverlayFile.value || headerMode.value === 'none') return ''
    if (!shouldShowLiveHeader(selectedOverlayFile.value)) return ''
    const file = selectedOverlayFile.value
    // Mirror buildHeaderFooterItems: global group wins when global apply is on,
    // and rules.headerMode overrides the group's own mode. The detected
    // existing header must not short-circuit here — processing writes the new
    // rule text regardless of file.existingHeaderText.
    const rules = currentRules.value
    const group = rules._globalHeaderGroup || selectedGroupFor(file, 'header')
    if (!group || group.enabled === false) return ''
    const mode = rules.headerMode !== undefined ? rules.headerMode : group.mode
    if (mode === 'none') return ''
    return buildHeaderTextForGroup(file, selectedOverlayIndex.value, { ...group, mode }, rules)
  })

  const previewFooterText = computed(() => {
    if (!insertHeaderFooterEnabled.value || !selectedOverlayFile.value || !footerInsertEnabled.value)
      return ''
    if (!shouldShowLiveFooter(selectedOverlayFile.value)) return ''
    const file = selectedOverlayFile.value
    // Mirror buildHeaderFooterItems' footer branch: global group wins when
    // global apply is on, and the detected existing footer must not
    // short-circuit here — processing writes the new rule text regardless of
    // file.existingFooterText. The legacy branch (file.footer ??
    // rules.footerText) is not mirrored: the workbench always defines
    // footerInsertEnabled/pageNumberEnabled in currentRules, so
    // legacyFooterMode is never true in this UI.
    const rules = currentRules.value
    const group = rules._globalFooterTextGroup || selectedGroupFor(file, 'footerText')
    const template = group && group.enabled !== false ? group.text || rules.footerTextContent : ''
    if (!template) return ''
    const page = footerContinuous.value
      ? file.pageStart + previewPage.value - 1
      : previewPage.value
    const total = footerContinuous.value ? totalOverlayPages.value : file.pages || 1
    return expandPlaceholders(template, page, total, file, selectedOverlayIndex.value, rules)
  })

  const previewHeaderStyle = computed(() =>
    textOverlayStyle('header', previewData.value, {
      align: headerAlign.value,
      marginMm: headerMarginMm.value,
      fontSize: headerFontSize.value,
      fontFamily: headerFontFamily.value,
      offsetXMm: headerOffsetXMm.value,
      color: headerColor.value,
    }),
  )

  const previewFooterStyle = computed(() => {
    const g = selectedFooterTextGroup.value || {}
    return textOverlayStyle('footer', previewData.value, {
      align: g.align || 'left',
      marginMm: g.marginMm || 10,
      fontSize: g.fontSize || 9,
      fontFamily: g.fontFamily || 'auto',
      offsetXMm: g.offsetXMm || 0,
      color: g.color || '#000000',
    })
  })

  const truePreviewFrameStyle = computed(() => ({
    aspectRatio:
      truePreview.value?.widthPx && truePreview.value?.heightPx
        ? `${truePreview.value.widthPx} / ${truePreview.value.heightPx}`
        : `${truePreview.value?.widthPt || 595.28} / ${truePreview.value?.heightPt || 841.89}`,
  }))

  const selectedFooterCandidates = computed(() => selectedOverlayFile.value?.footerCandidateChoices || [])

  const footerCandidatePanelVisible = computed(
    () => !mergedImportPlan.value && !truePreview.value && selectedFooterCandidates.value.length > 1,
  )

  const selectedFooterCandidate = computed(() => {
    const candidates = selectedFooterCandidates.value
    return (
      candidates.find((candidate) => candidateKey(candidate) === selectedFooterCandidateKey.value) ||
      candidates[0] ||
      null
    )
  })

  const footerCandidatePreviewMarker = computed(() => {
    if (!footerCandidatePanelVisible.value || !selectedFooterCandidate.value) return null
    if (
      !isPageInDetectedRange(
        previewPage.value,
        selectedFooterCandidate.value.pageRange?.start,
        selectedFooterCandidate.value.pageRange?.end,
      )
    )
      return null
    return buildCandidatePreviewMarker(selectedFooterCandidate.value)
  })

  const deletionPreviewMarkers = computed(() => {
    if (!showRulePreviewOverlays.value || !selectedOverlayFile.value || truePreview.value) return []
    const file = selectedOverlayFile.value
    const markers = []
    // Deletion happens regardless of the insert toggles, so markers must not be
    // gated by headerInsertEnabled/footerInsertEnabled/footerEnabled.
    if (
      (file.removeExistingHeader || file.convertPlainHeader) &&
      isPageInDetectedRange(previewPage.value, file.existingHeaderPageStart, file.existingHeaderPageEnd)
    ) {
      markers.push(
        buildDeletionPreviewMarker(
          file.existingHeaderBBox,
          'header',
          file.removeExistingHeader ? '删除旧页眉' : '替换旧页眉',
        ),
      )
    }
    if (
      (file.removeExistingFooter || file.convertPlainFooter) &&
      isPageInDetectedRange(previewPage.value, file.existingFooterPageStart, file.existingFooterPageEnd)
    ) {
      markers.push(
        buildDeletionPreviewMarker(
          file.existingFooterBBox,
          'footer',
          file.removeExistingFooter ? '删除旧页脚' : '替换旧页脚',
        ),
      )
    }
    if (
      (file.removeExistingPageNumber || file.convertPlainPageNumber) &&
      isPageInDetectedRange(previewPage.value, file.existingPageNumberPageStart, file.existingPageNumberPageEnd)
    ) {
      markers.push(
        buildDeletionPreviewMarker(
          file.existingPageNumberBBox,
          'footer',
          file.removeExistingPageNumber ? '删除旧页码' : '替换旧页码',
        ),
      )
    }
    return markers.filter(Boolean)
  })

  const convertedExistingPreviewOverlays = computed(() => {
    if (!showRulePreviewOverlays.value || !selectedOverlayFile.value || truePreview.value || !insertHeaderFooterEnabled.value) return []
    const items = buildHeaderFooterItems(overlayRows.value, currentRules.value, overlayOutputDir.value)
    const item = items.find((candidate) => candidate.inputPath === selectedOverlayFile.value.path)
    return (item?.extraOverlays || [])
      .filter((overlay) => {
        if (overlay.region === 'header') return headerInsertEnabled.value
        if (overlay.artifactKind === 'PageNumber') return footerEnabled.value
        return footerInsertEnabled.value
      })
      .filter((overlay) => {
        // Page number overlays carry their own local page range
        if (overlay.artifactKind !== 'PageNumber') return true
        return isPageInDetectedRange(previewPage.value, overlay.pageStart, overlay.pageEnd)
      })
      .map((overlay, index) => {
      const region = overlay.region === 'header' ? 'header' : 'footer'
      const pageNumberOverlay = overlay.artifactKind === 'PageNumber'
      const page = pageNumberOverlay
        ? (overlay.sequence === 'per-file' ? previewPage.value : selectedOverlayFile.value.pageStart + previewPage.value - 1) +
          Number(overlay.numberOffset || 0)
        : footerContinuous.value
          ? selectedOverlayFile.value.pageStart + previewPage.value - 1
          : previewPage.value
      const total = pageNumberOverlay
        ? Number(overlay.numberTotal || selectedOverlayFile.value.pages || 1)
        : footerContinuous.value
          ? totalOverlayPages.value || selectedOverlayFile.value.pages || 1
          : selectedOverlayFile.value.pages || 1
      return {
        key: `${selectedOverlayFile.value.path}-converted-${index}`,
        region,
        text: pageNumberOverlay
          ? renderPageNumberTemplate(overlay.text, page, total, overlay.numberStyle || pageNumberStyle.value)
          : expandPlaceholders(overlay.text, page, total, selectedOverlayFile.value, selectedOverlayIndex.value, currentRules.value),
        style: textOverlayStyle(region, previewData.value, {
          align: overlay.align,
          marginMm: overlay.marginMm,
          fontSize: overlay.fontSize,
          fontFamily: overlay.fontFamily,
          offsetXMm: overlay.offsetXMm,
          color: overlay.color,
        }),
      }
    })
  })

  const headerFooterOverflowWarnings = computed(() => {
    const warnings = []
    const widthPt = previewData.value?.widthPt || 595.28
    if (
      previewHeaderText.value &&
      estimateTextWidthPt(previewHeaderText.value, headerFontSize.value) > widthPt * 0.92
    ) {
      warnings.push('当前页眉可能超出页面宽度，请缩短文本、调整位置或减小字号')
    }
    if (
      previewFooterText.value &&
      estimateTextWidthPt(previewFooterText.value, footerFontSize.value) > widthPt * 0.92
    ) {
      warnings.push('当前页脚可能超出页面宽度，请缩短文本、调整位置或减小字号')
    }
    return warnings
  })

  function refreshPreview() {
    truePreview.value = null
    previewReloadKey.value += 1
  }

  function safeRefreshPreview() {
    try {
      refreshPreview()
    } catch (err) {
      void logWarn('pdf.evidence', 'refresh split preview failed', { error: err })
    }
  }

  function movePreviewPage(delta) {
    truePreviewRequestSeq += 1
    previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(previewPage.value || 1) + delta))
    truePreview.value = null
    refreshPreview()
  }

  function selectPreviewRow(row) {
    const index = overlayRows.value.findIndex((item) => item.path === row.path)
    if (index >= 0) {
      truePreviewRequestSeq += 1
      selectedOverlayIndex.value = index
      previewPage.value = 1
      truePreview.value = null
      previewReloadKey.value += 1
    }
  }

  async function renderTruePreview() {
    if (!selectedOverlayFile.value || truePreviewLoading.value) return
    const requestSeq = ++truePreviewRequestSeq
    const requestPath = selectedOverlayFile.value.path
    const requestPage = previewPage.value
    truePreviewLoading.value = true
    const items = buildHeaderFooterItems(overlayRows.value, currentRules.value, overlayOutputDir.value)
    const item = items.find((candidate) => candidate.inputPath === requestPath)
    if (!item) {
      truePreviewLoading.value = false
      return
    }

    const result = await tauriCallSafe('preview_pdf_header_footer', {
      args: {
        job: item,
        page: requestPage,
        dpi: TRUE_PREVIEW_DPI,
        annotationRule: {
          removeAnnotations: removeAnnotations.value,
          kinds: annotationKinds.value,
        },
      },
    })
    if (
      requestSeq !== truePreviewRequestSeq ||
      selectedOverlayFile.value?.path !== requestPath ||
      previewPage.value !== requestPage
    ) {
      truePreviewLoading.value = false
      return
    }
    if (result.ok) {
      truePreview.value = result.data
      previewData.value = result.data
    } else {
      ElMessage.error(userFacingError(result.error, '真实预览生成失败'))
    }
    if (requestSeq === truePreviewRequestSeq) {
      truePreviewLoading.value = false
    }
  }

  function handlePreviewLoaded(info) {
    previewData.value = info
  }

  function handlePreviewError(message) {
    previewData.value = {}
    ElMessage.error(userFacingError(message, 'PDF 预览失败'))
  }

  function buildCandidatePreviewMarker(candidate) {
    return {
      label: candidate.text || candidate.normalizedText || '候选',
      style: candidate.bbox ? bboxOverlayStyle(candidate.bbox) : fallbackDeletionMarkerStyle('footer'),
    }
  }

  function estimateTextWidthPt(text, fontSize) {
    return String(text || '')
      .split('')
      .reduce((sum, ch) => sum + estimatePreviewCharWidth(ch) * Number(fontSize || 10), 0)
  }

  function isPageInDetectedRange(page, start, end) {
    const current = Number(page || 1)
    const rangeStart = Number(start || 1)
    const rangeEnd = Number(end || rangeStart)
    return current >= rangeStart && current <= rangeEnd
  }

  function buildDeletionPreviewMarker(bbox, kind, label) {
    const style = bbox ? bboxOverlayStyle(bbox) : fallbackDeletionMarkerStyle(kind)
    // 标签文字（如“删除旧页眉”）比多数检测 bbox 宽：去掉 bbox 的固定宽度，
    // 配合 CSS 的 nowrap 让标记按文字展开，避免折行。
    delete style.width
    return {
      key: `${kind}-${label}`,
      label,
      style,
    }
  }

  function fallbackDeletionMarkerStyle(kind) {
    const top =
      kind === 'header'
        ? `${Math.max(1, cleanupHeaderHeightMm.value / 3)}%`
        : `${100 - Math.max(4, cleanupFooterHeightMm.value / 3)}%`
    return {
      left: '8%',
      top,
      width: '84%',
      height: '20px',
    }
  }

  function estimatePreviewCharWidth(ch) {
    if (/[\u3400-\u9fff\uf900-\ufaff]/.test(ch)) return PREVIEW_CHAR_WIDTH.cjk
    if (/\s/.test(ch)) return PREVIEW_CHAR_WIDTH.whitespace
    if (/[0-9A-Za-z]/.test(ch)) return PREVIEW_CHAR_WIDTH.latin
    return PREVIEW_CHAR_WIDTH.other
  }

  function shouldShowLiveHeader(row) {
    return Boolean(row)
  }

  function shouldShowLiveFooter(row) {
    return Boolean(row)
  }

  return {
    showRulePreviewOverlays,
    previewHeaderText,
    previewFooterText,
    previewHeaderStyle,
    previewFooterStyle,
    truePreviewFrameStyle,
    selectedFooterCandidates,
    footerCandidatePanelVisible,
    selectedFooterCandidate,
    footerCandidatePreviewMarker,
    deletionPreviewMarkers,
    convertedExistingPreviewOverlays,
    headerFooterOverflowWarnings,
    refreshPreview,
    safeRefreshPreview,
    movePreviewPage,
    selectPreviewRow,
    renderTruePreview,
    handlePreviewLoaded,
    handlePreviewError,
    buildCandidatePreviewMarker,
    estimateTextWidthPt,
    isPageInDetectedRange,
    buildDeletionPreviewMarker,
    fallbackDeletionMarkerStyle,
    estimatePreviewCharWidth,
    shouldShowLiveHeader,
    shouldShowLiveFooter,
  }
}
