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

  it('supports multi-page pagination navigation', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '2x1' // 2 per page, 6 images => 3 pages
    expect(state.totalPages.value).toBe(3)
    expect(state.currentPageIndex.value).toBe(0)
    expect(state.previewImages.value.length).toBe(2)
    expect(state.previewImages.value[0].path).toBe('/images/0.png')

    state.nextPage()
    expect(state.currentPageIndex.value).toBe(1)
    expect(state.previewImages.value[0].path).toBe('/images/2.png')

    state.goToPage(2)
    expect(state.currentPageIndex.value).toBe(2)
    expect(state.previewImages.value[0].path).toBe('/images/4.png')

    state.prevPage()
    expect(state.currentPageIndex.value).toBe(1)
  })

  it('handles single page scaling lifecycle (adjust, save, cancel, reset)', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '2x1'
    state.settings.scale_mode = 'fixed_width'
    state.settings.fixed_width_mm = 50

    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)
    expect(state.isCurrentPageDirty.value).toBe(false)

    // Adjust slider on page 0 to 120%
    state.activePageScale.value = 120
    expect(state.isCurrentPageDirty.value).toBe(true)

    // Save
    state.saveCurrentPageScale()
    expect(state.hasSavedScale.value).toBe(true)
    expect(state.isCurrentPageDirty.value).toBe(false)
    expect(state.pageScales.value[0]).toBe(1.2)

    // Switch to page 1
    state.nextPage()
    expect(state.currentPageIndex.value).toBe(1)
    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)

    // Switch back to page 0
    state.prevPage()
    expect(state.activePageScale.value).toBe(120)
    expect(state.hasSavedScale.value).toBe(true)

    // Modify but cancel
    state.activePageScale.value = 80
    expect(state.isCurrentPageDirty.value).toBe(true)
    state.cancelCurrentPageScale()
    expect(state.activePageScale.value).toBe(120)
    expect(state.isCurrentPageDirty.value).toBe(false)

    // Reset
    state.resetCurrentPageScale()
    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)
    expect(state.pageScales.value[0]).toBeUndefined()
  })

  it('passes 1.0.7 arguments (page_scales, pair_mode, caption_position) to run', async () => {
    const state = useImagePaddlerState()
    state.folders.value = ['/images']
    state.analysis.value = { images }
    state.settings.layout = '2x1'
    state.pageScales.value = { 0: 1.15 }
    state.settings.pair_mode = 'page-gather'
    state.settings.caption_position = 'above'

    tauriCallSafe.mockResolvedValue({ ok: true, data: { images: 6, pages: 3 } })
    await state.run()

    const payload = tauriCallSafe.mock.calls[0][1].args
    expect(payload.page_scales).toEqual([1.15, 1.0, 1.0])
    expect(payload.pair_mode).toBe('page-gather')
    expect(payload.caption_position).toBe('above')
    expect(payload.print_safety_pad_mm).toBe(6)
  })

  it('detects page conflicts and provides badge resolvers and doclet tips', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '2x1'
    const badge = state.imageBadgeResolver(images[0])
    expect(badge).not.toBeNull()
    expect(badge.pageNumber).toBe(1)
    expect(badge.pageIndex).toBe(0)
    expect(typeof badge.color).toBe('string')
    expect(typeof state.currentPageConflictState.value.worstColor).toBe('string')
  })

  it('golden fixture: flow mode last page compacts without empty grid slots', () => {
    const state = useImagePaddlerState()
    const fiveImages = images.slice(0, 5)
    state.analysis.value = { images: fiveImages }
    state.settings.output_format = 'docx'
    state.settings.use_table = false
    state.settings.layout = '2x1' // Flow mode: 2 rows, 1 col per page

    expect(state.totalPages.value).toBe(3)
    // Page 0 has 2 images -> 2 rows, 1 col
    expect(state.previewSlots.value.length).toBe(2)
    expect(state.previewLayoutGrid.value).toEqual({ rows: 2, cols: 1 })

    // Jump to last page (page index 2)
    state.goToPage(2)
    expect(state.currentPageIndex.value).toBe(2)
    expect(state.previewSlots.value.length).toBe(1)
    // 1 image on last page should compact to 1x1, without leftover empty slots
    expect(state.previewLayoutGrid.value).toEqual({ rows: 1, cols: 1 })
  })

  it('supports batch scale actions: apply to all pages, subsequent pages, and reset all', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images } // 6 images, layout 2x1 -> 3 pages
    state.settings.layout = '2x1'
    expect(state.totalPages.value).toBe(3)

    // Set page 0 to 85% and apply to all pages
    state.activePageScale.value = 85
    state.applyScaleToAllPages()
    expect(state.pageScales.value[0]).toBe(0.85)
    expect(state.pageScales.value[1]).toBe(0.85)
    expect(state.pageScales.value[2]).toBe(0.85)
    expect(state.hasAnySavedScales.value).toBe(true)

    // On page 1, adjust to 90% and apply to subsequent pages (pages 1 and 2)
    state.goToPage(1)
    state.activePageScale.value = 90
    state.applyScaleToSubsequentPages()
    expect(state.pageScales.value[0]).toBe(0.85)
    expect(state.pageScales.value[1]).toBe(0.9)
    expect(state.pageScales.value[2]).toBe(0.9)

    // Reset all pages
    state.resetAllPageScales()
    expect(Object.keys(state.pageScales.value).length).toBe(0)
    expect(state.hasAnySavedScales.value).toBe(false)
    expect(state.activePageScale.value).toBe(100)
  })

  it('supports image annotations and reserve note placeholder', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '2x1'

    const path = images[0].path
    // Default title is filename
    expect(state.imageTitle(path)).toBe('0.png')
    expect(state.imageDescription(path)).toBe('')

    // Set custom annotation
    state.setImageAnnotation(path, {
      title: '现场勘查照片 1',
      description: '拍摄时间：2026-09-18',
    })
    expect(state.imageTitle(path)).toBe('现场勘查照片 1')
    expect(state.imageDescription(path)).toBe('拍摄时间：2026-09-18')

    // Test reserve_note_placeholder
    state.settings.reserve_note_placeholder = true
    state.settings.note_placeholder_text = '[点击输入说明]'
    // Path with annotation keeps custom description
    expect(state.imageDescription(path)).toBe('拍摄时间：2026-09-18')
    // Other path gets placeholder
    expect(state.imageDescription(images[1].path)).toBe('[点击输入说明]')

    // Verify layoutMetrics reserves height for note
    expect(state.layoutMetrics.value.noteReserve).toBeGreaterThan(0)
  })

  it('P0 regression: previewImageAreaContainerStyle maps pair_mode alignment correctly', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images: images.slice(0, 2) } // 2 images, 2x1 grid
    state.settings.layout = '2x1'

    // page-gather: top image gathers down (flex-end), bottom image gathers up (flex-start)
    state.settings.pair_mode = 'page-gather'
    expect(state.previewImageAreaContainerStyle(0)).toEqual({
      justifyContent: 'center',
      alignItems: 'flex-end',
    })
    expect(state.previewImageAreaContainerStyle(1)).toEqual({
      justifyContent: 'center',
      alignItems: 'flex-start',
    })

    // page-spread: top image spreads up (flex-start), bottom image spreads down (flex-end)
    state.settings.pair_mode = 'page-spread'
    expect(state.previewImageAreaContainerStyle(0)).toEqual({
      justifyContent: 'center',
      alignItems: 'flex-start',
    })
    expect(state.previewImageAreaContainerStyle(1)).toEqual({
      justifyContent: 'center',
      alignItems: 'flex-end',
    })

    // 1x2 grid (side by side)
    state.settings.layout = '1x2'
    state.settings.pair_mode = 'page-gather'
    expect(state.previewImageAreaContainerStyle(0)).toEqual({
      justifyContent: 'flex-end',
      alignItems: 'center',
    })
    expect(state.previewImageAreaContainerStyle(1)).toEqual({
      justifyContent: 'flex-start',
      alignItems: 'center',
    })
  })

  it('P1 regression: safeColumnWidthValue does not drift on compacted last page', () => {
    const state = useImagePaddlerState()
    // 3 images with 2x1 grid -> 2 pages: page 0 has 2 images, page 1 (last page) has 1 image
    state.analysis.value = { images: images.slice(0, 3) }
    state.settings.layout = '2x1'
    state.settings.margin_mm = 12
    state.settings.print_safety_pad_mm = 6

    const widthPage0 = state.safeColumnWidthValue.value
    state.goToPage(1) // Jump to last page, which compacts to 1x1
    expect(state.previewLayoutGrid.value).toEqual({ rows: 1, cols: 1 })
    // safeColumnWidthValue must remain bounded by primary layoutGrid (cols=1 in 2x1), not drift
    expect(state.safeColumnWidthValue.value).toBe(widthPage0)
  })

  it('P1 & P2 regression: imageBadgeResolver uses per-page metrics and caches results', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images: images.slice(0, 3) } // 3 images, 2 pages
    state.settings.layout = '2x1'

    const badge0 = state.imageBadgeResolver(images[0])
    expect(badge0.pageNumber).toBe(1)
    expect(badge0.color).toBeDefined()

    const badge2 = state.imageBadgeResolver(images[2])
    expect(badge2.pageNumber).toBe(2)
    expect(badge2.color).toBeDefined()
  })
})
