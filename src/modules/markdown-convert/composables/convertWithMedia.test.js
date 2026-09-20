import { beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn() }))
vi.mock('./renderMarkdownMedia.js', () => ({ renderMarkdownMedia: vi.fn() }))
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { renderMarkdownMedia } from './renderMarkdownMedia.js'
import { convertWithMedia } from './convertWithMedia.js'

describe('MD 公式与图表转换', () => {
  beforeEach(() => vi.resetAllMocks())
  it('文件与粘贴都将渲染资源和源哈希传给导出器', async () => {
    const plan = { sourceHash: 'source-hash', items: [{ id: 'docsy-media-0', kind: 'math', source: 'x' }] }
    const rendered = { sourceHash: 'source-hash', items: [{ id: 'docsy-media-0', png: 'png', mathml: '<math/>' }] }
    for (const [command, args, input] of [
      [
        'convert_markdown',
        { input: '/日本語.md', inputEncoding: 'shift_jis' },
        { input: '/日本語.md', inputEncoding: 'shift_jis' },
      ],
      ['convert_markdown_text', { text: '$x$', format: 'docx' }, { text: '$x$' }],
    ]) {
      tauriCallSafe
        .mockResolvedValueOnce({ ok: true, data: plan })
        .mockResolvedValueOnce({ ok: true, data: { output_path: '/out.docx' } })
      renderMarkdownMedia.mockResolvedValueOnce(rendered)
      const result = await convertWithMedia(command, args)
      expect(tauriCallSafe).toHaveBeenCalledWith('prepare_markdown_media', input)
      expect(tauriCallSafe).toHaveBeenLastCalledWith(command, { ...args, renderedMedia: rendered })
      expect(result.ok).toBe(true)
    }
  })
  it('普通文本不调用图形渲染器', async () => {
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { items: [] } }).mockResolvedValueOnce({ ok: true })
    await convertWithMedia('convert_markdown', { input: '/a.md' })
    expect(renderMarkdownMedia).not.toHaveBeenCalled()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('convert_markdown', { input: '/a.md' })
  })
  it('图表或公式错误时不会写入不完整文档，重试能恢复', async () => {
    const plan = { sourceHash: 'h', items: [{ kind: 'mermaid' }] }
    tauriCallSafe.mockResolvedValue({ ok: true, data: plan })
    renderMarkdownMedia.mockRejectedValueOnce(new Error('第 1 个 Mermaid 图渲染失败'))
    const result = await convertWithMedia('convert_markdown_text', { text: 'bad', format: 'docx' })
    expect(result.ok).toBe(false)
    expect(result.error).toContain('Mermaid')
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
  })
  it('停止发生在读取或渲染结束时也不会开始原生导出', async () => {
    tauriCallSafe.mockResolvedValue({ ok: true, data: { items: [] } })
    const result = await convertWithMedia('convert_markdown', { input: '/a.md' }, () => {
      throw new Error('操作已取消')
    })
    expect(result.error).toBe('操作已取消')
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
  })
})
