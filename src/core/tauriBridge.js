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

function emitOperationEvent(type, command, operationId) {
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

export function openPath(path) {
  return tauriCallSafe('open_path', { path })
}

export function getPdfPageCount(input) {
  return tauriCallSafe('get_pdf_page_count', { input })
}
