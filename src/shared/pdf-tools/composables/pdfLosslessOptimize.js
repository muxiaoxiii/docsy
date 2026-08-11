/**
 * PDF 无损体积优化：压缩页签的体积对比文案，以及证据导入时
 * 调用后端 optimize_pdf_lossless，有收益用优化副本，否则静默回退原路径。
 */
import { tauriCallSafe } from '../../../core/tauriBridge.js'

/** 人类可读的文件体积（如 38.0MB）。 */
export function formatFileSize(bytes) {
  const size = Number(bytes)
  if (!Number.isFinite(size) || size <= 0) return '0B'
  if (size < 1024) return `${Math.round(size)}B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)}KB`
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)}MB`
  return `${(size / (1024 * 1024 * 1024)).toFixed(2)}GB`
}

/** 体积节省百分比（0-100），输入无效时返回 null。 */
export function savedPercent(inputSize, outputSize) {
  const input = Number(inputSize)
  const output = Number(outputSize)
  if (!Number.isFinite(input) || input <= 0 || !Number.isFinite(output) || output < 0) return null
  return Math.max(0, Math.round((1 - output / input) * 100))
}

/** "38.0MB → 3.4MB(节省 91%)" 形式的对比文案。 */
export function sizeSavingText(inputSize, outputSize) {
  const base = `${formatFileSize(inputSize)} → ${formatFileSize(outputSize)}`
  const percent = savedPercent(inputSize, outputSize)
  return percent == null ? base : `${base}(节省 ${percent}%)`
}

/**
 * 批量任务汇总文案（items 为 { status, name, inputSize, outputSize }）：
 * "压缩完成 4/5：共 190.0MB → 17.5MB(节省 91%)；失败：a.pdf"。
 * 无体积数据（如文档互转）时省略体积部分。
 */
export function batchSummaryText(label, items) {
  const list = Array.isArray(items) ? items : []
  const done = list.filter((item) => item?.status === 'done')
  const failed = list.filter((item) => item?.status === 'failed')
  const inputSize = done.reduce((sum, item) => sum + (Number(item.inputSize) || 0), 0)
  const outputSize = done.reduce((sum, item) => sum + (Number(item.outputSize) || 0), 0)
  let text = `${label} ${done.length}/${list.length}`
  if (done.length && inputSize > 0) text += `：共 ${sizeSavingText(inputSize, outputSize)}`
  if (failed.length) text += `；失败：${failed.map((item) => item.name).join('、')}`
  return text
}

/**
 * 决定导入时使用优化副本还是原件：
 * 仅当后端返回 changed=true 且给出输出路径时用副本，其余一律回退原路径。
 */
export function resolveOptimizedImport(originalPath, result) {
  const data = result?.ok ? result.data : null
  if (data?.changed && data.output_path) {
    return {
      path: data.output_path,
      optimized: true,
      inputSize: Number(data.input_size) || 0,
      outputSize: Number(data.output_size) || 0,
    }
  }
  return { path: originalPath, optimized: false, inputSize: 0, outputSize: 0 }
}

/** 汇总一次批量导入中真正发生优化的文件。 */
export function summarizeOptimizedImports(decisions) {
  const changed = (decisions || []).filter((d) => d.optimized)
  return {
    count: changed.length,
    inputSize: changed.reduce((sum, d) => sum + d.inputSize, 0),
    outputSize: changed.reduce((sum, d) => sum + d.outputSize, 0),
  }
}

/** 逐个无损优化待导入的 PDF；失败或无收益静默回退原路径，绝不阻断导入。 */
export async function optimizeImportsLossless(paths, outputDir = '') {
  const decisions = []
  for (const path of paths) {
    const result = await tauriCallSafe('optimize_pdf_lossless', {
      input: path,
      outputDir: outputDir || null,
    })
    if (!result.ok) {
      console.warn('[pdfLosslessOptimize] 无损优化失败，回退原路径:', path, result.error)
    }
    decisions.push(resolveOptimizedImport(path, result))
  }
  return decisions
}
