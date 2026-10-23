import { afterEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async original => ({ ...(await original()), onBeforeUnmount: vi.fn(), useSSRContext: () => ({ modules: new Set() }) }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readFile: vi.fn() }))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn() }))
import { readFile } from '@tauri-apps/plugin-fs'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import Preview from './PdfJsPreview.vue'
import { DESKTOP_ONLY_MESSAGE } from '../../../core/desktopRuntime.js'
afterEach(() => { vi.clearAllMocks(); vi.unstubAllGlobals() })
const props = () => ({ filePath: '/sample.pdf', page: 1, scale: 1.6, reloadKey: 0, engine: 'auto' })
describe('PDF preview runtime boundary', () => {
  it('explains browser-only mode before attempting native calls or reading a local file', () => {
    vi.stubGlobal('window', {})
    const emit = vi.fn(), state = Preview.setup(props(), { emit, expose: vi.fn() })
    expect(state.errorText.value).toBe(DESKTOP_ONLY_MESSAGE)
    expect(emit).toHaveBeenCalledWith('error', DESKTOP_ONLY_MESSAGE)
    expect(tauriCallSafe).not.toHaveBeenCalled()
    expect(readFile).not.toHaveBeenCalled()
  })
  it('uses the real backend path when the desktop runtime is present', async () => {
    vi.stubGlobal('window', { __TAURI_INTERNALS__: { invoke: vi.fn() } })
    tauriCallSafe.mockResolvedValue({ ok: true, data: { imageDataUrl: 'data:image/png;base64,fixture', page: 1, pages: 6, widthPt: 595, heightPt: 842, widthPx: 992, heightPx: 1403 } })
    const emit = vi.fn(), state = Preview.setup(props(), { emit, expose: vi.fn() })
    await Promise.resolve(); await Promise.resolve(); await Promise.resolve()
    expect(tauriCallSafe).toHaveBeenCalledWith('render_pdf_preview', { args: { inputPath: '/sample.pdf', page: 1, dpi: 115 } })
    expect(state.imageDataUrl.value).toContain('data:image/png')
    expect(state.errorText.value).toBe('')
    expect(emit).toHaveBeenCalledWith('loaded', expect.objectContaining({ pages: 6 }))
  })
})
