import { stat } from '@tauri-apps/plugin-fs'
import { tauriCallSafe } from '../../core/tauriBridge.js'

// Share rendered pages between the main preview and thumbnails. Bound the
// retained strings conservatively as UTF-16; do not keep whole PDF files here.
export function createPdfPreviewCache({ statFile, renderPage, maxBytes = 24 * 1024 * 1024, maxEntries = 16 }) {
  const entries = new Map(),
    pending = new Map(),
    epochs = new Map()
  let bytes = 0
  function invalidate(path) {
    epochs.set(path, (epochs.get(path) || 0) + 1)
    for (const [key, entry] of entries) {
      if (entry.path !== path) continue
      bytes -= entry.bytes
      entries.delete(key)
    }
  }
  async function fingerprint(path) {
    try {
      const info = await statFile(path)
      if (info.mtime == null || !Number.isFinite(Number(info.size))) return null
      const modified = new Date(info.mtime).getTime()
      if (!Number.isFinite(modified)) return null
      const born = info.birthtime == null ? '' : new Date(info.birthtime).getTime()
      return `${info.size}:${modified}:${Number.isFinite(born) ? born : ''}`
    } catch {
      return null
    }
  }
  async function load({ inputPath, page, dpi, forceRefresh = false }, retry = true) {
    if (forceRefresh) invalidate(inputPath)
    const epoch = epochs.get(inputPath) || 0
    const version = await fingerprint(inputPath)
    const key = JSON.stringify([inputPath, version, epoch, page, dpi])
    const hit = version === null ? null : entries.get(key)
    if (hit) {
      entries.delete(key)
      entries.set(key, hit)
      return hit.result
    }
    if (pending.has(key)) return pending.get(key)
    const request = (async () => {
      const args = { inputPath, page, dpi, ...(forceRefresh ? { forceRefresh: true } : {}) }
      const result = await renderPage(args)
      const after = await fingerprint(inputPath)
      if (version !== null && after !== version) {
        invalidate(inputPath)
        if (retry) return load({ inputPath, page, dpi, forceRefresh }, false)
        throw new Error('文件在预览期间发生变化，请稍后重新预览')
      }
      const size = String(result.imageDataUrl || '').length * 2
      if (version !== null && size > 0 && size <= maxBytes && epoch === (epochs.get(inputPath) || 0)) {
        // Remove obsolete versions of this file instead of retaining stale images.
        for (const [oldKey, entry] of entries) {
          if (entry.path === inputPath && entry.version !== version) {
            entries.delete(oldKey)
            bytes -= entry.bytes
          }
        }
        entries.set(key, { path: inputPath, version, bytes: size, result })
        bytes += size
        while (bytes > maxBytes || entries.size > maxEntries) {
          const oldest = entries.keys().next().value
          bytes -= entries.get(oldest).bytes
          entries.delete(oldest)
        }
      }
      return result
    })()
    pending.set(key, request)
    try {
      return await request
    } finally {
      if (pending.get(key) === request) pending.delete(key)
    }
  }
  return { load, invalidate, fingerprint }
}

export const pdfPreviewCache = createPdfPreviewCache({
  statFile: stat,
  renderPage: async (args) => {
    const result = await tauriCallSafe('render_pdf_preview', { args })
    if (!result.ok) throw new Error(result.error || '后端预览失败')
    return result.data
  },
})
