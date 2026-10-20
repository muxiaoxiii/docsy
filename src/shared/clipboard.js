/**
 * Clipboard helpers for copying rich text plus a clean plain-text alternative.
 *
 * Targets choose the representation they understand: mail/word processors usually
 * take text/html, chat composers usually take text/plain. Keeping both on the
 * clipboard means one copy action works for Foxmail and WeChat without asking the
 * user which editor they are pasting into.
 */

function fallbackCopyWithSelection(element) {
  if (typeof document === 'undefined') return false
  const host = document.createElement('div')
  host.setAttribute('aria-hidden', 'true')
  host.style.position = 'fixed'
  host.style.left = '-10000px'
  host.style.top = '0'
  host.style.opacity = '0'
  host.style.pointerEvents = 'none'
  host.appendChild(element)
  document.body.appendChild(host)
  const selection = window.getSelection()
  const range = document.createRange()
  range.selectNodeContents(element)
  selection?.removeAllRanges()
  selection?.addRange(range)
  let copied
  try {
    copied = document.execCommand('copy')
  } catch {
    copied = false
  } finally {
    selection?.removeAllRanges()
    host.remove()
  }
  return Boolean(copied)
}

function fallbackCopyHtml(html) {
  const container = document.createElement('div')
  container.contentEditable = 'true'
  container.innerHTML = html
  return fallbackCopyWithSelection(container)
}

function fallbackCopyText(text) {
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  return fallbackCopyWithSelection(textarea)
}

export async function copyRichText(html, plainText) {
  const safePlain = String(plainText || '')
  const safeHtml = String(html || safePlain)

  if (typeof navigator !== 'undefined' && navigator.clipboard?.write && typeof globalThis.ClipboardItem !== 'undefined') {
    try {
      await navigator.clipboard.write([
        new globalThis.ClipboardItem({
          'text/html': new Blob([safeHtml], { type: 'text/html' }),
          'text/plain': new Blob([safePlain], { type: 'text/plain' }),
        }),
      ])
      return
    } catch {
      // WKWebView/WebView2 may expose the API but reject HTML payloads; fall through.
    }
  }

  if (typeof document !== 'undefined' && fallbackCopyHtml(safeHtml)) return
  throw new Error('当前环境不允许写入剪贴板')
}

export async function copyPlainText(text) {
  const safeText = String(text || '')
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(safeText)
      return
    } catch {
      // fall through to the selection-based path
    }
  }

  if (typeof document !== 'undefined' && fallbackCopyText(safeText)) return
  throw new Error('当前环境不允许写入剪贴板')
}
