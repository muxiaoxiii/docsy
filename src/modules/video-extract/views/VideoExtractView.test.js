import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async original => ({ ...(await original()), onMounted: vi.fn(), onBeforeUnmount: vi.fn(), onDeactivated: vi.fn(), useSSRContext: () => ({ modules: new Set() }) }))
vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }) }))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({ useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }) }))
vi.mock('../../../stores/workspace.js', () => ({ useWorkspaceStore: () => ({ clearFrameSelectionDraft: vi.fn(), setFrameSelectionDraft: vi.fn() }) }))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn(), tauriCallQuiet: vi.fn(), userFacingError: String }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn() }, ElMessageBox: { confirm: vi.fn() } }))
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { ElMessageBox } from 'element-plus'
import View from './VideoExtractView.vue'

const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r }); return { promise, resolve } }
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve() }
describe('视频历史恢复时序', () => {
  beforeEach(() => { vi.clearAllMocks(); vi.useFakeTimers(); vi.stubGlobal('window', globalThis) })
  afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals() })
  it('分析超过 650ms 仍先恢复人工决定，之后才允许保存', async () => {
    const analysis = deferred()
    tauriCallSafe.mockImplementation((cmd) => {
      if (cmd === 'find_media_workspace_session') return Promise.resolve({ ok: true, data: { found: true, match_kind: 'exact', session_id: 'saved', items: [{ path: '/A/1.png', user_decision: 'exclude', match_status: 'matched' }] } })
      if (cmd === 'analyze_frame_selection') return analysis.promise
      if (cmd === 'save_media_workspace_session') return Promise.resolve({ ok: true, data: { id: 'saved' } })
    })
    ElMessageBox.confirm.mockResolvedValue('confirm')
    const state = View.setup({}, { expose: vi.fn() })
    const pending = state.initializeResultImages(['/A/1.png'], '/A')
    await flush()
    await vi.advanceTimersByTimeAsync(1000)
    expect(tauriCallSafe.mock.calls.some(([cmd]) => cmd === 'save_media_workspace_session')).toBe(false)
    expect(state.resultImages.value[0].user_decision).toBe('exclude')
    analysis.resolve({ ok: true, data: { items: [{ path: '/A/1.png', engine_decision: 'keep' }] } })
    await pending
    await vi.advanceTimersByTimeAsync(650)
    const saved = tauriCallSafe.mock.calls.find(([cmd]) => cmd === 'save_media_workspace_session')[1].args
    expect(saved.id).toBe('saved')
    expect(saved.items[0].user_decision).toBe('exclude')
  })
  it('切换来源后旧保存响应不能回填新会话 ID', async () => {
    const save = deferred()
    tauriCallSafe.mockImplementation(cmd => cmd === 'save_media_workspace_session' ? save.promise : Promise.resolve({ ok: true, data: { found: false } }))
    const state = View.setup({}, { expose: vi.fn() })
    state.analysisSettings.autoAnalyze = false
    await state.initializeResultImages(['/A/1.png'], '/A')
    const old = state.saveMediaSession()
    const next = state.initializeResultImages(['/B/1.png'], '/B')
    save.resolve({ ok: true, data: { id: 'old-A' } })
    await old; await next
    expect(state.sourceDirectory.value).toBe('/B')
    expect(state.mediaSessionId.value).toBeNull()
  })
})
