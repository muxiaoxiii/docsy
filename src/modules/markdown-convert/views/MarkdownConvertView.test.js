import { beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async original => ({ ...(await original()), onMounted: vi.fn(), onBeforeUnmount: vi.fn(), useSSRContext: () => ({ modules: new Set() }) }))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({ useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }) }))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn(), tauriCallQuiet: vi.fn(), openPath: vi.fn(), userFacingError: String }))
vi.mock('element-plus', () => ({ ElMessage: { info: vi.fn(), warning: vi.fn(), error: vi.fn(), success: vi.fn() }, ElMessageBox: {}, ElNotification: vi.fn() }))
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import View from './MarkdownConvertView.vue'

describe('MD 转换队列操作一致性', () => {
  beforeEach(() => vi.clearAllMocks())
  it('添加后修改输出选项，执行使用当前显示的格式和版式', async () => {
    const state = View.setup({}, { expose: vi.fn() })
    await state.loadFiles(['/a.md'])
    expect(tauriCallSafe).not.toHaveBeenCalled()
    state.fileOfficeFormat.value = 'xlsx'
    state.docxStyle.value = 'legal'
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a.xlsx' } })
    await state.runQueue()
    expect(tauriCallSafe).toHaveBeenCalledWith('convert_markdown', { input: '/a.md', outputDir: null, docEngine: null, outputFormat: 'xlsx', docxStyle: 'legal' })
    expect(state.files.value[0].directionTag).toContain('Excel')
  })
  it('运行中保留整个列表，取消的项目可以再次执行', async () => {
    const state = View.setup({}, { expose: vi.fn() })
    await state.loadFiles(['/a.md', '/b.md'])
    let resolve
    tauriCallSafe.mockImplementationOnce(() => new Promise(r => { resolve = r }))
    const running = state.runQueue()
    state.clearFiles()
    state.removeFile(1)
    await state.loadFiles(['/c.md'])
    expect(state.files.value).toHaveLength(2)
    resolve({ ok: false, error: '操作已取消' })
    await running
    expect(state.hasPendingFiles.value).toBe(true)
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/converted.docx' } })
    await state.runQueue()
    expect(state.files.value.map(f => f.status)).toEqual(['done', 'done'])
  })
})
