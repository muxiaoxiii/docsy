/**
 * Manual loading animation trigger.
 * Uses the same CustomEvent pipeline as tauriBridge's auto-start/finish,
 * so the DocletWorkingPet animation works for any long-running operation,
 * not just Tauri IPC calls.
 *
 * Usage:
 *   import { showLoading, hideLoading } from '../core/loading.js'
 *   const id = showLoading('正在导出数据…')
 *   try { await doWork() } finally { hideLoading(id) }
 */

let manualSeq = 0

function nextId() {
  manualSeq += 1
  return `_manual:${manualSeq}:${Date.now()}`
}

export function showLoading(label = 'Doclet 正在处理…') {
  if (typeof window === 'undefined') return ''
  const id = nextId()
  window.dispatchEvent(
    new CustomEvent('docsy-operation-start', {
      detail: { id, command: '_manual', label },
    }),
  )
  return id
}

export function hideLoading(id) {
  if (!id || typeof window === 'undefined') return
  window.dispatchEvent(
    new CustomEvent('docsy-operation-finish', {
      detail: { id, command: '_manual' },
    }),
  )
}
