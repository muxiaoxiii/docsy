import { marked } from 'marked'
import { copyPlainText, copyRichText } from '../../../shared/clipboard.js'

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function stripHtml(value) {
  return String(value)
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<[^>]*>/g, '')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
}

function inlineToPlain(tokens) {
  let out = ''
  for (const token of tokens || []) {
    switch (token.type) {
      case 'text':
      case 'escape':
        out += token.tokens ? inlineToPlain(token.tokens) : token.text || ''
        break
      case 'strong':
      case 'em':
      case 'del':
        out += inlineToPlain(token.tokens)
        break
      case 'codespan':
        out += token.text || ''
        break
      case 'link': {
        const label = inlineToPlain(token.tokens)
        out += label
        if (token.href && !token.href.startsWith('#') && token.href !== label) out += ` (${token.href})`
        break
      }
      case 'image':
        out += token.text ? `[图片：${token.text}]` : '[图片]'
        break
      case 'br':
        out += '\n'
        break
      case 'html':
        out += stripHtml(token.text)
        break
      default:
        out += token.tokens ? inlineToPlain(token.tokens) : token.text || ''
    }
  }
  return out
}

function listToPlain(list, depth) {
  const lines = []
  const indent = '  '.repeat(depth)
  const start = Number(list.start || 1)
  list.items?.forEach((item, index) => {
    const marker = item.task ? (item.checked ? '☑ ' : '☐ ') : list.ordered ? `${start + index}. ` : '• '
    const body = tokensToPlain(item.tokens || [], depth + 1).trim()
    const bodyLines = body ? body.split('\n') : ['']
    lines.push(`${indent}${marker}${bodyLines[0] || ''}`)
    for (const line of bodyLines.slice(1)) lines.push(`${indent}  ${line}`)
  })
  return lines.join('\n')
}

function tableToPlain(table) {
  const headers = (table.header || []).map((cell) => inlineToPlain(cell.tokens).trim())
  const rows = (table.rows || []).map((row) =>
    (row || [])
      .map((cell, index) => {
        const value = inlineToPlain(cell.tokens).trim()
        const label = headers[index]
        return label ? `${label}：${value}` : value
      })
      .filter(Boolean)
      .join('；'),
  )
  return rows.filter(Boolean).join('\n')
}

function cleanMathForPlainText(text) {
  return String(text)
    .replace(/\$\$([\s\S]*?)\$\$/g, (_, body) => body.trim())
    .replace(/\\\[([\s\S]*?)\\\]/g, (_, body) => body.trim())
    .replace(/\\\(([\s\S]*?)\\\)/g, (_, body) => body.trim())
    .replace(/\$([^$\n]{1,200})\$/g, (match, body) => (/[\\^_=]/.test(body) ? body.trim() : match))
}

function tokensToPlain(tokens, depth = 0) {
  let out = ''
  for (const token of tokens || []) {
    switch (token.type) {
      case 'heading':
        out += `${inlineToPlain(token.tokens)}\n\n`
        break
      case 'paragraph':
        out += `${inlineToPlain(token.tokens)}\n\n`
        break
      case 'text':
        out += `${token.tokens ? inlineToPlain(token.tokens) : token.text || ''}\n\n`
        break
      case 'code':
        out += `${String(token.text || '').replace(/\n$/, '')}\n\n`
        break
      case 'blockquote':
        out += `${tokensToPlain(token.tokens, depth).trim()}\n\n`
        break
      case 'list':
        out += `${listToPlain(token, depth)}\n`
        break
      case 'table':
        out += `${tableToPlain(token)}\n\n`
        break
      case 'hr':
        out += '────────\n\n'
        break
      case 'space':
        out += '\n'
        break
      case 'html':
        out += `${stripHtml(token.text)}\n\n`
        break
      default:
        if (token.tokens) out += `${tokensToPlain(token.tokens, depth)}`
        else if (token.text) out += `${token.text}\n\n`
    }
  }
  return out
}

function styleHtml(html) {
  return html
    .replace(
      /<table>/g,
      '<table border="1" cellspacing="0" cellpadding="6" style="border-collapse:collapse;width:100%;margin:8px 0;font-size:13px;">',
    )
    .replace(
      /<th>/g,
      '<th style="border:1px solid #d8d2c7;padding:5px 8px;background:#f5efe5;text-align:left;font-weight:600;">',
    )
    .replace(/<td>/g, '<td style="border:1px solid #d8d2c7;padding:5px 8px;vertical-align:top;">')
    .replace(/<pre>/g, '<pre style="margin:8px 0;padding:10px 12px;background:#f5efe5;border-radius:4px;white-space:pre-wrap;font-family:ui-monospace,Consolas,monospace;font-size:12px;">')
    .replace(
      /<blockquote>/g,
      '<blockquote style="margin:8px 0;padding:2px 0 2px 12px;border-left:3px solid #cfc1b0;color:#46534e;">',
    )
}

export function markdownToClipboardPayload(markdown) {
  const source = String(markdown || '')
  const renderer = new marked.Renderer()
  // Raw HTML is displayed as text instead of being pasted as executable markup.
  renderer.html = ({ text }) => escapeHtml(text)
  // Local image paths would be broken in Foxmail/WeChat; keep a readable placeholder.
  renderer.image = ({ text }) => (text ? `[图片：${escapeHtml(text)}]` : '[图片]')

  const rendered = marked.parse(source, { gfm: true, breaks: false, renderer })
  const body = styleHtml(String(rendered || ''))
  const html = `<div style="font-family:'Microsoft YaHei','PingFang SC',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;font-size:14px;line-height:1.65;color:#1f2e2a;">${body}</div>`
  const text = cleanMathForPlainText(tokensToPlain(marked.lexer(source, { gfm: true })))
    .replace(/\r\n?/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()

  return { html, text }
}

export function decodeMarkdownBytes(bytes, encoding = 'auto') {
  const data = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes || [])
  let label = encoding || 'auto'
  if (label === 'auto') {
    if (data[0] === 0xff && data[1] === 0xfe) label = 'utf-16le'
    else if (data[0] === 0xfe && data[1] === 0xff) label = 'utf-16be'
    else label = 'utf-8'
  }
  try {
    return new globalThis.TextDecoder(label, { fatal: false }).decode(data).replace(/^\uFEFF/, '')
  } catch {
    return new globalThis.TextDecoder('utf-8', { fatal: false }).decode(data).replace(/^\uFEFF/, '')
  }
}

export async function copyMarkdownContent(markdown, mode = 'rich') {
  const payload = markdownToClipboardPayload(markdown)
  if (mode === 'plain') {
    await copyPlainText(payload.text)
    return payload
  }
  await copyRichText(payload.html, payload.text)
  return payload
}
