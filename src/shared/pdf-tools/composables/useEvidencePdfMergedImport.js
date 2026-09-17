import { computed, getCurrentScope, onScopeDispose, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElMessageBox } from 'element-plus'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { emitOperationUpdate, hideLoading, showLoading, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import {
  buildRangeAfter,
  insertRangeAfter,
  insertRangeAtPage,
  pageCount,
  smartSetRangeEnd,
  smartSetRangeStart,
} from '../../../core/pdfUtils.js'
import { createEvidenceFile, naturalCompare, sortByNatural } from './useEvidencePdfSession.js'
import { formatSplitFileName, cleanSplitBaseName } from './splitFileName.js'
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
    safeRefreshPreview()
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
    return plan.pagesAnalyzed ? '待核对' : '未检测'
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
      ...(mergedImportPlan.value.warnings || []),
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
  const currentMergedImportRangeIndex = computed(() => {
    const page = Number(previewPage.value || 1)
    return (mergedImportPlan.value?.items || []).findIndex(
      (item) => page >= Number(item.pageStart || 0) && page <= Number(item.pageEnd || 0),
    )
  })
  const canMergeCurrentMergedImportRange = computed(() => currentMergedImportRangeIndex.value > 0)

  function mergedImportSnapshot() {
    return {
      items: (mergedImportPlan.value?.items || []).map((item) => ({ ...item })),
      selectedIndex: selectedMergedImportIndex.value,
      previewPage: previewPage.value,
    }
  }

  function restoreMergedImportSnapshot(snapshot) {
    if (!mergedImportPlan.value || !snapshot) return false
    mergedImportPlan.value.reviewed = false
    mergedImportPlan.value.items = snapshot.items.map((item) => ({ ...item }))
    selectedMergedImportIndex.value = Math.min(
      Math.max(0, Number(snapshot.selectedIndex || 0)),
      Math.max(0, mergedImportPlan.value.items.length - 1),
    )
    previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(snapshot.previewPage || 1)))
    truePreview.value = null
    safeRefreshPreview()
    return true
  }

  function runMergedImportMutation(mutate) {
    if (batchRunning.value || mergedImportPlan.value?.splitRequest || detectingMergedImport.value || splittingMergedImport.value) return
    const before = mergedImportSnapshot()
    const beforeItems = JSON.stringify(before.items)
    const result = mutate()
    if (JSON.stringify(mergedImportPlan.value?.items || []) !== beforeItems) {
      mergedImportPlan.value.reviewed = false
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

  function selectMergedImportRange(row) {
    if (!row) return
    const index = mergedImportPlan.value?.items?.indexOf(row) ?? -1
    if (index >= 0) selectedMergedImportIndex.value = index
    previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(row.pageStart || 1)))
    truePreview.value = null
    refreshPreview()
  }

  function setSelectedMergedRangeStart() {
    const items = mergedImportPlan.value?.items
    if (!items) return
    const index = items.indexOf(selectedMergedImportRange.value)
    if (index < 0) return
    // 智能调整：改起始页后自动与前一段无缝衔接，前段被吃空则自动并入
    const newIndex = runMergedImportMutation(() =>
      smartSetRangeStart(items, index, previewPage.value, mergedImportPlan.value.totalPages),
    )
    selectedMergedImportIndex.value = newIndex
  }

  function setSelectedMergedRangeEnd() {
    const items = mergedImportPlan.value?.items
    if (!items) return
    const index = items.indexOf(selectedMergedImportRange.value)
    if (index < 0) return
    // 智能调整：改结束页后自动与后一段无缝衔接，后段被吃空则自动并入
    const newIndex = runMergedImportMutation(() =>
      smartSetRangeEnd(items, index, previewPage.value, mergedImportPlan.value.totalPages),
    )
    selectedMergedImportIndex.value = newIndex
  }

  // 预览区「添加新页段」：以当前预览页为起始页插入新页段，
  // 结束页与后续页段接续；边界页提出一页，段中页承接段尾，末段到最后一页。
  function addMergedImportRangeFromPage() {
    const plan = mergedImportPlan.value
    if (!plan) return
    const items = plan.items
    const newIndex = runMergedImportMutation(() =>
      insertRangeAtPage(items, previewPage.value, plan.totalPages, {
        name: `文件${items.length + 1}`,
        extra: { source: 'manual' },
      }),
    )
    if (newIndex < 0) {
      ElMessage.info('当前页已是最后一个独立页段，无需新增')
      return
    }
    selectedMergedImportIndex.value = newIndex
    const item = items[newIndex]
    previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(item.pageStart || 1)))
    truePreview.value = null
    refreshPreview()
  }

  function splitMergedImportAtPage(page) {
    const plan = mergedImportPlan.value
    if (!plan) return
    const target = Math.min(Number(plan.totalPages || 1), Math.max(1, Number(page || 1)))
    const items = plan.items
    const newIndex = runMergedImportMutation(() =>
      insertRangeAtPage(items, target, plan.totalPages, {
        name: `文件${items.length + 1}`,
        extra: { source: 'manual' },
        reuseBoundary: true,
      }),
    )
    if (newIndex < 0) return
    selectedMergedImportIndex.value = newIndex
    previewPage.value = target
    truePreview.value = null
    refreshPreview()
  }

  function splitMergedImportFromCurrentPage() {
    splitMergedImportAtPage(previewPage.value)
  }

  function splitMergedImportFromPreviousPage() {
    if (previewPage.value <= 1) return
    splitMergedImportAtPage(previewPage.value - 1)
  }

  function mergeCurrentMergedImportRangeIntoPrevious() {
    const plan = mergedImportPlan.value
    if (!plan?.items?.length) return
    let index = currentMergedImportRangeIndex.value
    if (index < 0) index = selectedMergedImportIndex.value
    if (index <= 0 || index >= plan.items.length) {
      ElMessage.info('当前已是第一个页段，无法合并到上段')
      return
    }
    runMergedImportMutation(() => {
      const current = plan.items[index]
      const previous = plan.items[index - 1]
      previous.pageEnd = Math.max(Number(previous.pageEnd || 0), Number(current.pageEnd || 0))
      previous.source = 'manual'
      plan.items.splice(index, 1)
    })
    selectedMergedImportIndex.value = index - 1
    truePreview.value = null
    safeRefreshPreview()
  }

  // 计划表格内直接修改起始页/结束页后，同样走智能无缝调整
  function onMergedRangeStartChanged(index, value) {
    const items = mergedImportPlan.value?.items
    if (!items || index < 0) return
    const newIndex = smartSetRangeStart(items, index, value, mergedImportPlan.value.totalPages)
    selectedMergedImportIndex.value = newIndex
  }

  function onMergedRangeEndChanged(index, value) {
    const items = mergedImportPlan.value?.items
    if (!items || index < 0) return
    const newIndex = smartSetRangeEnd(items, index, value, mergedImportPlan.value.totalPages)
    selectedMergedImportIndex.value = newIndex
  }

  function addMergedImportRange() {
    if (!mergedImportPlan.value) return
    const items = mergedImportPlan.value.items
    runMergedImportMutation(() => {
      items.push(buildRangeAfter(items, items.length - 1, previewMaxPage.value, { extra: { source: 'manual' } }))
    })
    selectMergedImportRange(items[items.length - 1])
  }

  function insertMergedImportRangeAfter(index) {
    if (!mergedImportPlan.value) return
    const items = mergedImportPlan.value.items
    const item = runMergedImportMutation(() =>
      insertRangeAfter(items, index, previewMaxPage.value, { extra: { source: 'manual' } }),
    )
    selectMergedImportRange(item)
  }

  function removeMergedImportRange(index) {
    if (!mergedImportPlan.value) return
    const items = mergedImportPlan.value.items
    if (index < 0 || index >= items.length) return
    runMergedImportMutation(() => {
      const [removed] = items.splice(index, 1)
      // 删除页段后不留页面空洞：删的是首页段则并入下一段，否则并入上一个页段
      if (removed && items.length) {
        if (index === 0) {
          items[0].pageStart = Math.min(Number(items[0].pageStart || 1), Number(removed.pageStart || 1))
        } else {
          items[index - 1].pageEnd = Math.max(Number(items[index - 1].pageEnd || 0), Number(removed.pageEnd || 0))
        }
      }
    })
    selectedMergedImportIndex.value = Math.min(selectedMergedImportIndex.value, Math.max(0, items.length - 1))
  }

  function sortMergedImportItems({ prop, order }) {
    if (detectingMergedImport.value || splittingMergedImport.value) return
    if (!mergedImportPlan.value || !prop || !order) return
    const selected = selectedMergedImportRange.value
    mergedImportPlan.value.items = sortByNatural(
      mergedImportPlan.value.items,
      (row, index) => mergedImportSortValue(row, prop, index),
      order,
    )
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
    splitMergedImportFromPreviousPage,
    mergeCurrentMergedImportRangeIntoPrevious,
    undoMergedImportEdit,
    redoMergedImportEdit,
    onMergedRangeStartChanged,
    onMergedRangeEndChanged,
    addMergedImportRange,
    insertMergedImportRangeAfter,
    removeMergedImportRange,
    sortMergedImportItems,
    mergedImportSortValue,
    mergedImportRangePageCount: pageCount,
    mergedImportSourceType,
    mergedImportSourceText,
    headerFooterDetectionZoneMm,
    sourcePageRangeKey,
    hasSplitWarning,
  }
}
