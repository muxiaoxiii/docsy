/**
 * Composable for batch fill state and logic.
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open, save } from '@tauri-apps/plugin-dialog'
import { fileName, parentDir, stripExtension } from '../../../core/filePath.js'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { ensureExtension } from './fieldRowUtils.js'

export function useBatchFill(templatePath, templateManifest, normalizeValues, normalizeStructureOverrides, itemSeparatorSetting, loadTemplateHistoryRuns) {
  const batchProcessing = ref(false)
  const batchSaveVisible = ref(false)
  const batchSaveRows = ref([])
  const batchSaveSelected = ref([])

  async function handleBatchCommand(command) {
    if (command === 'export') {
      await exportBatchTemplate()
    } else if (command === 'import') {
      await importAndBatchRender()
    }
  }

  async function exportBatchTemplate() {
    if (!templatePath.value || !templateManifest.value) return
    const defaultName = `${stripExtension(fileName(templatePath.value), /\.docsytpl$/i)}-批量填写模板.xlsx`
    const outputPath = await save({
      defaultPath: `${parentDir(templatePath.value)}/${defaultName}`,
      filters: [{ name: 'Excel 文件', extensions: ['xlsx'] }],
    })
    if (!outputPath) return

    batchProcessing.value = true
    const result = await tauriCallSafe('export_template_fields_xlsx', {
      templatePath: templatePath.value,
      outputPath: ensureExtension(outputPath, 'xlsx'),
      defaultValues: normalizeValues(),
    })
    batchProcessing.value = false

    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '导出失败'))
      return
    }
    ElMessage.success('字段表已导出')
    ElMessage.info('若模板有填写历史，第 3 行为最近一次填写示例（标"否"不会生成数据）；需要可复制一行后填写')
    const openResult = await openPath(result.data)
    if (!openResult.ok) {
      ElMessage.warning('字段表已导出但无法自动打开，请到保存目录查看')
    }
  }

  async function importAndBatchRender() {
    if (!templatePath.value || !templateManifest.value) return

    const selected = await open({
      multiple: false,
      filters: [{ name: 'Excel 文件', extensions: ['xlsx'] }],
    })
    if (!selected) return

    const xlsxPath = selected

    batchProcessing.value = true
    const validation = await tauriCallSafe('validate_batch_import', {
      templatePath: templatePath.value,
      xlsxPath,
    })

    if (!validation.ok) {
      batchProcessing.value = false
      ElMessage.error(userFacingError(validation.error, '校验失败'))
      return
    }

    const v = validation.data
    const { proceed, skipRows } = await showValidationDialog(v)
    if (!proceed) {
      batchProcessing.value = false
      return
    }

    const outputDir = await open({
      directory: true,
      multiple: false,
      defaultPath: parentDir(templatePath.value),
    })
    if (!outputDir) {
      batchProcessing.value = false
      return
    }

    const result = await tauriCallSafe('batch_render_from_xlsx', {
      templatePath: templatePath.value,
      xlsxPath,
      outputDir,
      namePattern: '',
      skipRows,
      structureOverrides: normalizeStructureOverrides(),
      itemSeparator: itemSeparatorSetting.value || '、',
    })
    batchProcessing.value = false

    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '批量生成失败'))
      return
    }

    const r = result.data
    const outputDirText = typeof outputDir === 'string' ? outputDir : ''
    const lines = [`批量填写完成：成功 ${r.success} 份` + (r.failed > 0 ? `，失败 ${r.failed} 份` : '')]
    if (outputDirText) lines.push(`输出目录：${outputDirText}`)
    if (r.rows?.length) lines.push(`本次生成了 ${r.rows.length} 行填写数据，可保存到模板填写历史`)

    let saveData = false
    try {
      await ElMessageBox.confirm(lines.join('\n'), '批量填写完成', {
        confirmButtonText: outputDirText ? '打开输出目录' : '完成',
        cancelButtonText: r.rows?.length ? '保存数据' : '',
        type: 'success',
        customStyle: { whiteSpace: 'pre-line' },
      })
      if (outputDirText) {
        const openResult = await openPath(outputDirText)
        if (!openResult.ok) {
          ElMessage.warning('无法打开输出目录，请手动查看')
        }
      }
    } catch {
      saveData = true
    }
    if (saveData && r.rows?.length) {
      openBatchSaveDialog(r.rows)
    }
  }

  function openBatchSaveDialog(rows) {
    batchSaveRows.value = (rows || []).map((row, index) => ({
      key: `${index}`,
      outputPath: row.outputPath || '',
      values: row.values || {},
      selected: true,
    }))
    batchSaveSelected.value = []
    batchSaveVisible.value = true
  }

  function toggleBatchSaveAll(value) {
    if (value) {
      batchSaveSelected.value = batchSaveRows.value.map((row) => row.key)
    } else {
      batchSaveSelected.value = []
    }
  }

  function invertBatchSaveSelection() {
    const selected = new Set(batchSaveSelected.value)
    batchSaveSelected.value = batchSaveRows.value
      .map((row) => row.key)
      .filter((key) => !selected.has(key))
  }

  function toggleBatchSaveRow(key) {
    const idx = batchSaveSelected.value.indexOf(key)
    if (idx >= 0) {
      batchSaveSelected.value = batchSaveSelected.value.filter((k) => k !== key)
    } else {
      batchSaveSelected.value = [...batchSaveSelected.value, key]
    }
  }

  function batchSaveRowSummary(row) {
    const values = row.values || {}
    const parts = Object.values(values)
      .map((value) => {
        if (Array.isArray(value)) return value.map((item) => (item?.text ?? item?.name ?? '')).filter(Boolean).join('、')
        if (value && typeof value === 'object') return value.text ?? value.name ?? ''
        return String(value ?? '')
      })
      .filter(Boolean)
      .slice(0, 5)
    return parts.length ? parts.join(' | ') : '（空）'
  }

  async function submitBatchSave() {
    const rows = batchSaveRows.value.filter((row) => batchSaveSelected.value.includes(row.key))
    if (!rows.length) {
      ElMessage.warning('请至少选择一行')
      return
    }
    const payload = rows.map((row) => ({
      templatePath: templatePath.value,
      outputPath: row.outputPath,
      values: row.values,
    }))
    const result = await tauriCallSafe('save_batch_history_rows', { rows: payload })
    if (result.ok) {
      ElMessage.success(`已保存 ${result.data} 行填写记录到模板历史`)
      batchSaveVisible.value = false
      if (loadTemplateHistoryRuns) loadTemplateHistoryRuns()
    } else {
      ElMessage.error(userFacingError(result.error, '保存填写记录失败'))
    }
  }

  async function showValidationDialog(validation) {
    const v = validation
    const lines = []

    if (!v.templateIdMatch) {
      lines.push('⚠️ Excel 文件的模板 ID 与当前模板不一致，将按字段名称匹配导入。')
      lines.push('')
    }

    lines.push(`共 ${v.totalRows} 行数据，${v.validRows} 行有效。`)

    if (v.warnings.length) {
      lines.push('')
      lines.push('警告：')
      for (const w of v.warnings) {
        lines.push(`  · ${w.message}`)
      }
    }

    const errorRowSet = new Set(v.errors.map((e) => e.row))

    if (v.errors.length) {
      lines.push('')
      lines.push('错误：')
      const shown = v.errors.slice(0, 10)
      for (const e of shown) {
        lines.push(`  · 第 ${e.row + 1} 行：${e.message}`)
      }
      if (v.errors.length > 10) {
        lines.push(`  · ...还有 ${v.errors.length - 10} 个错误`)
      }
    }

    if (v.validRows === 0 && v.totalRows > 0) {
      lines.push('')
      lines.push('❌ 没有有效数据行，无法生成。')
    }

    if (v.validRows === 0) {
      try {
        await ElMessageBox.alert(lines.join('\n'), '无有效数据', {
          confirmButtonText: '知道了',
          type: 'warning',
          customStyle: { whiteSpace: 'pre-line' },
        })
      } catch {
        // User dismissed
      }
      return { proceed: false, skipRows: [] }
    }

    const hasErrors = v.errors.length > 0
    const title = hasErrors ? '校验结果（有错误）' : '校验结果'
    const type = hasErrors ? 'warning' : 'info'

    try {
      if (hasErrors) {
        await ElMessageBox.confirm(lines.join('\n'), title, {
          confirmButtonText: '跳过错误行继续',
          cancelButtonText: '取消',
          type,
          customStyle: { whiteSpace: 'pre-line' },
        })
      } else {
        await ElMessageBox.confirm(lines.join('\n'), title, {
          confirmButtonText: '开始生成',
          cancelButtonText: '取消',
          type,
          customStyle: { whiteSpace: 'pre-line' },
        })
      }
      return { proceed: true, skipRows: Array.from(errorRowSet) }
    } catch {
      return { proceed: false, skipRows: [] }
    }
  }

  return {
    batchProcessing,
    batchSaveVisible,
    batchSaveRows,
    batchSaveSelected,
    handleBatchCommand,
    toggleBatchSaveAll,
    invertBatchSaveSelection,
    toggleBatchSaveRow,
    batchSaveRowSummary,
    submitBatchSave,
  }
}
