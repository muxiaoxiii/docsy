import { computed } from 'vue'
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
import { formatSplitFileName } from './splitFileName.js'
import { splitRangeWarnings } from './usePdfSplitRanges.js'
import { headerFooterDetectionZoneMm } from './useEvidencePdfDetection.js'

export function useEvidencePdfMergedImport({
  overlayFiles,
  overlayOutputDir,
  _overlayRows,
  importingMergedPdf,
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
  splitCleanupHeader,
  splitCleanupFooter,
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
  detectAllHeaderFooter,
  refreshPreview,
  safeRefreshPreview,
  applyWorkflowDefaults,
  refreshOverlayPageCounts,
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

  async function importMergedPdfAsEvidence() {
    if (importingMergedPdf.value) return
    importingMergedPdf.value = true
    let knownTotalPages = 1
    let input = ''
    let outputDir = ''
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      })
      if (!selected) return
      const paths = Array.isArray(selected) ? selected : [selected]
      if (!paths.length) return
      if (paths.length > 1) {
        await importMergedPdfsForBatch(paths)
        return
      }
      input = paths[0]
      outputDir = defaultMergedImportOutputDir(input)

      const countResult = await tauriCallSafe('get_pdf_page_count', { input })
      const totalPages = countResult.ok ? Number(countResult.data || 0) : 0
      knownTotalPages = Math.max(1, totalPages || 1)
      const headerScanMm = headerFooterDetectionZoneMm(cleanupHeaderHeightMm.value)
      const footerScanMm = headerFooterDetectionZoneMm(cleanupFooterHeightMm.value)
      const inspect = await tauriCallSafe('inspect_merged_evidence_pdf', {
        args: {
          inputPath: input,
          maxPages: knownTotalPages,
          headerZoneMm: headerScanMm,
          footerZoneMm: footerScanMm,
        },
      })
      const detectedItems = inspect.ok ? inspect.data.items || [] : []
      const detectedTotalPages = inspect.ok ? Number(inspect.data.totalPages || 0) : 0
      const detectedPagesAnalyzed = inspect.ok ? Number(inspect.data.pagesAnalyzed || 0) : 0
      const detectedHeaderPages = inspect.ok ? Number(inspect.data.headerPages || 0) : 0
      const detectedPageNumberFooterPages = inspect.ok ? Number(inspect.data.pageNumberFooterPages || 0) : 0
      const warnings = inspect.ok
        ? [...(inspect.data.warnings || [])]
        : [inspect.error || '合并 PDF 页眉分析失败，已保留手动拆分页段']
      const planTotalPages = Math.max(1, detectedTotalPages || totalPages || 1)
      const items = detectedItems
        .filter((item) => Number(item.pageStart) > 0 && Number(item.pageEnd) >= Number(item.pageStart))
        .map((item, index) => {
          const segment = {
            name: String(item.name || '').trim() || defaultMergedImportName(input, index),
            pageStart: Number(item.pageStart),
            pageEnd: Number(item.pageEnd),
            source: item.source || 'unknown',
          }
          segment.existingPageNumberHasTotal = item.hasTotal || false
          segment.existingPageNumberSequenceForm = item.sequenceForm || ''
          return segment
        })
      if (!items.length) {
        items.push(defaultMergedImportRange(input, planTotalPages))
        warnings.push('未识别到可用页眉页段，已生成一个覆盖全文的手动页段')
      }

      mergedImportPlan.value = {
        inputPath: input,
        outputDir,
        totalPages: planTotalPages,
        pagesAnalyzed: detectedPagesAnalyzed,
        headerPages: detectedHeaderPages,
        pageNumberFooterPages: detectedPageNumberFooterPages,
        warnings,
        items,
      }
      selectedMergedImportIndex.value = 0
      previewPage.value = items[0]?.pageStart || 1
      truePreview.value = null
      safeRefreshPreview()
      if (!inspect.ok) {
        ElMessage.warning('自动分析失败，已进入手动拆分页段确认')
      } else if (items.some((item) => item.source === 'manual')) {
        ElMessage.warning('未识别到页眉页段，请手动调整拆分范围')
      } else {
        ElMessage.success(`已识别 ${items.length} 个页段，请核对后确认拆分`)
      }
    } catch (err) {
      mergedImportPlan.value = buildManualMergedImportPlan(input, outputDir, knownTotalPages, [
        `分析流程中断：${String(err?.message || err || '未知错误')}`,
        '已生成一个覆盖全文的手动页段',
      ])
      selectedMergedImportIndex.value = 0
      previewPage.value = 1
      truePreview.value = null
      safeRefreshPreview()
      ElMessage.warning('分析中断，已进入手动拆分页段确认')
    } finally {
      importingMergedPdf.value = false
    }
  }

  async function importMergedPdfsForBatch(paths) {
    importingMergedPdf.value = true
    try {
      mergedImportPlan.value = null
      overlayFiles.value = paths.map((path) => ({
        ...createEvidenceFile(path),
        header: stripPdf(fileName(path)),
        sourceDetectionSource: 'merged_pdf',
        detectionSummary: '作为合并证据 PDF 批量处理',
        statusText: '等待',
        statusType: 'info',
      }))
      overlayOutputDir.value = defaultMergedBatchOutputDir(paths)
      splitReplacementOutputDir.value = overlayOutputDir.value
      selectedOverlayIndex.value = 0
      selectedMergedImportIndex.value = 0
      previewPage.value = 1
      truePreview.value = null
      applyWorkflowDefaults()
      await refreshOverlayPageCounts()
      await detectAllHeaderFooter({ silent: true })
      refreshPreview()
      ElMessage.success(`已导入 ${paths.length} 个合并证据 PDF，可按统一规则批量处理`)
    } finally {
      importingMergedPdf.value = false
    }
  }

  async function executeMergedImportPlan() {
    if (!mergedImportPlan.value || splittingMergedImport.value) return
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

      const split = await tauriCallSafe('split_merged_evidence_pdf', {
        args: {
          inputPath: mergedImportPlan.value.inputPath,
          outputDir: mergedImportPlan.value.outputDir,
          items,
          cleanup: {
            headerEnabled: Boolean(splitCleanupHeader?.value),
            footerEnabled: Boolean(splitCleanupFooter?.value),
            headerHeightMm: cleanupHeaderHeightMm.value,
            footerHeightMm: cleanupFooterHeightMm.value,
          },
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
        const pages = Math.max(0, Number(output.pageEnd || 0) - Number(output.pageStart || 0) + 1)
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
      mergedImportPlan.value = null
      refreshPreview()
      await detectAllHeaderFooter({ silent: true })

      const failed = split.data.failed?.length || 0
      const warnings = split.data.warnings || []
      if (failed) {
        ElMessage.warning(`已生成 ${outputs.length} 个证据，失败 ${failed} 个`)
      } else if (warnings.length) {
        ElMessage.warning(`已生成 ${outputs.length} 个证据，需核对页段提示`)
      } else {
        ElMessage.success(`已生成 ${outputs.length} 个证据`)
      }
    } finally {
      splittingMergedImport.value = false
    }
  }

  async function selectMergedImportOutputDir() {
    if (!mergedImportPlan.value) return
    const selected = await open({ directory: true })
    if (!selected) return
    mergedImportPlan.value.outputDir = selected
  }

  function cancelMergedImportPlan() {
    mergedImportPlan.value = null
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
    const newIndex = smartSetRangeStart(items, index, previewPage.value, mergedImportPlan.value.totalPages)
    selectedMergedImportIndex.value = newIndex
  }

  function setSelectedMergedRangeEnd() {
    const items = mergedImportPlan.value?.items
    if (!items) return
    const index = items.indexOf(selectedMergedImportRange.value)
    if (index < 0) return
    // 智能调整：改结束页后自动与后一段无缝衔接，后段被吃空则自动并入
    const newIndex = smartSetRangeEnd(items, index, previewPage.value, mergedImportPlan.value.totalPages)
    selectedMergedImportIndex.value = newIndex
  }

  // 预览区「添加新页段」：以当前预览页为起始页插入新页段，
  // 结束页与后续页段接续；边界页提出一页，段中页承接段尾，末段到最后一页。
  function addMergedImportRangeFromPage() {
    const plan = mergedImportPlan.value
    if (!plan) return
    const items = plan.items
    const newIndex = insertRangeAtPage(items, previewPage.value, plan.totalPages, {
      name: `文件${items.length + 1}`,
      extra: { source: 'manual' },
    })
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
    items.push(buildRangeAfter(items, items.length - 1, previewMaxPage.value, { extra: { source: 'manual' } }))
    selectMergedImportRange(items[items.length - 1])
  }

  function insertMergedImportRangeAfter(index) {
    if (!mergedImportPlan.value) return
    const items = mergedImportPlan.value.items
    const item = insertRangeAfter(items, index, previewMaxPage.value, { extra: { source: 'manual' } })
    selectMergedImportRange(item)
  }

  function removeMergedImportRange(index) {
    if (!mergedImportPlan.value) return
    const items = mergedImportPlan.value.items
    if (index < 0 || index >= items.length) return
    const [removed] = items.splice(index, 1)
    // 删除页段后不留页面空洞：删的是首页段则并入下一段，否则并入上一个页段
    if (removed && items.length) {
      if (index === 0) {
        items[0].pageStart = Math.min(Number(items[0].pageStart || 1), Number(removed.pageStart || 1))
      } else {
        items[index - 1].pageEnd = Math.max(
          Number(items[index - 1].pageEnd || 0),
          Number(removed.pageEnd || 0),
        )
      }
    }
    selectedMergedImportIndex.value = Math.min(selectedMergedImportIndex.value, Math.max(0, items.length - 1))
  }

  function sortMergedImportItems({ prop, order }) {
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
    selectedMergedImportRange,
    importMergedPdfAsEvidence,
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
