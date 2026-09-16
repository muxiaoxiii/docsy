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
