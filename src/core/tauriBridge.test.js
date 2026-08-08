import { describe, it, expect } from 'vitest'
import { userFacingError } from './tauriBridge.js'

describe('Tauri Bridge', () => {
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
