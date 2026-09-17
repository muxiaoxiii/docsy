import { computed, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElMessageBox } from 'element-plus'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import {
  buildRangeAfter,
  insertRangeAfter,
  insertRangeAtPage,
  pageCount,
  smartSetRangeEnd,
  smartSetRangeStart,
} from '../../../core/pdfUtils.js'
import { createEvidenceFile, sortByNatural } from './useEvidencePdfSession.js'
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
  const mergedImportUndoStack = ref([])
  const mergedImportRedoStack = ref([])
  /** 待处理的后续合并 PDF（多选导入时的队列，逐份检测/拆分）。 */
  const pendingMergedImportPaths = ref([])
  const pendingMergedImportCount = computed(() => pendingMergedImportPaths.value.length)
  const pendingMergedImportNextName = computed(
    () => (pendingMergedImportPaths.value[0] ? fileName(pendingMergedImportPaths.value[0]) : ''),
  )
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
    if (detectingMergedImport.value || splittingMergedImport.value) return
    const before = mergedImportSnapshot()
    const beforeItems = JSON.stringify(before.items)
    const result = mutate()
    if (JSON.stringify(mergedImportPlan.value?.items || []) !== beforeItems) {
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
    if (detectingMergedImport.value || splittingMergedImport.value) return false
    const snapshot = mergedImportUndoStack.value.pop()
    if (!snapshot || !mergedImportPlan.value) return false
    mergedImportRedoStack.value.push(mergedImportSnapshot())
    return restoreMergedImportSnapshot(snapshot)
  }

  function redoMergedImportEdit() {
    if (detectingMergedImport.value || splittingMergedImport.value) return false
    const snapshot = mergedImportRedoStack.value.pop()
    if (!snapshot || !mergedImportPlan.value) return false
    mergedImportUndoStack.value.push(mergedImportSnapshot())
    return restoreMergedImportSnapshot(snapshot)
  }

  async function loadOneMergedImportPath(input) {
    const outputDir = defaultMergedImportOutputDir(input)
    const countResult = await tauriCallSafe('get_pdf_page_count', { input })
    if (!countResult.ok || Number(countResult.data) < 1) {
      ElMessage.error(countResult.error || '无法读取 PDF 页数，请检查文件后重新导入')
      return false
    }
    const totalPages = Math.max(1, Number(countResult.data) || 1)
    mergedImportPlan.value = buildManualMergedImportPlan(input, outputDir, totalPages)
    overlayFiles.value = []
    resetMergedImportHistory()
    selectedMergedImportIndex.value = 0
    previewPage.value = 1
    truePreview.value = null
    safeRefreshPreview()
    return true
  }

  function enqueuePendingMergedImportPaths(paths) {
    const existing = new Set(pendingMergedImportPaths.value)
    for (const path of paths) {
      if (!existing.has(path)) {
        pendingMergedImportPaths.value.push(path)
        existing.add(path)
      }
    }
  }

  function clearPendingMergedImport() {
    pendingMergedImportPaths.value = []
  }

  /** 拆分成功后：若有队列则自动载入下一份，否则保持空计划。 */
  async function advancePendingMergedImportIfAny() {
    if (!pendingMergedImportPaths.value.length) return false
    const next = pendingMergedImportPaths.value.shift()
    const ok = await loadOneMergedImportPath(next)
    if (ok) {
      const left = pendingMergedImportPaths.value.length
      ElMessage.success(
        left > 0
          ? `已载入下一份合并 PDF：${fileName(next)}（队列中还有 ${left} 份）`
          : `已载入下一份合并 PDF：${fileName(next)}`,
      )
      return true
    }
    return false
  }

  async function importMergedPdfAsEvidence(providedPaths) {
    if (importingMergedPdf.value || detectingMergedImport.value || splittingMergedImport.value) return
    importingMergedPdf.value = true
    let knownTotalPages = 1
    let input = ''
    let outputDir = ''
    try {
      const selected = Array.isArray(providedPaths)
        ? providedPaths
        : await open({
            multiple: true,
            filters: [{ name: 'PDF', extensions: ['pdf'] }],
          })
      if (!selected) return
      const paths = (Array.isArray(selected) ? selected : [selected]).filter(Boolean)
      if (!paths.length) return

      // 已有进行中的拆分方案：多选只入队，不打断当前核对。
      if (mergedImportPlan.value) {
        enqueuePendingMergedImportPaths(paths)
        ElMessage.success(
          `已加入待拆分队列 ${paths.length} 份，当前方案拆分完成后自动载入下一份`,
        )
        return
      }

      const [first, ...rest] = paths
      input = first
      outputDir = defaultMergedImportOutputDir(input)
      const loaded = await loadOneMergedImportPath(input)
      if (!loaded) {
        if (rest.length) enqueuePendingMergedImportPaths(rest)
        return
      }
      if (rest.length) {
        enqueuePendingMergedImportPaths(rest)
        ElMessage.success(
          `PDF 已导入；队列中还有 ${rest.length} 份，本份拆分完成后自动载入下一份`,
        )
      } else {
        ElMessage.success('PDF 已导入，点击“检测证据页段”生成拆分建议；检测耗时取决于文件大小和页数')
      }
    } catch (err) {
      if (input) {
        mergedImportPlan.value = buildManualMergedImportPlan(input, outputDir, knownTotalPages, [
          `导入流程中断：${String(err?.message || err || '未知错误')}`,
          '已生成一个覆盖全文的手动页段',
        ])
        resetMergedImportHistory()
        selectedMergedImportIndex.value = 0
        previewPage.value = 1
        truePreview.value = null
        safeRefreshPreview()
        ElMessage.warning('导入中断，已进入手动拆分页段确认')
      }
    } finally {
      importingMergedPdf.value = false
    }
  }

  async function detectMergedImportPlan() {
    const plan = mergedImportPlan.value
    if (!plan || detectingMergedImport.value || splittingMergedImport.value || importingMergedPdf.value) return
    detectingMergedImport.value = true
    for (const candidate of plan.cleanupCandidates || []) candidate.selected = false
    try {
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
        plan.warnings = [inspect.error || '合并 PDF 页段检测失败，已保留当前手动页段']
        ElMessage.warning('页段检测失败，当前手动页段保持不变')
        return
      }
      const items = (inspect.data.items || [])
        .filter((item) => Number(item.pageStart) > 0 && Number(item.pageEnd) >= Number(item.pageStart))
        .map((item, index) => detectedMergedImportItem(item, plan.inputPath, index))
      if (!items.length) {
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

  /** 处理前检测电子签名：有则要求用户确认（压平/重写可能使签章证书失效）。 */
  async function confirmPdfSignatureRisk(inputPath) {
    const result = await tauriCallSafe('detect_pdf_signatures', { input: inputPath })
    if (!result.ok) return true // 检测失败不阻断，由后续步骤暴露错误
    if (!result.data?.hasSignatures) return true
    try {
      await ElMessageBox.confirm(
        `「${fileName(inputPath)}」${result.data.summary}。继续处理可能会压平签章外观或使数字签名失效（可见图像通常仍保留）。是否继续？`,
        '检测到电子签章',
        {
          type: 'warning',
          confirmButtonText: '继续处理',
          cancelButtonText: '取消',
        },
      )
      return true
    } catch {
      return false
    }
  }

  async function executeMergedImportPlan() {
    if (!mergedImportPlan.value || splittingMergedImport.value || detectingMergedImport.value || importingMergedPdf.value) return
    splittingMergedImport.value = true
    const items = normalizedMergedImportItems()
    try {
      if (!items.length) {
        ElMessage.warning('没有可拆分的页段')
        return
      }
      const invalid = items.find((item) => !item.name || item.pageStart < 1 || item.pageEnd < item.pageStart)
      if (invalid) {
        ElMessage.warning('请先修正文件名或页码范围')
        return
      }
      const blockingWarnings = splitRangeWarnings(items, mergedImportPlan.value.totalPages)
      if (blockingWarnings.length) {
        ElMessage.warning(`请先核对页段：${blockingWarnings[0]}`)
        return
      }
      const signatureOk = await confirmPdfSignatureRisk(mergedImportPlan.value.inputPath)
      if (!signatureOk) return
      if (overlayFiles.value.length) {
        try {
          await ElMessageBox.confirm('确认拆分后会替换当前证据列表。', '替换当前列表', {
            confirmButtonText: '替换并拆分',
            cancelButtonText: '取消',
            type: 'warning',
          })
        } catch {
          return
        }
      }

      const headerScanMm = headerFooterDetectionZoneMm(cleanupHeaderHeightMm.value)
      const footerScanMm = headerFooterDetectionZoneMm(cleanupFooterHeightMm.value)
      const split = await tauriCallSafe('split_merged_evidence_pdf', {
        args: {
          inputPath: mergedImportPlan.value.inputPath,
          outputDir: mergedImportPlan.value.outputDir,
          items,
          removeBlankPages: Boolean(removeBlankPages?.value),
          headerZoneMm: headerScanMm,
          footerZoneMm: footerScanMm,
          cleanupTargets: (mergedImportPlan.value.cleanupCandidates || [])
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
        },
      })
      if (!split.ok) {
        ElMessage.error(userFacingError(split.error, '拆分合并 PDF 失败'))
        return
      }

      const outputs = split.data.outputs || []
      if (!outputs.length) {
        ElMessage.warning('没有生成可导入的拆分文件')
        return
      }
      const rawItemByRange = new Map(
        (mergedImportPlan.value.items || []).map((item) => [sourcePageRangeKey(item), item]),
      )
      overlayFiles.value = outputs.map((output) => {
        const sourceItem = rawItemByRange.get(sourcePageRangeKey(output)) || {}
        const pages = Math.max(0, Number(output.pageEnd || 0) - Number(output.pageStart || 0) + 1 - Number(output.removedBlankPages || 0))
        const needsReview =
          sourceItem.source === 'fallback' ||
          sourceItem.source === 'manual' ||
          hasSplitWarning(split.data.warnings || [], output)
        return {
          ...createEvidenceFile(output.outputPath),
          header: sourceItem.name || output.name,
          pages,
          sourcePageStart: Number(output.pageStart || sourceItem.pageStart || 0),
          sourcePageEnd: Number(output.pageEnd || sourceItem.pageEnd || 0),
          sourceDetectionSource: sourceItem.source || 'unknown',
          detectionSummary: `来自合并 PDF 第 ${output.pageStart}-${output.pageEnd} 页`,
          statusText: needsReview ? '需核对' : '就绪',
          statusType: needsReview ? 'warning' : 'success',
        }
      })
      overlayOutputDir.value = mergedImportPlan.value.outputDir
      splitReplacementOutputDir.value = ''
      selectedOverlayIndex.value = 0
      previewPage.value = 1
      applyWorkflowDefaults()

      const failed = split.data.failed?.length || 0
      const warnings = split.data.warnings || []
      const removedBlanks = (outputs || []).reduce((sum, output) => sum + Number(output.removedBlankPages || 0), 0)
      const blankSuffix = removedBlanks > 0 ? `（已删除 ${removedBlanks} 个空白页）` : ''
      const cleanupSuffix = Number(split.data.removedHeaders || 0) + Number(split.data.removedFooters || 0) > 0
        ? `；清除源副本页眉 ${split.data.removedHeaders || 0} 处、页脚 ${split.data.removedFooters || 0} 处` : ''

      if (failed > 0) {
        const failedDetails = (split.data.failed || [])
          .map((f) => `${f.name || f.outputPath || '?'}：${f.message || '未知错误'}`)
          .join('；')
        if (mergedImportPlan.value) {
          mergedImportPlan.value.warnings = [
            `拆分存在 ${failed} 项失败：${failedDetails}`,
            ...(warnings || []),
          ]
        }
        if (pendingMergedImportPaths.value.length) {
          try {
            await ElMessageBox.confirm(
              `本份已生成 ${outputs.length} 个证据，失败 ${failed} 个（${failedDetails}）。队列中还有 ${pendingMergedImportPaths.value.length} 份。是否跳过失败项并继续下一份？`,
              '拆分部分失败',
              {
                type: 'warning',
                confirmButtonText: '跳过并继续',
                cancelButtonText: '留在本份重试',
              },
            )
            mergedImportPlan.value = null
            resetMergedImportHistory()
            await advancePendingMergedImportIfAny()
            ElMessage.warning(`已跳过失败项；本份成功输出 ${outputs.length} 个证据${blankSuffix}`)
          } catch {
            ElMessage.warning(`已生成 ${outputs.length} 个证据，失败 ${failed} 个；已留在本份供检查重试`)
          }
        } else {
          ElMessage.warning(`已生成 ${outputs.length} 个证据，失败 ${failed} 个（${failedDetails}）${blankSuffix}；计划已保留供重试`)
        }
      } else {
        mergedImportPlan.value = null
        resetMergedImportHistory()
        if (warnings.length) {
          ElMessage.warning(`已生成 ${outputs.length} 个证据${blankSuffix}${cleanupSuffix}；提示：${warnings[0]}`)
        } else {
          ElMessage.success(`已生成 ${outputs.length} 个证据${blankSuffix}${cleanupSuffix}`)
        }
        // 队列：拆完自动进入下一份（仅在无失败时推进，避免失败项被静默跳过）
        await advancePendingMergedImportIfAny()
      }
      refreshPreview()
    } finally {
      splittingMergedImport.value = false
    }
  }

  async function selectMergedImportOutputDir() {
    if (detectingMergedImport.value || splittingMergedImport.value) return
    if (!mergedImportPlan.value) return
    const selected = await open({ directory: true })
    if (!selected) return
    mergedImportPlan.value.outputDir = selected
  }

  function cancelMergedImportPlan() {
    if (detectingMergedImport.value || splittingMergedImport.value || importingMergedPdf.value) return
    mergedImportPlan.value = null
    resetMergedImportHistory()
    selectedMergedImportIndex.value = 0
    previewPage.value = 1
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
      totalPages: Math.max(1, Number(total || 1)),
      pagesAnalyzed: 0,
      headerPages: 0,
      pageNumberFooterPages: 0,
      warnings,
      items: [defaultMergedImportRange(inputPath, total)],
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
    if (detectingMergedImport.value || splittingMergedImport.value) return
    const items = mergedImportPlan.value?.items
    if (!items || index < 0) return
    let newIndex = index
    runMergedImportMutation(() => {
      newIndex = smartSetRangeStart(items, index, value, mergedImportPlan.value.totalPages)
    })
    selectedMergedImportIndex.value = newIndex
    truePreview.value = null
    safeRefreshPreview()
  }

  function onMergedRangeEndChanged(index, value) {
    if (detectingMergedImport.value || splittingMergedImport.value) return
    const items = mergedImportPlan.value?.items
    if (!items || index < 0) return
    let newIndex = index
    runMergedImportMutation(() => {
      newIndex = smartSetRangeEnd(items, index, value, mergedImportPlan.value.totalPages)
    })
    selectedMergedImportIndex.value = newIndex
    truePreview.value = null
    safeRefreshPreview()
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
    mergedImportWarnings,
    pendingMergedImportPaths,
    pendingMergedImportCount,
    pendingMergedImportNextName,
    clearPendingMergedImport,
    advancePendingMergedImportIfAny,
    confirmPdfSignatureRisk,
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
