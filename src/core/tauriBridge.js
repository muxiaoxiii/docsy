import { invoke } from '@tauri-apps/api/core'
import { logError } from '../services/appLogger.js'

const operationLabels = {
  extract_frames: 'Doclet 正在抽取视频帧…',
  render_docx_template: 'Doclet 正在生成 Word…',
  apply_evidence_pdf_rules: 'Doclet 正在处理证据 PDF…',
  build_evidence_group_pdfs: 'Doclet 正在生成证据 PDF…',
  split_merged_evidence_pdf: 'Doclet 正在拆分 PDF…',
  merge_pdfs: 'Doclet 正在合并 PDF…',
  compress_pdf: 'Doclet 正在压缩 PDF…',
  extract_pdf_pages: 'Doclet 正在提取页面…',
  detect_pdf_header_footer: 'Doclet 正在检测页眉页脚…',
  preview_pdf_header_footer: 'Doclet 正在生成真实预览…',
  scan_evidence_folder: 'Doclet 正在扫描证据文件夹…',
  install_external_tool: 'Doclet 正在下载安装工具…',
  install_external_tool_from_package: 'Doclet 正在安装工具包…',
  inspect_docx_template: 'Doclet 正在读取 Word 模板…',
  inspect_merged_evidence_pdf: 'Doclet 正在分析合并 PDF…',
  inspect_docsytpl: 'Doclet 正在打开模板…',
  analyze_image_paddler_folder: 'Doclet 正在分析图片文件夹…',
  render_pdf_preview: 'Doclet 正在生成 PDF 预览…',
  probe_video: 'Doclet 正在读取视频信息…',
  unlock_pdf: 'Doclet 正在解锁 PDF…',
  run_image_paddler: 'Doclet 正在生成文档…',
}

let operationSeq = 0

function nextOperationId(command) {
  operationSeq += 1
  return `${command}:${operationSeq}`
}

export function emitOperationEvent(type, command, operationId) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(
    new CustomEvent(`docsy-operation-${type}`, {
      detail: {
        id: operationId,
        command,
        label: operationLabels[command] || 'Doclet 正在处理…',
      },
    }),
  )
}

export async function tauriCall(command, args = {}) {
  const operationId = nextOperationId(command)
  emitOperationEvent('start', command, operationId)
  try {
    return await invoke(command, args)
  } catch (err) {
    void logError('tauri.bridge', `${command} failed`, { error: err })
    throw err
  } finally {
    emitOperationEvent('finish', command, operationId)
  }
}

// Call Tauri without triggering Doclet animation — for use in batch loops
export async function tauriCallQuiet(command, args = {}) {
  try {
    const result = await invoke(command, args)
    return { ok: true, data: result }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    return { ok: false, error: message }
  }
}

export async function tauriCallSafe(command, args = {}) {
  const operationId = nextOperationId(command)
  emitOperationEvent('start', command, operationId)
  try {
    const result = await invoke(command, args)
    return { ok: true, data: result }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    const details = {
      message,
      stack: err instanceof Error ? err.stack : '',
      raw: err,
    }
    void logError('tauri.bridge', `${command} failed`, details)
    return { ok: false, error: message, details }
  } finally {
    emitOperationEvent('finish', command, operationId)
  }
}

/// 查询 Rust 侧活跃操作列表（用于取消功能）
export async function listActiveOperations() {
  return invoke('list_active_operations')
}

export function openPath(path) {
  return tauriCallSafe('open_path', { path })
}

export function openExternalUrl(url) {
  return tauriCallSafe('open_external_url', { url })
}

export function getPdfPageCount(input) {
  return tauriCallSafe('get_pdf_page_count', { input })
}

export function userFacingError(error, fallback = '操作失败', maxLength = 220) {
  const message = String(error || '').trim()
  if (!message) return fallback

  // Suppress noisy qpdf warnings that don't affect output
  if (/qpdf --json.*WARNING:.*object has offset 0.*handled correctly by qpdf/is.test(message)) {
    return `${fallback}：PDF 结构存在可修复警告，详细信息已写入日志`
  }

  // Detect which external tool caused the failure and prefix with tool name
  const toolMatch = message.match(/(?:执行 |使用 )?(qpdf|poppler|pdftoppm|pdftotext|ffmpeg|ffprobe|Word|WPS|LibreOffice)/i)
  let prefix = ''
  if (toolMatch) {
    const tool = toolMatch[1].toLowerCase()
    const toolNames = {
      qpdf: 'qpdf（PDF 处理引擎）',
      poppler: 'Poppler（PDF 文本检测）',
      pdftoppm: 'Poppler（PDF 渲染）',
      pdftotext: 'Poppler（PDF 文本提取）',
      ffmpeg: 'FFmpeg（视频处理）',
      ffprobe: 'FFmpeg（视频信息读取）',
      word: 'Microsoft Word',
      wps: 'WPS Writer',
      libreoffice: 'LibreOffice',
    }
    prefix = (toolNames[tool] || tool) + '报错：'
  }

  // Extract the most useful part of qpdf error messages
  const qpdfDetail = message.match(/退出码\s*(\d+)[:：]\s*(.+)/)
  if (qpdfDetail) {
    const code = qpdfDetail[1]
    const detail = qpdfDetail[2].trim()
    // Exit code 2 = encrypted, 3 = warnings (ok), others = real errors
    if (code === '2' && !detail.toLowerCase().includes('error')) {
      return `${prefix || fallback}：PDF 文件可能已加密或受保护`
    }
    return `${prefix || fallback}：${detail}`
  }

  // Clean up "context" chains from anyhow — keep the most relevant message
  const compact = message.replace(/\s+/g, ' ')

  if (compact.length <= maxLength) return `${prefix}${compact}`
  return `${prefix}${compact.slice(0, maxLength).trim()}…（详情见日志）`
}

// Re-export manual loading animation API for single-import convenience
export { showLoading, hideLoading } from './loading.js'
