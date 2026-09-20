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
    state.settings.size_mode = 'manual'
    state.settings.scale_mode = 'fixed_width'
    state.settings.fixed_width_mm = 40
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

  it('handles single page scaling lifecycle with instant apply + cancel to global', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images }
    state.settings.layout = '2x1'
    state.settings.scale_mode = 'fixed_width'
    state.settings.fixed_width_mm = 50

    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)
    expect(state.isCurrentPageDirty.value).toBe(false)

    // Adjust slider on page 0 to 120% — 立即进入导出状态
    state.activePageScale.value = 120
    expect(state.isCurrentPageDirty.value).toBe(true)
    expect(state.pageScales.value[0]).toBeCloseTo(1.2)
    expect(state.hasSavedScale.value).toBe(true)

    // Switch to page 1 — 继承全局 100%
    state.nextPage()
    expect(state.currentPageIndex.value).toBe(1)
    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)
    expect(state.pageScales.value[0]).toBeCloseTo(1.2)

    // Switch back to page 0
    state.prevPage()
    expect(state.currentPageIndex.value).toBe(0)
    expect(state.activePageScale.value).toBe(120)
    expect(state.hasSavedScale.value).toBe(true)

    // Modify then cancel = 回到全局，清除局部覆盖
    state.activePageScale.value = 80
    expect(state.isCurrentPageDirty.value).toBe(true)
    state.cancelCurrentPageScale()
    expect(state.activePageScale.value).toBe(100)
    expect(state.pageScales.value[0]).toBeUndefined()
    expect(state.isCurrentPageDirty.value).toBe(false)

    // Local override then reset
    state.activePageScale.value = 130
    state.resetCurrentPageScale()
    expect(state.activePageScale.value).toBe(100)
    expect(state.hasSavedScale.value).toBe(false)
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

  it('golden fixture: flow mode last page keeps grid by default, reflow only when opted in', () => {
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

    // Jump to last page (page index 2) — 默认保留原网格，避免末页栏宽静默变化
    state.goToPage(2)
    expect(state.currentPageIndex.value).toBe(2)
    expect(state.previewSlots.value.length).toBe(1)
    expect(state.previewLayoutGrid.value).toEqual({ rows: 2, cols: 1 })

    // 显式选择末页重新铺满后才 compact
    state.settings.last_page_mode = 'reflow'
    expect(state.previewLayoutGrid.value).toEqual({ rows: 1, cols: 1 })
  })

  it('supports batch scale actions: global scale is independent, local overrides optional', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images } // 6 images, layout 2x1 -> 3 pages
    state.settings.layout = '2x1'
    expect(state.totalPages.value).toBe(3)

    // Set global scale to 85% — 不写死每页，新页自动继承
    state.activePageScale.value = 85
    state.applyScaleToAllPages()
    expect(state.globalScalePercent.value).toBe(85)
    expect(state.pageScales.value[0]).toBeUndefined()
    expect(state.pageScales.value[1]).toBeUndefined()

    // On page 1, adjust to 90% as local override
    state.goToPage(1)
    state.activePageScale.value = 90
    expect(state.pageScales.value[1]).toBeCloseTo(0.9)
    expect(state.pageScales.value[0]).toBeUndefined()
    expect(state.pageScales.value[2]).toBeUndefined()

    // Reset all
    state.resetAllPageScales()
    expect(Object.keys(state.pageScales.value).length).toBe(0)
    expect(state.globalScalePercent.value).toBe(100)
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

  it('P1 regression: safeColumnWidthValue does not drift on incomplete last page', () => {
    const state = useImagePaddlerState()
    // 3 images with 2x1 grid -> 2 pages: page 0 has 2 images, page 1 (last page) has 1 image
    state.analysis.value = { images: images.slice(0, 3) }
    state.settings.layout = '2x1'
    state.settings.margin_mm = 12
    state.settings.print_safety_pad_mm = 6

    const widthPage0 = state.safeColumnWidthValue.value
    state.goToPage(1) // 末页默认保留原网格
    expect(state.previewLayoutGrid.value).toEqual({ rows: 2, cols: 1 })
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

  it('renders border with robust inset box-shadow and visible white contrast', () => {
    const state = useImagePaddlerState()
    expect(state.previewCellStyle.value.boxShadow).toBe('none')

    state.settings.border_enabled = true
    state.settings.border_color = 'red'
    expect(state.previewCellStyle.value.boxShadow).toBe('inset 0 0 0 1.5px #dc2626')

    state.settings.border_color = 'white'
    expect(state.previewCellStyle.value.boxShadow).toContain('#ffffff')
    expect(state.previewCellStyle.value.boxShadow).toContain('rgba(0, 0, 0, 0.25)')
  })

  it('supports global scale controls across all pages', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images: images.slice(0, 4) }
    state.settings.layout = '2x1' // 2 per page -> 2 pages
    expect(state.totalPages.value).toBe(2)

    // Initially 100%
    expect(state.globalScalePercent.value).toBe(100)

    // Set global scale to 80% — 独立存储，导出按 effectiveScaleForPage 继承
    state.setGlobalScale(80)
    expect(state.globalScalePercent.value).toBe(80)
    expect(state.pageScales.value[0]).toBeUndefined()
    expect(state.pageScales.value[1]).toBeUndefined()

    // 重新分页后全局比例仍保留
    state.settings.layout = '1'
    expect(state.totalPages.value).toBe(4)
    expect(state.globalScalePercent.value).toBe(80)

    // Reset
    state.resetAllPageScales()
    expect(state.globalScalePercent.value).toBe(100)
    expect(Object.keys(state.pageScales.value).length).toBe(0)
  })

  it('layout-aware recommendation updates with per-page count and does not overwrite layout on import apply', () => {
    const state = useImagePaddlerState()
    const wideImages = Array.from({ length: 9 }, (_, index) => ({
      path: `/images/wide_${index}.png`,
      width: 1920,
      height: 1080,
    }))
    state.analysis.value = {
      images: wideImages,
      recommended: {
        orientation: 'portrait',
        layout: '2x1',
        scale_mode: 'fixed_width',
        recommended_width_mm: 165,
        margin_mm: 12,
        show_filename: true,
        reason: '导入推荐：竖页上下 2 张',
      },
    }
    state.settings.size_mode = 'smart'
    state.settings.images_per_page = 2
    state.settings.arrange_mode = 'stack'
    state.settings.scale_mode = 'fixed_width'
    state.settings.fixed_width_mm = 165
    state.settings.layout = '2x1'

    const widthTwo = state.layoutAwareRecommendation.value.recommended_width_mm
    expect(widthTwo).toBeGreaterThan(80)

    // 切到 9 张：智能宽度必须跟着变小，而不是仍用 165
    state.setImagesPerPage(9)
    expect(state.perPage.value).toBe(9)
    const recNine = state.layoutAwareRecommendation.value
    expect(recNine.recommended_width_mm).toBeLessThan(widthTwo)
    expect(recNine.recommended_width_mm).toBeLessThanOrEqual(recNine.safe_column_width_mm + 1)
    // smart 模式自动收敛固定宽度
    expect(state.settings.fixed_width_mm).toBe(recNine.recommended_width_mm)
    expect(state.settings.layout).toBe('3x3')

    // 应用导入推荐会改回导入方案的布局
    state.applyImportRecommendation(false)
    expect(state.perPage.value).toBe(2)
    expect(state.settings.layout).toBe('2x1')
    expect(state.settings.fixed_width_mm).toBe(165)

    // 应用当前布局智能尺寸不改张数
    state.setImagesPerPage(4)
    const beforeLayout = state.settings.layout
    state.applyCurrentLayoutRecommendation(false)
    expect(state.settings.layout).toBe(beforeLayout)
    expect(state.settings.fixed_width_mm).toBe(state.layoutAwareRecommendation.value.recommended_width_mm)
    expect(state.settings.fixed_width_mm).toBeLessThan(165)
  })

  it('exports current page scale immediately (preview == export) and inherits global on new pages', async () => {
    const state = useImagePaddlerState()
    state.folders.value = ['/images']
    state.analysis.value = { images }
    state.settings.layout = '2x1'
    state.setGlobalScale(80)

    // 当前页调到 75%，无需点保存即进入导出参数
    state.activePageScale.value = 75
    tauriCallSafe.mockResolvedValue({ ok: true, data: { images: 6, pages: 3 } })
    await state.run()
    const payload = tauriCallSafe.mock.calls[0][1].args
    expect(payload.page_scales[0]).toBeCloseTo(0.75)
    expect(payload.page_scales[1]).toBeCloseTo(0.8)
    expect(payload.page_scales[2]).toBeCloseTo(0.8)
  })

  it('caption gap participates in layout metrics', () => {
    const state = useImagePaddlerState()
    state.analysis.value = { images: images.slice(0, 2) }
    state.settings.layout = '2x1'
    state.settings.show_filename = true
    state.settings.caption_gap_mm = 0
    const reserve0 = state.layoutMetrics.value.filenameReserve
    state.settings.caption_gap_mm = 8
    expect(state.layoutMetrics.value.captionGapMm).toBe(8)
    expect(state.layoutMetrics.value.filenameReserve).toBeGreaterThan(reserve0)
    expect(state.layoutMetrics.value.imageCellHeight).toBeLessThan(state.layoutMetrics.value.cellHeight)
  })
})

