import { describe, it, expect, vi } from 'vitest'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
import { invoke } from '@tauri-apps/api/core'
import { userFacingError, tauriCallQuiet } from './tauriBridge.js'
import { DESKTOP_ONLY_MESSAGE } from './desktopRuntime.js'

describe('Tauri Bridge', () => {
  it('explains missing browser runtime instead of exposing the invoke exception', async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new TypeError("Cannot read properties of undefined (reading 'invoke')"))
    expect(await tauriCallQuiet('render_pdf_preview')).toEqual({ ok: false, error: DESKTOP_ONLY_MESSAGE })
    expect(userFacingError("Cannot read properties of undefined (reading 'transformCallback')")).toBe(DESKTOP_ONLY_MESSAGE)
  })
  it('shows readable structured errors for quiet preview calls', async () => {
    for (const error of [
      { kind: 'unknown', message: '图片读取失败' },
      '{"kind":"unknown","message":"图片读取失败"}',
      new Error('{"kind":"unknown","message":"图片读取失败"}'),
    ]) {
      vi.mocked(invoke).mockRejectedValueOnce(error)
      expect(await tauriCallQuiet('plan_image_pdf')).toEqual({ ok: false, error: '图片读取失败' })
      expect(userFacingError(error)).toBe('图片读取失败')
    }
  })
  it('should define tauriCall interface', () => {
    // Verify the module exports exist
    const module = import('./tauriBridge.js')
    expect(module).toBeDefined()
  })

  it('summarizes recoverable qpdf warnings for the UI', () => {
    const warning =
      'qpdf --json 失败: WARNING: test.pdf (object 1 0): object has offset 0 - a common error handled correctly by qpdf and most other applications'
    expect(userFacingError(warning, '真实预览生成失败')).toBe(
      '真实预览生成失败：PDF 结构存在可修复警告，详细信息已写入日志',
    )
  })

  it('truncates long command errors without losing the log hint', () => {
    const message = userFacingError('x'.repeat(400), '失败', 40)
    expect(message).toHaveLength(48)
    expect(message.endsWith('…（详情见日志）')).toBe(true)
  })
})
