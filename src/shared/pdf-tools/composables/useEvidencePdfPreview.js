import { computed } from 'vue'
import { ElMessage } from 'element-plus'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { logWarn } from '../../../services/appLogger.js'
import { bboxOverlayStyle, textOverlayStyle } from './pdfPreviewCoordinates.js'
import { candidateKey } from './useEvidencePdfDetection.js'
import {
  buildHeaderFooterItems,
  expandPlaceholders,
} from './useEvidencePdfSession.js'
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
  resolvedHeaderFooter,
  overlayOutputDir,
  insertHeaderFooterEnabled: _insertHeaderFooterEnabled,
  headerInsertEnabled: _headerInsertEnabled,
  headerMode: _headerMode,
  footerInsertEnabled: _footerInsertEnabled,
  footerEnabled: _footerEnabled,
  footerContinuous: _footerContinuous,
  selectedFooterTextGroup: _selectedFooterTextGroup,
  totalOverlayPages: _totalOverlayPages,
  headerAlign: _headerAlign,
  headerMarginMm: _headerMarginMm,
  headerFontSize,
  headerFontFamily: _headerFontFamily,
  headerOffsetXMm: _headerOffsetXMm,
  headerColor: _headerColor,
  footerAlign: _footerAlign,
  footerMarginMm: _footerMarginMm,
  footerFontSize: _footerFontSize,
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
  const activePreviewJob = computed(() => {
    const file = selectedOverlayFile.value
    if (!file) return null
    if (resolvedHeaderFooter) return resolvedHeaderFooter.value.find(item => item.job.inputPath === file.path)?.job || null
    return buildHeaderFooterItems(overlayRows.value, currentRules.value, overlayOutputDir.value)
      .find(item => item.inputPath === file.path) || null
  })

  // 页眉/页脚 overlay 定位用的页面尺寸（pt），回退顺序：
  // 1. 预览组件上报的当前页真实尺寸（previewData，含异形页/逐页尺寸）；
  // 2. 当前选中文件检测结果里随 bbox 保存的页宽/页高（bbox.width/height 即页面 pt 尺寸）；
  // 3. 都没有时传 null，由 textOverlayStyle 回退 A4 兜底（比例可能失真，仅兜底）。
  const previewPageInfo = computed(() => {
    const data = previewData.value
    if (data?.widthPt && data?.heightPt) return data
    const file = selectedOverlayFile.value
    const bbox = file?.existingHeaderBBox || file?.existingFooterBBox || file?.existingPageNumberBBox
    if (bbox?.width && bbox?.height) {
      return { widthPt: bbox.width, heightPt: bbox.height }
    }
    return null
  })

  const previewHeaderText = computed(() => {
    const job = activePreviewJob.value
    if (!job?.header || !isPageInDetectedRange(previewPage.value, job.header.pageStart, job.header.pageEnd)) return ''
    const file = selectedOverlayFile.value
    return expandPlaceholders(job.header.text, job.pageStart + previewPage.value - 1, job.totalPages, file, selectedOverlayIndex.value, currentRules.value)
  })

  const previewFooterText = computed(() => {
    const job = activePreviewJob.value
    if (!job?.footer || !isPageInDetectedRange(previewPage.value, job.footer.pageStart, job.footer.pageEnd)) return ''
    const file = selectedOverlayFile.value
    return expandPlaceholders(job.footer.text, job.pageStart + previewPage.value - 1, job.totalPages, file, selectedOverlayIndex.value, currentRules.value)
  })

  const previewHeaderStyle = computed(() =>
    textOverlayStyle('header', previewPageInfo.value, {
      ...(activePreviewJob.value?.header || {}),
      fontSize: activePreviewJob.value?.header?.fontSize ?? headerFontSize.value,
    }),
  )

  const previewFooterStyle = computed(() => {
    const g = activePreviewJob.value?.footer || {}
    return textOverlayStyle('footer', previewPageInfo.value, {
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
    if (file.existingElements?.length) {
      const labels = { header: '页眉', footerText: '页脚', pageNumber: '页码' }
      return file.existingElements
        .filter(element => labels[element.kind] && ['delete', 'edit'].includes(element.decision) &&
          isPageInDetectedRange(previewPage.value, element.pageStart, element.pageEnd || file.pages))
        .map(element => ({
          ...buildDeletionPreviewMarker(element.pageSamples?.find(sample => sample.page === previewPage.value)?.bbox || element.bbox, element.kind === 'header' ? 'header' : 'footer',
            `${element.decision === 'delete' ? '删除' : '替换'}旧${labels[element.kind]}`),
          key: element.id,
        }))
    }
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
    if (
      !showRulePreviewOverlays.value ||
      !selectedOverlayFile.value ||
      truePreview.value
    )
      return []
    const item = activePreviewJob.value
    return (item?.extraOverlays || [])
      .filter((overlay) => isPageInDetectedRange(previewPage.value, overlay.pageStart, overlay.pageEnd))
      .map((overlay, index) => {
        const region = overlay.region === 'header' ? 'header' : 'footer'
        const pageNumberOverlay = overlay.artifactKind === 'PageNumber'
        const page = item.pageStart + previewPage.value - 1 + (pageNumberOverlay ? Number(overlay.numberOffset || 0) : 0)
        const total = pageNumberOverlay
          ? Number(overlay.numberTotal || selectedOverlayFile.value.pages || 1)
          : item.totalPages || selectedOverlayFile.value.pages || 1
        return {
          key: `${selectedOverlayFile.value.path}-converted-${index}`,
          region,
          text: pageNumberOverlay
            ? renderPageNumberTemplate(overlay.text, page, total, overlay.numberStyle || pageNumberStyle.value)
            : expandPlaceholders(
                overlay.text,
                page,
                total,
                selectedOverlayFile.value,
                selectedOverlayIndex.value,
                currentRules.value,
              ),
          style: textOverlayStyle(region, previewPageInfo.value, {
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
    const widthPt = previewPageInfo.value?.widthPt || 595.28
    if (
      previewHeaderText.value &&
      estimateTextWidthPt(previewHeaderText.value, activePreviewJob.value?.header?.fontSize ?? headerFontSize.value) > widthPt * 0.92
    ) {
      warnings.push('当前页眉可能超出页面宽度，请缩短文本、调整位置或减小字号')
    }
    if (
      previewFooterText.value &&
      estimateTextWidthPt(previewFooterText.value, activePreviewJob.value?.footer?.fontSize || 9) > widthPt * 0.92
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
      style: candidate.bbox ? bboxOverlayStyle(candidate.bbox, previewPageInfo.value) : fallbackDeletionMarkerStyle('footer'),
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
    const rangeEnd = Number(end || selectedOverlayFile.value?.pages || rangeStart)
    return current >= rangeStart && current <= rangeEnd
  }

  function buildDeletionPreviewMarker(bbox, kind, label) {
    const style = bbox ? bboxOverlayStyle(bbox, previewPageInfo.value) : fallbackDeletionMarkerStyle(kind)
    return {
      key: `${kind}-${label}`,
      label,
      style,
      // Place captions toward the page interior, outside the actual deletion area.
      captionBelow: Number.parseFloat(style.top) < 50,
      captionRight: Number.parseFloat(style.left) > 50,
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
    previewPageInfo,
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
