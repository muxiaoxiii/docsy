import { describe, expect, it } from 'vitest'
import { decodeMarkdownBytes, markdownToClipboardPayload } from './markdownClipboard.js'

describe('Markdown 剪贴板内容', () => {
  it('生成干净的纯文本而不是 Markdown 标记', () => {
    const { text } = markdownToClipboardPayload(`# 标题

这是一段**加粗**和[链接](https://example.com)。

- 项目一
- 项目二

| 姓名 | 金额 |
| --- | --- |
| 张三 | 100 |
`)
    expect(text).toContain('标题')
    expect(text).toContain('这是一段加粗和链接 (https://example.com)。')
    expect(text).toContain('• 项目一')
    expect(text).toContain('姓名：张三；金额：100')
    expect(text).not.toContain('**')
    expect(text).not.toContain('| ---')
  })

  it('生成可用于邮件的 HTML，并转义原始 HTML', () => {
    const { html } = markdownToClipboardPayload('**加粗**\n\n<script>alert(1)</script>')
    expect(html).toContain('<strong>加粗</strong>')
    expect(html).toContain('&lt;script&gt;')
    expect(html).not.toContain('<script>')
  })

  it('表格在纯文本中按表头展开', () => {
    const { text } = markdownToClipboardPayload('| 字段 | 值 |\n| --- | --- |\n| 案号 | A-1 |')
    expect(text).toBe('字段：案号；值：A-1')
  })

  it('按 BOM 与指定编码解码 Markdown 文件', () => {
    const utf8 = new globalThis.TextEncoder().encode('# 中文')
    expect(decodeMarkdownBytes(utf8, 'auto')).toBe('# 中文')
    const utf16 = new Uint8Array([0xff, 0xfe, 0x23, 0x00, 0x20, 0x00, 0x2d, 0x00, 0x2d, 0x00, 0x20, 0x00, 0x2d, 0x00, 0x2d, 0x00])
    expect(decodeMarkdownBytes(utf16, 'auto')).toBe('# -- --')
  })
})
