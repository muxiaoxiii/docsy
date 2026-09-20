import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('vue', async importOriginal => ({
  ...(await importOriginal()), onMounted: vi.fn(), onBeforeUnmount: vi.fn(),
  useSSRContext: () => ({ modules: new Set() }),
}))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({ useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }) }))
vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(), getPdfPageCount: vi.fn(), openPath: vi.fn(), userFacingError: error => String(error),
}))
vi.mock('element-plus', () => ({ ElMessage: { error: vi.fn(), warning: vi.fn(), success: vi.fn() }, ElNotification: vi.fn() }))
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import PdfToolsView from './PdfToolsView.vue'

describe('PDF 防复制界面链路', () => {
  beforeEach(() => vi.clearAllMocks())

  it('按后端字体字段显示检测结果，检测中的文件不能执行', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/a.pdf', name: 'a.pdf', hasAntiOcr: null }]
    expect(state.antiOcrReadyCount.value).toBe(0)
    tauriCallSafe.mockResolvedValue({ ok: true, data: { has_protection: true, has_backup: true, total_fonts: 4, protected_fonts: 2 } })
    await state.inspectAntiOcrFiles(['/a.pdf'])
    expect(state.antiOcrProtectedCount.value).toBe(1)
    expect(state.antiOcrFiles.value[0].statusText).toContain('2/4 个字体')
  })

  it('移除操作使用刚生成的副本，不再错误地处理原件', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/a.pdf', name: 'a.pdf', hasAntiOcr: false }]
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/a_anti_copy-2.pdf', detection: { has_protection: true, has_backup: true } } })
    await state.batchAntiOcrApply()
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/a_restored.pdf', detection: { has_protection: false, has_backup: false } } })
    await state.batchAntiOcrRemove()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('remove_anti_copy', { input: '/a_anti_copy-2.pdf', output: '/a_restored.pdf' })
    expect(state.antiOcrFiles.value[0].path).toBe('/a.pdf')
    expect(state.antiOcrFiles.value[0].outputPath).toBe('/a_restored.pdf')
    expect(state.antiOcrProtectedCount.value).toBe(0)
  })

  it('不能恢复没有原始映射备份的外部防复制文件', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/external.pdf', hasAntiOcr: true, hasBackup: false }]
    await state.batchAntiOcrRemove()
    expect(tauriCallSafe).not.toHaveBeenCalled()
  })
})

describe('PDF 队列的统一执行方式', () => {
  beforeEach(() => vi.clearAllMocks())

  it('添加加密 PDF 仅检测，确认执行后才解锁，并保留结果入口', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    tauriCallSafe.mockResolvedValue({ ok: true, data: { encrypted: true } })
    state.addUnlockFiles(['/a.pdf'])
    for (let i = 0; i < 12; i++) await Promise.resolve()
    expect(tauriCallSafe.mock.calls.map(([cmd]) => cmd)).toEqual(['inspect_pdf'])
    expect(state.unlockReadyCount.value).toBe(1)
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_unlocked.pdf', skipped: false } })
    await state.batchUnlock()
    expect(state.unlockFiles.value[0].outputPath).toBe('/a_unlocked.pdf')
    expect(tauriCallSafe).toHaveBeenLastCalledWith('unlock_pdf', { input: '/a.pdf' })
  })

  it('压缩仅在点击执行时运行，完成后更改设置不会自动重跑', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.loadCompressFiles(['/a.pdf'])
    expect(tauriCallSafe).not.toHaveBeenCalled()
    state.compressImageReencode.value = true
    state.compressLevel.value = 2
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_small.pdf', input_size: 100, output_size: 70 } })
    await state.runCompressQueue()
    expect(tauriCallSafe).toHaveBeenCalledWith('compress_pdf', { input: '/a.pdf', outputDir: null, level: 2 })
    state.compressLevel.value = 3
    await Promise.resolve()
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
    expect(state.compressFiles.value[0].status).toBe('pending')
    await state.runCompressQueue()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('compress_pdf', { input: '/a.pdf', outputDir: null, level: 3 })
  })

  it('处理中禁止清空、移除或追加；失败后仍可手动重试', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.loadCompressFiles(['/a.pdf', '/b.pdf'])
    let resolve
    tauriCallSafe.mockImplementationOnce(() => new Promise(r => { resolve = r }))
      .mockResolvedValue({ ok: true, data: { output_path: '/b_small.pdf' } })
    const running = state.runCompressQueue()
    state.clearCompressFiles()
    state.removeCompressFile(1)
    state.loadCompressFiles(['/c.pdf'])
    expect(state.compressFiles.value.map(f => f.path)).toEqual(['/a.pdf', '/b.pdf'])
    resolve({ ok: false, error: '读取失败' })
    await running
    expect(state.compressFiles.value[0].status).toBe('failed')
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_small.pdf' } })
    await state.runCompressQueue()
    expect(state.compressFiles.value.map(f => f.status)).toEqual(['done', 'done'])
    expect(tauriCallSafe.mock.calls.filter(([cmd]) => cmd === 'compress_pdf')).toHaveLength(3)
  })
})
