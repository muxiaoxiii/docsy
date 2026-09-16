import { beforeEach, describe, expect, it, vi } from 'vitest'
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
const { open } = await import('@tauri-apps/plugin-dialog')
const { tauriCallSafe } = await import('../../../core/tauriBridge.js')
const { ElMessageBox } = await import('element-plus')

beforeEach(() => vi.clearAllMocks())

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

describe('批量生成中的模板隔离', () => {
  it('文件选择期间拦截重复执行，取消后释放状态', async () => {
    let selectFile
    open.mockImplementationOnce(() => new Promise(resolve => { selectFile = resolve }))
    const batch = makeBatchFill()
    const pending = batch.handleBatchCommand('import')
    expect(batch.batchProcessing.value).toBe(true)
    await batch.handleBatchCommand('import')
    expect(open).toHaveBeenCalledTimes(1)
    selectFile(null)
    await pending
    expect(batch.batchProcessing.value).toBe(false)
  })

  it('切换模板后仍用原模板生成并保存历史', async () => {
    const templatePath = ref('/t/原模板.docsytpl')
    const batch = useBatchFill(templatePath, ref({ fields: [] }), () => ({}), () => ({}), ref('、'))
    open.mockResolvedValueOnce('/t/data.xlsx').mockResolvedValueOnce('/output')
    tauriCallSafe.mockImplementation(async command => {
      if (command === 'validate_batch_import') return { ok: true, data: { templateIdMatch: true, totalRows: 1, validRows: 1, warnings: [], errors: [] } }
      if (command === 'batch_render_from_xlsx') return { ok: true, data: { success: 1, rows: [{ outputPath: '/output/1.docx', values: { name: 'A' } }] } }
      return { ok: true, data: 1 }
    })
    ElMessageBox.confirm.mockImplementationOnce(async () => { templatePath.value = '/t/新模板.docsytpl' })
    await batch.handleBatchCommand('import')
    expect(tauriCallSafe).toHaveBeenCalledWith('batch_render_from_xlsx', expect.objectContaining({ templatePath: '/t/原模板.docsytpl' }))
    batch.openBatchSaveFromCompletion()
    await batch.submitBatchSave()
    expect(tauriCallSafe).toHaveBeenCalledWith('save_batch_history_rows', { rows: [{ templatePath: '/t/原模板.docsytpl', outputPath: '/output/1.docx', values: { name: 'A' } }] })
  })
})
