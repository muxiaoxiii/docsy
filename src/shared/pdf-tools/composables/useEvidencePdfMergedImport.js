import { computed, getCurrentScope, onScopeDispose, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElMessageBox } from 'element-plus'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { emitOperationUpdate, hideLoading, showLoading, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import {
  insertRangeAtPage,
  pageCount,
} from '../../../core/pdfUtils.js'
import { createEvidenceFile, naturalCompare, sortByNatural } from './useEvidencePdfSession.js'
import { formatSplitFileName, cleanSplitBaseName } from './splitFileName.js'
import { editSplitRangeBoundary, splitRangeRowKey, splitRangeAfterDefaults, unassignedSplitRanges } from './splitRangeEditing.js'
import { validRangeSubdivision } from './batchSplitRanges.js'
import { splitRangeWarnings } from './usePdfSplitRanges.js'
import { headerFooterDetectionZoneMm } from './useEvidencePdfDetection.js'

export function useEvidencePdfMergedImport({
  overlayFiles,
  overlayOutputDir,
  _overlayRows,
  importingMergedPdf,
  detectingMergedImport,
  splittingMergedImport,
  mergedImportPlan,
  mergedImportPlans = ref([]),
  selectedMergedImportIndex,
  selectedOverlayIndex,
  previewPage,
  truePreview,
  splitNamePrefix,
  splitNameSuffix,
  splitNameDateValue,
  splitNameSeparator,
  splitNameCustomSeparator,
  splitReplacementOutputDir,
  removeBlankPages,
  _workflowMode,
  _insertHeaderFooterEnabled,
  _headerMode,
  _footerEnabled,
  _footerContinuous,
  _outputMode,
  _mergeFileName,
  previewMaxPage,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
  refreshPreview,
  safeRefreshPreview,
  applyWorkflowDefaults,
}) {
  const batchRunning = ref(false)
  const batchStopRequested = ref(false)
  const batchProgress = ref('')
  const optionBindings = { splitNamePrefix, splitNameSuffix, splitNameDateValue, splitNameSeparator, splitNameCustomSeparator, removeBlankPages }
  const reviewedPlanCount = computed(() => mergedImportPlans.value.filter(plan => plan.reviewed && plan.splitStatus !== 'complete').length)
  const pendingDetectionCount = computed(() => mergedImportPlans.value.filter(plan => !plan.splitRequest && (!plan.pagesAnalyzed || plan.detectionError)).length)

  function saveActivePlanSettings() {
    const plan = mergedImportPlan.value
    if (!plan) return
    plan.options = Object.fromEntries(Object.entries(optionBindings).map(([key, binding]) => [key, binding?.value]))
    plan.viewState = { selectedIndex: selectedMergedImportIndex.value, page: previewPage.value }
  }

  function activateMergedImportPlan(plan, internal = false) {
    if (!internal && (batchRunning.value || importingMergedPdf.value || detectingMergedImport.value || splittingMergedImport.value)) return
    if (!mergedImportPlans.value.includes(plan)) return
    saveActivePlanSettings()
    mergedImportPlan.value = plan
    for (const [key, binding] of Object.entries(optionBindings)) {
      if (binding && plan.options && key in plan.options) binding.value = plan.options[key]
    }
    selectedMergedImportIndex.value = plan.viewState?.selectedIndex || 0
    previewPage.value = plan.viewState?.page || 1
    truePreview.value = null
  }

  function requestStopMergedBatch() {
    if (batchRunning.value) batchStopRequested.value = true
  }
  if (typeof window !== 'undefined' && getCurrentScope()) {
    window.addEventListener('docsy-cancel-requested', requestStopMergedBatch)
    onScopeDispose(() => {
      batchStopRequested.value = true
      window.removeEventListener('docsy-cancel-requested', requestStopMergedBatch)
    })
  }

  async function runMergedBatch(mode) {
    if (batchRunning.value || importingMergedPdf.value || detectingMergedImport.value || splittingMergedImport.value) return
    const queue = mergedImportPlans.value.filter(plan => mode === 'detect'
      ? !plan.splitRequest && (!plan.pagesAnalyzed || plan.detectionError)
      : plan.reviewed && plan.splitStatus !== 'complete')
    if (!queue.length) return
    batchRunning.value = true
    batchStopRequested.value = false
    const operationId = showLoading(mode === 'detect' ? '正在检测多份 PDF…' : '正在拆分多份 PDF…')
    try {
      for (const [index, plan] of queue.entries()) {
        if (batchStopRequested.value) break
        activateMergedImportPlan(plan, true)
        batchProgress.value = `${mode === 'detect' ? '检测' : '拆分'} ${index + 1}/${queue.length} · ${fileName(plan.inputPath)}`
        emitOperationUpdate(operationId, batchProgress.value)
        try {
          if (mode === 'detect') await detectMergedImportPlan({ fromBatch: true })
          else await executeMergedImportPlan({ fromBatch: true })
        } catch (error) {
          if (mode === 'detect') {
            plan.detectionError = String(error?.message || error)
            plan.warnings = [plan.detectionError]
          } else {
            plan.splitStatus = 'failed'
            plan.splitError = String(error?.message || error)
          }
        }
        if (/cancel|取消|中止/i.test(plan.detectionError || plan.splitError || '')) batchStopRequested.value = true
      }
    } finally {
      batchProgress.value = ''
      batchRunning.value = false
      hideLoading(operationId)
    }
  }

  function mergedPlanStatus(plan) {
    if (plan === mergedImportPlan.value && detectingMergedImport.value) return '检测中'
    if (plan.splitStatus === 'running') return '正在拆分'
    if (plan.splitStatus === 'complete') return '已拆分'
    if (plan.splitRequest) return plan.outputs?.length ? '部分完成' : '拆分失败'
    if (plan.splitError) return '拆分受阻'
    if (plan.detectionError) return '检测失败'
    if (plan.reviewed) return '已核对'
    return plan.pagesAnalyzed ? '待核对' : plan.manualEdited ? '手动分段' : '未检测'
  }

  function reopenMergedImportPlan(plan) {
    if (batchRunning.value || splittingMergedImport.value || detectingMergedImport.value || plan.outputs?.length) return
    plan.splitRequest = null
    plan.splitError = ''
    plan.splitStatus = ''
    plan.reviewed = false
    activateMergedImportPlan(plan)
  }

  const mergedImportWarnings = computed(() => {
    if (!mergedImportPlan.value) return []
    return [
      ...(mergedImportPlan.value.warnings || []).map(message =>
        mergedImportPlan.value.items?.length > 1 && message.includes('仅生成一个默认页段')
          ? `未识别到稳定页眉，当前按手动设置的 ${mergedImportPlan.value.items.length} 个页段拆分`
          : message),
      ...splitRangeWarnings(mergedImportPlan.value.items || [], mergedImportPlan.value.totalPages),
    ]
  })

  const selectedMergedImportRange = computed(
    () => mergedImportPlan.value?.items?.[selectedMergedImportIndex.value] || null,
  )
  const mergedImportUndoStack = computed({
    get: () => mergedImportPlan.value?.undoStack || [],
    set: value => { if (mergedImportPlan.value) mergedImportPlan.value.undoStack = value },
  })
  const mergedImportRedoStack = computed({
    get: () => mergedImportPlan.value?.redoStack || [],
    set: value => { if (mergedImportPlan.value) mergedImportPlan.value.redoStack = value },
  })
  const canUndoMergedImport = computed(() => mergedImportUndoStack.value.length > 0)
  const canRedoMergedImport = computed(() => mergedImportRedoStack.value.length > 0)
  const canMergeCurrentMergedImportRange = computed(() => {
    const selected = selectedMergedImportRange.value
    return Boolean(selected && [...(mergedImportPlan.value?.items || [])].some(row => row !== selected && row.pageStart < selected.pageStart))
  })
  const mergedImportGaps = computed(() => unassignedSplitRanges(mergedImportPlan.value?.items || [], mergedImportPlan.value?.totalPages || 0))

  function mergedImportSnapshot() {
    return {
      items: (mergedImportPlan.value?.items || []).map((item) => ({ ...item, rangeId: splitRangeRowKey(item) })),
      selectedIndex: selectedMergedImportIndex.value,
      previewPage: previewPage.value,
    }
  }

  function restoreMergedImportSnapshot(snapshot) {
    if (!mergedImportPlan.value || !snapshot) return false
    mergedImportPlan.value.reviewed = false
    const existing = new Map(mergedImportPlan.value.items.map(row => [splitRangeRowKey(row), row]))
    mergedImportPlan.value.items.splice(0, mergedImportPlan.value.items.length, ...snapshot.items.map(item => {
      const row = existing.get(item.rangeId) || {}
      return Object.assign(row, item)
    }))
    selectedMergedImportIndex.value = Math.min(
      Math.max(0, Number(snapshot.selectedIndex || 0)),
      Math.max(0, mergedImportPlan.value.items.length - 1),
    )
    previewPage.value = Math.min(Number(mergedImportPlan.value.totalPages || previewMaxPage.value), Math.max(1, Number(snapshot.previewPage || 1)))
    truePreview.value = null
    return true
  }

  function runMergedImportMutation(mutate) {
    if (batchRunning.value || mergedImportPlan.value?.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return
    if (!Array.isArray(mergedImportPlan.value.undoStack)) mergedImportPlan.value.undoStack = []
    if (!Array.isArray(mergedImportPlan.value.redoStack)) mergedImportPlan.value.redoStack = []
    const before = mergedImportSnapshot()
    const beforeItems = JSON.stringify(before.items)
    const result = mutate()
    if (JSON.stringify(mergedImportPlan.value?.items || []) !== beforeItems) {
      mergedImportPlan.value.reviewed = false
      mergedImportPlan.value.manualEdited = true
      mergedImportUndoStack.value.push(before)
      if (mergedImportUndoStack.value.length > 50) mergedImportUndoStack.value.shift()
      mergedImportRedoStack.value = []
    }
    return result
  }

  function resetMergedImportHistory() {
    mergedImportUndoStack.value = []
    mergedImportRedoStack.value = []
  }

  function undoMergedImportEdit() {
    if (batchRunning.value || mergedImportPlan.value?.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return false
    const snapshot = mergedImportUndoStack.value.pop()
    if (!snapshot || !mergedImportPlan.value) return false
    mergedImportRedoStack.value.push(mergedImportSnapshot())
    return restoreMergedImportSnapshot(snapshot)
  }

  function redoMergedImportEdit() {
    if (batchRunning.value || mergedImportPlan.value?.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return false
    const snapshot = mergedImportRedoStack.value.pop()
    if (!snapshot || !mergedImportPlan.value) return false
    mergedImportUndoStack.value.push(mergedImportSnapshot())
    return restoreMergedImportSnapshot(snapshot)
  }

  async function importMergedPdfAsEvidence(providedPaths) {
    if (batchRunning.value || importingMergedPdf.value || detectingMergedImport.value || splittingMergedImport.value) return
    importingMergedPdf.value = true
    saveActivePlanSettings()
    try {
      const selected = Array.isArray(providedPaths) ? providedPaths : await open({
        multiple: true,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      })
      if (!selected) return
      const paths = [...new Set(Array.isArray(selected) ? selected : [selected])]
        .filter(path => !mergedImportPlans.value.some(plan => plan.inputPath === path))
      const imported = []
      for (const inputPath of paths) {
        const countResult = await tauriCallSafe('get_pdf_page_count', { input: inputPath })
        const total = countResult.ok ? Number(countResult.data) : 0
        const valid = Number.isInteger(total) && total > 0
        const plan = buildManualMergedImportPlan(inputPath, defaultMergedImportOutputDir(inputPath), valid ? total : 0)
        if (!valid) {
          plan.detectionError = countResult.error || '无法读取 PDF 页数，请重试检测'
          plan.warnings = [plan.detectionError]
        }
        mergedImportPlans.value.push(plan)
        imported.push(plan)
      }
      mergedImportPlans.value.sort((left, right) =>
        naturalCompare(fileName(left.inputPath), fileName(right.inputPath)) || naturalCompare(left.inputPath, right.inputPath))
      if (imported.length) {
        if (!overlayFiles.value.some(file => file.sourceInputPath)) overlayFiles.value = []
        activateMergedImportPlan(imported[0], true)
        ElMessage.success(`已导入 ${imported.length} 份 PDF，请手动检测页段`)
      }
    } catch (error) {
      ElMessage.error(userFacingError(error, '导入 PDF 失败'))
    } finally {
      importingMergedPdf.value = false
    }
  }

  async function detectMergedImportPlan(options = {}) {
    const plan = mergedImportPlan.value
    if (!plan || plan.splitRequest || (batchRunning.value && !options.fromBatch) || detectingMergedImport.value || splittingMergedImport.value || importingMergedPdf.value) return
    detectingMergedImport.value = true
    plan.reviewed = false
    plan.detectionError = ''
    for (const candidate of plan.cleanupCandidates || []) candidate.selected = false
    try {
      if (!plan.totalPages) {
        const count = await tauriCallSafe('get_pdf_page_count', { input: plan.inputPath })
        if (!count.ok || !Number.isInteger(Number(count.data)) || Number(count.data) < 1) {
          plan.detectionError = count.error || '无法读取 PDF 页数'
          plan.warnings = [plan.detectionError]
          return
        }
        plan.totalPages = Number(count.data)
        plan.items = [defaultMergedImportRange(plan.inputPath, plan.totalPages)]
      }
      const headerScanMm = headerFooterDetectionZoneMm(cleanupHeaderHeightMm.value)
      const footerScanMm = headerFooterDetectionZoneMm(cleanupFooterHeightMm.value)
      const inspect = await tauriCallSafe('inspect_merged_evidence_pdf', {
        args: {
          inputPath: plan.inputPath,
          maxPages: plan.totalPages,
          headerZoneMm: headerScanMm,
          footerZoneMm: footerScanMm,
        },
      })
      if (!inspect.ok) {
        plan.detectionError = inspect.error || '页段检测失败'
        plan.warnings = [inspect.error || '合并 PDF 页段检测失败，已保留当前手动页段']
        ElMessage.warning('页段检测失败，当前手动页段保持不变')
        return
      }
      const items = (inspect.data.items || [])
        .filter((item) => Number(item.pageStart) > 0 && Number(item.pageEnd) >= Number(item.pageStart))
        .map((item, index) => detectedMergedImportItem(item, plan.inputPath, index))
      if (!items.length) {
        plan.detectionError = '未识别到可用页段，请手动核对'
        plan.warnings = [...(inspect.data.warnings || []), '未识别到可用页段，当前手动页段保持不变']
        ElMessage.warning('未识别到可用页段，请继续手动拆分')
        return
      }
      plan.totalPages = Math.max(1, Number(inspect.data.totalPages || plan.totalPages || 1))
      plan.pagesAnalyzed = Number(inspect.data.pagesAnalyzed || 0)
      plan.headerPages = Number(inspect.data.headerPages || 0)
      plan.pageNumberFooterPages = Number(inspect.data.pageNumberFooterPages || 0)
      plan.warnings = [...(inspect.data.warnings || [])]
      plan.cleanupCandidates = (inspect.data.cleanupCandidates || [])
        .filter(candidate => candidate.source === 'artifact')
        .map(candidate => ({ ...candidate, selected: false }))
      mergedImportUndoStack.value.push(mergedImportSnapshot())
      mergedImportRedoStack.value = []
      plan.items = items
      selectedMergedImportIndex.value = 0
      previewPage.value = items[0].pageStart
      truePreview.value = null
      safeRefreshPreview()
      ElMessage.success(`检测完成，已生成 ${items.length} 个候选页段，请逐项核对`)
    } finally {
      plan.detectionAttempted = true
      detectingMergedImport.value = false
    }
  }

  function detectedMergedImportItem(item, inputPath, index) {
    const rawName = String(item.name || '').trim()
    return {
      name: cleanSplitBaseName(rawName) || defaultMergedImportName(inputPath, index),
      pageStart: Number(item.pageStart),
      pageEnd: Number(item.pageEnd),
      source: item.source || 'unknown',
      existingPageNumberHasTotal: item.hasTotal || false,
      existingPageNumberSequenceForm: item.sequenceForm || '',
    }
  }

  async function importMergedPdfsForBatch(paths) {
    await importMergedPdfAsEvidence(paths)
  }

  async function confirmPdfSignatureRisk(inputPath) {
    const result = await tauriCallSafe('detect_pdf_signatures', { input: inputPath })
    if (!result.ok || !result.data?.hasSignatures) return true
    try {
      await ElMessageBox.confirm(
        `「${fileName(inputPath)}」${result.data.summary}。继续处理可能会压平签章外观或使数字签名失效（可见图像通常仍保留）。是否继续？`,
        '检测到电子签章',
        { type: 'warning', confirmButtonText: '继续处理', cancelButtonText: '取消' },
      )
      return true
    } catch {
      return false
    }
  }

  async function executeMergedImportPlan(options = {}) {
    const plan = mergedImportPlan.value
    if (!plan || plan.splitStatus === 'complete' || (batchRunning.value && !options.fromBatch) ||
        splittingMergedImport.value || detectingMergedImport.value || importingMergedPdf.value) return
    saveActivePlanSettings()
    const directoryKey = path => String(path || '').replace(/\\/g, '/').replace(/\/$/, '').toLowerCase()
    if (mergedImportPlans.value.some(other => other !== plan && directoryKey(other.outputDir) === directoryKey(plan.outputDir))) {
      plan.splitError = '不同源文件需要独立的输出目录'
      ElMessage.warning(plan.splitError)
      return
    }
    splittingMergedImport.value = true
    plan.splitError = ''
    try {
      if (!plan.splitRequest) {
        const items = normalizedMergedImportItems()
        const invalid = !items.length || items.some(item => !item.name || item.pageStart < 1 || item.pageEnd < item.pageStart)
        const warnings = splitRangeWarnings(items, plan.totalPages)
        if (invalid || warnings.length) {
          plan.splitError = warnings[0] || '请先修正文件名或页码范围'
          ElMessage.warning(plan.splitError)
          return
        }
        if (!(await confirmPdfSignatureRisk(plan.inputPath))) {
          plan.splitStatus = ''
          return
        }
        const headerScanMm = headerFooterDetectionZoneMm(cleanupHeaderHeightMm?.value)
        const footerScanMm = headerFooterDetectionZoneMm(cleanupFooterHeightMm?.value)
        plan.splitRequest = {
          inputPath: plan.inputPath,
          outputDir: plan.outputDir,
          items,
          removeBlankPages: Boolean(removeBlankPages?.value),
          headerZoneMm: headerScanMm,
          footerZoneMm: footerScanMm,
          cleanupTargets: (plan.cleanupCandidates || [])
            .filter(candidate => candidate.selected)
            .map(candidate => ({
              region: candidate.region,
              normalizedText: candidate.normalizedText,
              pageStart: candidate.pageRange?.start || candidate.pageStart,
              pageEnd: candidate.pageRange?.end || candidate.pageEnd,
              source: candidate.source || 'content-text',
              text: candidate.text,
              bbox: candidate.bbox,
            })),
        }
      }
      const finished = new Set((plan.outputs || []).map(sourcePageRangeKey))
      const remaining = plan.splitRequest.items.filter(item => !finished.has(sourcePageRangeKey(item)))
      plan.splitStatus = 'running'
      const split = await tauriCallSafe('split_merged_evidence_pdf', {
        args: { ...plan.splitRequest, items: remaining },
      })
      if (!split.ok) {
        plan.splitError = userFacingError(split.error, '拆分合并 PDF 失败')
        plan.splitStatus = 'failed'
        ElMessage.error(plan.splitError)
        return
      }
      const outputs = split.data.outputs || []
      plan.outputs = [...(plan.outputs || []), ...outputs]
      const itemOrder = new Map(plan.splitRequest.items.map((item, index) => [sourcePageRangeKey(item), index]))
      plan.outputs.sort((left, right) => itemOrder.get(sourcePageRangeKey(left)) - itemOrder.get(sourcePageRangeKey(right)))
      plan.splitWarnings = [...(split.data.warnings || [])]
      const completed = new Set(plan.outputs.map(sourcePageRangeKey))
      const complete = plan.splitRequest.items.every(item => completed.has(sourcePageRangeKey(item)))
      plan.splitStatus = complete ? 'complete' : 'partial'
      const failures = (split.data.failed || []).map(failure => typeof failure === 'string' ? failure : JSON.stringify(failure))
      plan.splitError = complete ? '' : failures.join('；') || '部分页段未输出，可重试剩余页段'
      const rawItemByRange = new Map(plan.items.map(item => [sourcePageRangeKey(item), item]))
      const newFiles = outputs.map(output => {
        const sourceItem = rawItemByRange.get(sourcePageRangeKey(output)) || {}
        const pages = Math.max(0, Number(output.pageEnd || 0) - Number(output.pageStart || 0) + 1 - Number(output.removedBlankPages || 0))
        const needsReview = sourceItem.source === 'fallback' || sourceItem.source === 'manual' ||
          hasSplitWarning(plan.splitWarnings, output)
        return {
          ...createEvidenceFile(output.outputPath),
          header: sourceItem.name || output.name,
          pages,
          sourceInputPath: plan.inputPath,
          sourcePageStart: Number(output.pageStart || 0),
          sourcePageEnd: Number(output.pageEnd || 0),
          sourceDetectionSource: sourceItem.source || 'unknown',
          detectionSummary: `来自 ${fileName(plan.inputPath)} 第 ${output.pageStart}-${output.pageEnd} 页`,
          statusText: needsReview ? '需核对' : '就绪',
          statusType: needsReview ? 'warning' : 'success',
        }
      })
      const previous = overlayFiles.value.filter(file => file.sourceInputPath)
      overlayFiles.value = [...previous, ...newFiles]
      overlayOutputDir.value = plan.outputDir
      splitReplacementOutputDir.value = ''
      selectedOverlayIndex.value = Math.max(0, overlayFiles.value.length - newFiles.length)
      if (complete) {
        plan.reviewed = false
        ElMessage.success(`${fileName(plan.inputPath)}：已输出 ${plan.outputs.length} 个文件`)
        if (mergedImportPlans.value.length <= 1) {
          mergedImportPlan.value = null
          previewPage.value = 1
          applyWorkflowDefaults()
          refreshPreview()
        }
      } else {
        ElMessage.warning(`${fileName(plan.inputPath)}：已输出 ${plan.outputs.length} 个文件，剩余页段可重试`)
      }
    } finally {
      splittingMergedImport.value = false
    }
  }

  async function selectMergedImportOutputDir() {
    if (batchRunning.value || detectingMergedImport.value || splittingMergedImport.value || mergedImportPlan.value?.splitRequest) return
    if (!mergedImportPlan.value) return
    const selected = await open({ directory: true })
    if (!selected) return
    mergedImportPlan.value.outputDir = selected
  }

  function cancelMergedImportPlan() {
    removeMergedImportPlan(mergedImportPlan.value)
  }

  function removeMergedImportPlan(plan) {
    if (batchRunning.value || detectingMergedImport.value || splittingMergedImport.value || importingMergedPdf.value) return
    if (!plan) return
    mergedImportPlans.value = mergedImportPlans.value.filter(item => item !== plan)
    overlayFiles.value = overlayFiles.value.filter(file => file.sourceInputPath !== plan.inputPath)
    if (mergedImportPlan.value !== plan) return
    mergedImportPlan.value = null
    resetMergedImportHistory()
    selectedMergedImportIndex.value = 0
    previewPage.value = 1
    if (mergedImportPlans.value.length) activateMergedImportPlan(mergedImportPlans.value[0], true)
    refreshPreview()
  }

  function normalizedMergedImportItems() {
    return (mergedImportPlan.value?.items || []).map((item, index) => ({
      name: formatSplitOutputName(item, index),
      pageStart: Number(item.pageStart || 0),
      pageEnd: Number(item.pageEnd || 0),
      source: item.source || 'unknown',
    }))
  }

  function defaultMergedImportName(inputPath, index) {
    if (index === 0) {
      return '目录'
    }
    return `文件${index + 1}`
  }

  function defaultMergedImportOutputDir(inputPath) {
    const stem = stripPdf(fileName(inputPath)) || '合并PDF'
    return `${parentDir(inputPath)}/${stem}-分项`
  }

  function defaultMergedBatchOutputDir(paths = []) {
    const first = Array.isArray(paths) ? paths[0] : ''
    return `${parentDir(first || '.')}/合并证据处理`
  }

  function defaultMergedImportRange(inputPath, total) {
    return {
      name: defaultMergedImportName(inputPath, 0),
      pageStart: 1,
      pageEnd: Math.max(1, Number(total || 1)),
      source: 'manual',
    }
  }

  function splitOutputNamePreview(row, index) {
    return formatSplitOutputName(row, index)
  }

  function formatSplitOutputName(row, index) {
    const base = String(row?.name || defaultMergedImportName('', index)).trim() || defaultMergedImportName('', index)
    return formatSplitFileName({
      base,
      index,
      prefix: splitNamePrefix.value,
      suffix: splitNameSuffix.value,
      dateValue: splitNameDateValue.value,
      separator: splitNameSeparator.value,
      customSeparator: splitNameCustomSeparator.value,
    })
  }

  function buildManualMergedImportPlan(inputPath, outputDir, total, warnings = []) {
    return {
      inputPath,
      outputDir,
      totalPages: Math.max(0, Number(total || 0)),
      pagesAnalyzed: 0,
      headerPages: 0,
      pageNumberFooterPages: 0,
      warnings,
      items: total > 0 ? [defaultMergedImportRange(inputPath, total)] : [],
      options: Object.fromEntries(Object.entries(optionBindings).map(([key, binding]) => [key, binding?.value])),
      undoStack: [],
      redoStack: [],
      reviewed: false,
      outputs: [],
    }
  }

  function selectMergedImportRange(row, boundary = 'start') {
    if (!row) return false
    const index = mergedImportPlan.value?.items?.indexOf(row) ?? -1
    if (index < 0) return false
    selectedMergedImportIndex.value = index
    // Naming is metadata-only: retain the currently displayed page and render.
    if (boundary == null) return true
    const target = Number(boundary === 'end' ? row.pageEnd : row.pageStart)
    if (!Number.isFinite(target) || target < 1) return true
    const page = Math.min(Number(mergedImportPlan.value.totalPages || previewMaxPage.value), target)
    if (previewPage.value !== page) {
      previewPage.value = page
      truePreview.value = null
    }
    return true
  }

  function setSelectedMergedRangeStart() {
    const items = mergedImportPlan.value?.items
    if (!items) return
    const index = items.indexOf(selectedMergedImportRange.value)
    if (index < 0) return
    updateMergedRangeBoundary(index, previewPage.value, 'start')
  }

  function setSelectedMergedRangeEnd() {
    const items = mergedImportPlan.value?.items
    if (!items) return
    const index = items.indexOf(selectedMergedImportRange.value)
    if (index >= 0) updateMergedRangeBoundary(index, previewPage.value, 'end')
  }

  // Every add/split entry point uses the same idempotent boundary operation.
  function addMergedImportRangeFromPage() {
    return splitMergedImportAtPage(previewPage.value)
  }

  function nextManualRangeName() {
    const items = mergedImportPlan.value?.items || []
    const names = new Set(items.map(item => item.name))
    let number = items.length + 1
    while (names.has(`文件${number}`)) number += 1
    return `文件${number}`
  }

  function splitMergedImportAtPage(page, displayPage = page) {
    const plan = mergedImportPlan.value
    if (!plan || batchRunning.value || plan.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return false
    const target = Math.round(Number(page))
    if (!Number.isInteger(target) || target <= 1 || target > plan.totalPages) return false
    const items = plan.items
    const alreadySplit = items.some(item => Number(item.pageStart) === target)
    if (!alreadySplit && items.filter(item => target >= item.pageStart && target <= item.pageEnd).length !== 1) {
      ElMessage.warning('当前页存在重叠或尚未分配，请先修正范围或使用“补齐空缺”')
      return false
    }
    const newIndex = runMergedImportMutation(() => insertRangeAtPage(items, target, plan.totalPages, {
      name: nextManualRangeName(), extra: { source: 'manual' }, reuseBoundary: true,
    }))
    if (!Number.isInteger(newIndex) || newIndex < 0) return false
    const displayed = Math.max(1, Math.min(plan.totalPages, Number(displayPage) || target))
    const activeIndex = items.findIndex(item => displayed >= item.pageStart && displayed <= item.pageEnd)
    selectedMergedImportIndex.value = activeIndex >= 0 ? activeIndex : newIndex
    previewPage.value = displayed
    truePreview.value = null
    if (alreadySplit) ElMessage.info?.('此处已经分段，无需重复拆分')
    return !alreadySplit
  }

  function splitMergedImportFromCurrentPage() {
    return splitMergedImportAtPage(previewPage.value)
  }

  function splitMergedImportAfterCurrentPage() {
    return splitMergedImportAtPage(Number(previewPage.value) + 1, previewPage.value)
  }

  function mergeCurrentMergedImportRangeIntoPrevious() {
    const plan = mergedImportPlan.value
    if (!plan?.items?.length) return
    const current = selectedMergedImportRange.value
    if (!current) return false
    const previous = [...plan.items].filter(row => row !== current && row.pageStart < current.pageStart)
      .sort((a, b) => b.pageStart - a.pageStart)[0]
    if (!previous) return false
    runMergedImportMutation(() => {
      previous.pageEnd = Math.max(previous.pageEnd, current.pageEnd)
      previous.source = 'manual'
      plan.items.splice(plan.items.indexOf(current), 1)
    })
    selectedMergedImportIndex.value = plan.items.indexOf(previous)
    return true
  }

  // Inputs are drafts until change/Enter/blur, so the undo snapshot retains
  // the original interval. Clicking the endpoint itself only navigates.
  function updateMergedRangeBoundary(index, value, boundary) {
    const plan = mergedImportPlan.value
    if (!plan) return
    if (typeof index === 'object') index = plan.items.indexOf(index)
    if (!plan.items[index]) return
    const newIndex = runMergedImportMutation(() => editSplitRangeBoundary(plan.items, index, value, plan.totalPages, boundary))
    if (Number.isInteger(newIndex) && newIndex >= 0) selectMergedImportRange(plan.items[newIndex], boundary)
  }

  function onMergedRangeStartChanged(index, value) { updateMergedRangeBoundary(index, value, 'start') }
  function onMergedRangeEndChanged(index, value) { updateMergedRangeBoundary(index, value, 'end') }

  function applyMergedRangeBatch({ inputPath, rangeId, pageStart, pageEnd, items }) {
    const plan = mergedImportPlan.value
    if (!plan || inputPath !== plan.inputPath) return false
    const index = plan.items.findIndex(item => (!rangeId || splitRangeRowKey(item) === rangeId) && item.pageStart === pageStart && item.pageEnd === pageEnd)
    if (index < 0 || !validRangeSubdivision(plan.items[index], items)) {
      ElMessage.warning('原页段已变化，请重新打开批量分段')
      return false
    }
    const applied = runMergedImportMutation(() => {
      const original = plan.items[index]
      const first = Object.assign(original, items[0], { source: 'manual' })
      plan.items.splice(index, 1, first, ...items.slice(1).map(item => ({ ...item, rangeId: undefined, source: 'manual' })))
      return true
    })
    if (!applied) return false
    const selected = plan.items.findIndex(item => previewPage.value >= item.pageStart && previewPage.value <= item.pageEnd)
    selectedMergedImportIndex.value = selected >= 0 ? selected : index
    truePreview.value = null
    return true
  }

  function addMergedImportRange() { return insertMergedImportRangeAfter() }

  function insertMergedImportRangeAfter(index = selectedMergedImportIndex.value) {
    const plan = mergedImportPlan.value
    if (!plan || batchRunning.value || plan.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return false
    if (typeof index === 'object') index = plan.items.indexOf(index)
    const defaults = splitRangeAfterDefaults(plan.items, index, plan.totalPages)
    if (!defaults) return false
    const item = { name: nextManualRangeName(), ...defaults, source: 'manual' }
    const added = runMergedImportMutation(() => {
      plan.items.splice(index + 1, 0, item)
      return true
    })
    if (!added) return false
    selectedMergedImportIndex.value = index + 1
    // Adding a list row does not navigate the PDF or depend on the viewed page.
    if (item.pageStart == null) ElMessage.info?.('相邻页段没有空余页面，已插入待设置行；请设置起点并调整相邻边界')
    return true
  }

  function fillMergedImportGaps() {
    const plan = mergedImportPlan.value
    if (!plan || !mergedImportGaps.value.length || batchRunning.value || plan.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return false
    if (plan.items.some(row => !row.pageStart || !row.pageEnd || row.pageStart > row.pageEnd)) {
      ElMessage.warning('请先补全待设置的页段，并修正起始页大于结束页的范围')
      return false
    }
    const selected = selectedMergedImportRange.value
    runMergedImportMutation(() => {
      for (const gap of mergedImportGaps.value) {
        const next = [...plan.items].filter(row => row.pageStart > gap.pageEnd).sort((a, b) => a.pageStart - b.pageStart)[0]
        const index = next ? plan.items.indexOf(next) : plan.items.length
        plan.items.splice(index, 0, { ...gap, name: nextManualRangeName(), source: 'manual' })
      }
    })
    selectedMergedImportIndex.value = Math.max(0, plan.items.indexOf(selected))
    return true
  }

  function removeMergedImportRange(index) {
    if (!mergedImportPlan.value || batchRunning.value || mergedImportPlan.value.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return
    const items = mergedImportPlan.value.items
    if (items.length <= 1) {
      ElMessage.info?.('至少保留一个页段；如不处理此文件，可移除源文件')
      return
    }
    if (index < 0 || index >= items.length) return
    const selected = selectedMergedImportRange.value
    let recipient
    runMergedImportMutation(() => {
      const removed = items[index]
      if (!removed.pageStart || !removed.pageEnd || removed.pageStart > removed.pageEnd) {
        recipient = items[index - 1] || items[index + 1]
        items.splice(index, 1)
        return
      }
      const sorted = [...items].sort((a, b) => a.pageStart - b.pageStart)
      const physicalIndex = sorted.indexOf(removed)
      const previous = sorted[physicalIndex - 1], next = sorted[physicalIndex + 1]
      recipient = previous || next
      items.splice(index, 1)
      if (previous) {
        previous.pageEnd = Math.max(previous.pageEnd, removed.pageEnd)
        previous.source = 'manual'
      } else if (next) {
        next.pageStart = Math.min(next.pageStart, removed.pageStart)
        next.pageEnd = Math.max(next.pageEnd, removed.pageEnd)
        next.source = 'manual'
      }
    })
    selectedMergedImportIndex.value = Math.max(0, items.indexOf(items.includes(selected) ? selected : recipient))
  }

  function reorderMergedImportRanges(from, to) {
    const items = mergedImportPlan.value?.items
    if (!items || from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) return false
    const selected = selectedMergedImportRange.value
    const changed = runMergedImportMutation(() => {
      const [row] = items.splice(from, 1)
      items.splice(to, 0, row)
      return true
    })
    if (changed) selectedMergedImportIndex.value = Math.max(0, items.indexOf(selected))
    return Boolean(changed)
  }

  function sortMergedImportItems({ prop, order }) {
    if (detectingMergedImport.value || splittingMergedImport.value) return
    if (!mergedImportPlan.value || !prop || !order) return
    const selected = selectedMergedImportRange.value
    const sorted = sortByNatural(
      mergedImportPlan.value.items,
      (row, index) => mergedImportSortValue(row, prop, index),
      order,
    )
    runMergedImportMutation(() => mergedImportPlan.value.items.splice(0, mergedImportPlan.value.items.length, ...sorted))
    if (selected) {
      selectedMergedImportIndex.value = Math.max(0, mergedImportPlan.value.items.indexOf(selected))
    }
  }

  function mergedImportSortValue(row, prop, index) {
    if (prop === 'outputName') return splitOutputNamePreview(row, index)
    if (prop === 'pageStart') return Number(row?.pageStart || 0)
    if (prop === 'pageEnd') return Number(row?.pageEnd || 0)
    if (prop === 'pageCount') return pageCount(row)
    if (prop === 'source') return mergedImportSourceText(row)
    return row?.[prop] ?? ''
  }

  function mergedImportSourceType(row) {
    return row?.source === 'fallback' || row?.source === 'manual' ? 'warning' : 'success'
  }

  function mergedImportSourceText(row) {
    if (row?.source === 'fallback') return '需核对'
    if (row?.source === 'manual') return '手动'
    return '页眉'
  }

  function sourcePageRangeKey(item) {
    return `${Number(item.pageStart || 0)}-${Number(item.pageEnd || 0)}`
  }

  function hasSplitWarning(warnings, output) {
    const name = String(output.name || '').trim()
    if (!name) return false
    return warnings.some((warning) => String(warning || '').includes(name))
  }

  return {
    mergedImportPlans,
    batchRunning,
    batchStopRequested,
    batchProgress,
    reviewedPlanCount,
    pendingDetectionCount,
    removeMergedImportPlan,
    activateMergedImportPlan,
    requestStopMergedBatch,
    detectAllMergedImports: () => runMergedBatch('detect'),
    splitReviewedMergedImports: () => runMergedBatch('split'),
    mergedPlanStatus,
    reopenMergedImportPlan,
    mergedImportWarnings,
    mergedImportGaps,
    fillMergedImportGaps,
    splitRangeRowKey,
    selectedMergedImportRange,
    canUndoMergedImport,
    canRedoMergedImport,
    canMergeCurrentMergedImportRange,
    importMergedPdfAsEvidence,
    detectMergedImportPlan,
    importMergedPdfsForBatch,
    executeMergedImportPlan,
    selectMergedImportOutputDir,
    cancelMergedImportPlan,
    normalizedMergedImportItems,
    defaultMergedImportName,
    defaultMergedImportOutputDir,
    defaultMergedBatchOutputDir,
    defaultMergedImportRange,
    splitOutputNamePreview,
    formatSplitOutputName,
    buildManualMergedImportPlan,
    selectMergedImportRange,
    setSelectedMergedRangeStart,
    setSelectedMergedRangeEnd,
    addMergedImportRangeFromPage,
    splitMergedImportAtPage,
    splitMergedImportFromCurrentPage,
    splitMergedImportAfterCurrentPage,
    applyMergedRangeBatch,
    mergeCurrentMergedImportRangeIntoPrevious,
    undoMergedImportEdit,
    redoMergedImportEdit,
    onMergedRangeStartChanged,
    onMergedRangeEndChanged,
    addMergedImportRange,
    insertMergedImportRangeAfter,
    removeMergedImportRange,
    sortMergedImportItems,
    reorderMergedImportRanges,
    mergedImportSortValue,
    mergedImportRangePageCount: pageCount,
    mergedImportSourceType,
    mergedImportSourceText,
    headerFooterDetectionZoneMm,
    sourcePageRangeKey,
    hasSplitWarning,
  }
}
