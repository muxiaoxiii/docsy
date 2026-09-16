import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('vue', async (importOriginal) => ({
  ...(await importOriginal()),
  onMounted: vi.fn(),
  onBeforeUnmount: vi.fn(),
  onActivated: vi.fn(),
}))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({
  useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }),
}))
vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(),
  tauriCallQuiet: vi.fn(async () => ({ ok: false })),
  userFacingError: vi.fn(),
  openPath: vi.fn(),
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn() } }))

import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { useImagePaddlerState } from './useImagePaddlerState.js'

const images = Array.from({ length: 6 }, (_, index) => ({ path: `/images/${index}.png`, width: 100, height: 100 }))

describe('image paddler state integration', () => {
  beforeEach(() => vi.clearAllMocks())

  it('exports the displayed N-order exactly once', async () => {
    const state = useImagePaddlerState()
    state.folders.value = ['/images']
    state.analysis.value = { images }
    state.settings.layout = '2x3'
    state.settings.order_mode = 'n'
    tauriCallSafe.mockResolvedValue({ ok: true, data: { images: 6, pages: 1 } })
    await state.run()
    const payload = tauriCallSafe.mock.calls[0][1].args
    expect(payload.image_paths).toEqual([0, 3, 1, 4, 2, 5].map((index) => `/images/${index}.png`))
    expect(payload.order_mode).toBe('custom')
    expect(payload.fixed_width_mm).toBe(state.actualImageWidth.value)
  })

  it('does not restore an old analysis after clearing sources', async () => {
    let resolveRequest
    tauriCallSafe.mockReturnValue(
      new Promise((resolve) => {
        resolveRequest = resolve
      }),
    )
    const state = useImagePaddlerState()
    state.folders.value = ['/images']
    const pending = state.analyze()
    state.clearAllSources()
    resolveRequest({ ok: true, data: { images } })
    await pending
    expect(state.analysis.value).toBeNull()
    expect(state.analyzing.value).toBe(false)
  })

  it('reacts to width, layout and table mode changes', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '1x3'
    state.settings.scale_mode = 'fixed_width'
    state.settings.fixed_width_mm = 20
    const narrow = state.previewImageStyle(images[0]).width
    state.settings.fixed_width_mm = 40
    expect(parseFloat(state.previewImageStyle(images[0]).width)).toBeCloseTo(parseFloat(narrow) * 2)
    expect(state.previewGridStyle.value.gridTemplateColumns).toContain('repeat(3,')
    state.settings.output_format = 'docx'
    state.settings.use_table = false
    expect(state.previewGridStyle.value.gridTemplateColumns).toContain('repeat(1,')
    expect(state.optionLayoutLabel('1x3')).toBe('3 张（上下）')
    expect(state.previewNameStyle.value.color).toBe('#4B5563')
  })

  it('清空素材后不会把旧导出结果显示为当前结果', async () => {
    let finishExport
    tauriCallSafe.mockImplementationOnce(() => new Promise(resolve => { finishExport = resolve }))
    const state = useImagePaddlerState()
    state.folders.value = ['/images']
    state.analysis.value = { images }
    const pending = state.run()
    state.clearAllSources()
    finishExport({ ok: true, data: { images: 6, pages: 1, output_path: '/old.pdf' } })
    await pending
    expect(state.generating.value).toBe(false)
    expect(state.generatedResult.value).toBeNull()
  })
})
