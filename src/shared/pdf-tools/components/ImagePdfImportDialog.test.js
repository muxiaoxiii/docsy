import { afterEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async (original) => ({
  ...(await original()),
  onBeforeUnmount: vi.fn(),
  useSSRContext: () => ({ modules: new Set() }),
}))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallQuiet: vi.fn(), tauriCallSafe: vi.fn() }))
import ImagePdfImportDialog from './ImagePdfImportDialog.vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
describe('reviewed image import dialog page organization', () => {
  it('keeps images and manual scale when grid controls change, and fits only on request', () => {
    vi.useFakeTimers()
    const state = ImagePdfImportDialog.setup({}, { expose: vi.fn() })
    state.open(['/a.png', '/b.png'])
    state.images.value[0].referenceWidth = 190
    state.images.value[0].percent = 35
    state.images.value[0].rotation = 90
    state.images.value[0].trim = [5, 10, 15, 20]
    state.joinPage({ from: 1, target: state.images.value[0].pageId })
    state.currentLayout.value.rows = 1
    state.layoutChanged('rows')
    expect(state.args().page_layouts).toEqual([{ count: 2, rows: 1, cols: 2 }])
    expect(state.args().images[0]).toMatchObject({
      scale: 0.35,
      reference_width: 190,
      rotation: 90,
      crop: [0.05, 0.1, 0.85, 0.8],
    })
    state.currentLayout.value.cols = 1
    state.layoutChanged('cols')
    expect(state.args().page_layouts).toEqual([{ count: 2, rows: 2, cols: 1 }])
    expect(state.args().images.map((image) => image.path)).toEqual(['/a.png', '/b.png'])
    state.plan.value = {
      pages: [
        [
          { index: 0, max_scale: 0.42 },
          { index: 1, max_scale: 0.78 },
        ],
      ],
    }
    state.fitPage()
    expect(state.args().images.map((image) => image.scale)).toEqual([0.42, 0.78])
    state.finish(null)
  })
  afterEach(() => {
    vi.useRealTimers()
  })
  it('defaults to one image per page, groups only on a page drop, and restores individual pages', () => {
    vi.useFakeTimers()
    const state = ImagePdfImportDialog.setup({}, { expose: vi.fn() })
    state.open(['/a.png', '/b.png', '/c.png'])
    expect(state.args().page_layouts).toEqual(Array.from({ length: 3 }, () => ({ count: 1, rows: 1, cols: 1 })))
    state.reorder({ from: 0, to: 1 })
    expect(state.args().page_layouts).toHaveLength(3)
    const target = state.images.value[0].pageId
    state.joinPage({ from: 1, target })
    expect(state.args().page_layouts).toEqual([
      { count: 2, rows: 2, cols: 1 },
      { count: 1, rows: 1, cols: 1 },
    ])
    state.splitActive()
    expect(state.args().page_layouts).toHaveLength(3)
    state.finish(null)
    expect(state.images.value).toEqual([])
  })
  it('finishes the close animation before opening the next reviewed batch', async () => {
    vi.useFakeTimers()
    const state = ImagePdfImportDialog.setup({}, { expose: vi.fn() })
    const first = state.open(['/a.png'])
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: '/a.pdf' })
    await state.confirm()
    let settled = false
    first.then(() => {
      settled = true
    })
    await Promise.resolve()
    expect(settled).toBe(false)
    state.handleClosed()
    expect(await first).toBe('/a.pdf')
    const second = state.open(['/b.png'])
    expect(state.images.value[0].path).toBe('/b.png')
    state.handleClosed()
    expect(await second).toBeNull()
  })
  it('maps dragged crop edges to source fractions and preserves physical scale and rotation', async () => {
    vi.useFakeTimers()
    const state = ImagePdfImportDialog.setup({}, { expose: vi.fn() })
    state.open(['/a.png'])
    state.images.value[0].rotation = 90
    state.images.value[0].referenceWidth = 190
    state.previews.set('/a.png', { src: 'data:image/png;base64,fixture' })
    await state.openCrop()
    state.cropSurface.value = { setPointerCapture: vi.fn(), getBoundingClientRect: () => ({ width: 600, height: 360 }) }
    state.startCrop({ button: 0, pointerId: 7, clientX: 0, clientY: 0 }, 'nw')
    state.moveCrop({ pointerId: 8, clientX: 120, clientY: 72 })
    expect(state.cropDraft.value).toEqual([0, 0, 1, 1])
    state.moveCrop({ pointerId: 7, clientX: 60, clientY: 36 })
    expect(state.cropDraft.value).toEqual([0.1, 0.1, 1, 1])
    state.applyCrop()
    expect(state.args().images[0]).toMatchObject({
      rotation: 90,
      scale: 1,
      reference_width: 190,
      crop: [0.1, 0.1, 1, 1],
    })
    expect(state.cropVisible.value).toBe(false)
    state.endCrop()
    state.finish(null)
  })
})
