import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

vi.mock('element-plus', () => ({
  ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  ElMessageBox: { confirm: vi.fn(), alert: vi.fn() },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => null),
}))

vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(async () => ({ ok: true, data: null })),
  openPath: vi.fn(async () => ({ ok: true })),
  userFacingError: (error, fallback) => fallback || String(error),
}))

const { useBatchFill } = await import('./useBatchFill.js')

function makeBatchFill() {
  return useBatchFill(
    ref('/t/模板.docsytpl'),
    ref({ fields: [] }),
    () => ({}),
    () => ({}),
    ref('、'),
    () => {},
  )
}

describe('openBatchSaveDialog 批量保存对话框', () => {
  it('打开时默认全选（不再有行上 selected 死状态）', () => {
    const b = makeBatchFill()
    b.openBatchSaveDialog([
      { outputPath: '/o/1.docx', values: { 法院: 'A' } },
      { outputPath: '/o/2.docx', values: { 法院: 'B' } },
    ])
    expect(b.batchSaveVisible.value).toBe(true)
    expect(b.batchSaveRows.value).toHaveLength(2)
    // 勾选状态以 batchSaveSelected 为准，打开即为全选
    expect(b.batchSaveSelected.value).toEqual(['0', '1'])
    // 行上不再携带从未被读取的 selected 属性
    expect(b.batchSaveRows.value.every((row) => !('selected' in row))).toBe(true)
  })

  it('空行列表打开时选择集为空', () => {
    const b = makeBatchFill()
    b.openBatchSaveDialog([])
    expect(b.batchSaveVisible.value).toBe(true)
    expect(b.batchSaveSelected.value).toEqual([])
  })
})
