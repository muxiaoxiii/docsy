import { describe, expect, it } from 'vitest'
import {
  safeImageWidth,
  safeColumnWidth,
  effectiveImageWidth,
  effectivePageWidth,
  pairOffsets,
  pairAlignToXY,
  detectPageConflicts,
  computeOptimalPageScale,
  layoutOptionLabel,
} from './layoutPreview.js'

describe('image layout preview', () => {
  it('changes actual width below the limit and exposes the cap', () => {
    const maximum = safeImageWidth([{ width: 100, height: 200 }], 186, 100)
    expect(maximum).toBe(50)
    expect(effectiveImageWidth(30, maximum)).toBe(30)
    expect(effectiveImageWidth(40, maximum)).toBe(40)
    expect(effectiveImageWidth(160, maximum)).toBe(50)
  })

  it('calculates safeColumnWidth correctly', () => {
    // A4 竖向 210mm, margin 12mm, 1 列, pad 6mm -> (210 - 24)/1 - 6 = 180mm
    expect(safeColumnWidth(210, 12, 1, 6)).toBe(180)
    // A4 竖向 210mm, margin 12mm, 2 列, pad 6mm -> (210 - 24)/2 - 6 = 93 - 6 = 87mm
    expect(safeColumnWidth(210, 12, 2, 6)).toBe(87)
    // 最小保护 20mm
    expect(safeColumnWidth(50, 20, 2, 6)).toBe(20)
  })

  it('scales effectivePageWidth by pageScale', () => {
    expect(effectivePageWidth(100, 1.0, 180)).toBe(100)
    expect(effectivePageWidth(100, 1.2, 180)).toBe(120)
    expect(effectivePageWidth(100, 0.8, 180)).toBe(80)
  })

  it('computes pairOffsets for 2x1 and 1x2 modes correctly', () => {
    expect(pairOffsets(2, 2, 1, 'page-gather')).toEqual(['bottom', 'top'])
    expect(pairOffsets(2, 2, 1, 'page-spread')).toEqual(['top', 'bottom'])
    expect(pairOffsets(2, 1, 2, 'page-gather')).toEqual(['right', 'left'])
    expect(pairOffsets(2, 1, 2, 'page-spread')).toEqual(['left', 'right'])
    expect(pairOffsets(2, 2, 1, 'cell-center')).toEqual(['center', 'center'])
    // 4 张图时不触发 pair_mode
    expect(pairOffsets(4, 2, 2, 'page-gather')).toEqual(['center', 'center', 'center', 'center'])
  })

  it('detects page conflicts accurately', () => {
    // 正常图片（4:3 比例在 0.55~1.55 之间，不超界）
    const okResult = detectPageConflicts({
      images: [{ path: 'test.png', width: 1000, height: 750 }],
      grid: { rows: 2, cols: 1 },
      cellWidth: 186,
      imageCellHeight: 120,
      fixedWidthMm: 140,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
    })
    expect(okResult.items[0].color).toBe('ok')

    // 极端长图超出格高
    const overflowResult = detectPageConflicts({
      images: [{ path: 'long.png', width: 500, height: 1500 }],
      grid: { rows: 2, cols: 1 },
      cellWidth: 186,
      imageCellHeight: 100,
      fixedWidthMm: 150,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
    })
    expect(overflowResult.items[0].overflow).toBe(true)
    expect(['green', 'yellow', 'red']).toContain(overflowResult.items[0].color)
  })

  it('never promises horizontal layout in paragraph mode', () => {
    expect(layoutOptionLabel('1x3', false)).toBe('3 张（左右）')
    expect(layoutOptionLabel('1x3', true)).toBe('3 张（上下）')
    expect(layoutOptionLabel('custom', true)).toContain('上下')
  })

  it('golden fixture: 2x2 with pageScale 1.2 scales all 4 cells uniformly', () => {
    const fourImages = Array.from({ length: 4 }, (_, i) => ({
      path: `img_${i}.png`,
      width: 800,
      height: 600,
    }))
    const result1x = detectPageConflicts({
      images: fourImages,
      grid: { rows: 2, cols: 2 },
      cellWidth: 90,
      imageCellHeight: 120,
      fixedWidthMm: 80,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
    })
    expect(result1x.items.length).toBe(4)
    expect(result1x.items[0].drawW).toBe(80)

    const result1_2x = detectPageConflicts({
      images: fourImages,
      grid: { rows: 2, cols: 2 },
      cellWidth: 90,
      imageCellHeight: 120,
      fixedWidthMm: 80,
      pageScale: 1.2,
      scaleMode: 'fixed_width',
    })
    // 80 * 1.2 = 96 > cellWidth 90 => overflow
    expect(result1_2x.items[0].drawW).toBeCloseTo(96)
    expect(result1_2x.items[0].overflow).toBe(true)
    expect(result1_2x.hasOverflow).toBe(true)
  })

  it('golden fixture: 3x3 with filenames detects caption overlap and density warnings', () => {
    const nineImages = Array.from({ length: 9 }, (_, i) => ({
      path: `sample_photo_${i}.jpg`,
      name: `测试图片名称_${i}.jpg`,
      width: 600,
      height: 800, // 竖长图
    }))
    const result = detectPageConflicts({
      images: nineImages,
      grid: { rows: 3, cols: 3 },
      cellWidth: 60,
      imageCellHeight: 65,
      fixedWidthMm: 55,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
      showFilename: true,
      captionReserveMm: 14,
      captionPosition: 'below',
    })
    expect(result.items.length).toBe(9)
    // 竖长图 55 * (800/600) = 73.3mm > imgAreaH (65) => 探入标题带并超界
    expect(result.items[0].drawH).toBeCloseTo(73.33, 1)
    expect(result.items[0].overflow).toBe(true)
  })

  it('computeOptimalPageScale computes safe scale for overflowing tall images', () => {
    // 竖长图 600x800, fixedWidth 55, cellWidth 60, imageCellHeight 65
    // 55 * (800/600) = 73.33 > 65 => overflows at 100%
    // To fit height 65: scale <= 65 / 73.333 = 0.886 -> 88%
    const nineImages = Array.from({ length: 9 }, (_, i) => ({
      path: `sample_photo_${i}.jpg`,
      name: `测试图片名称_${i}.jpg`,
      width: 600,
      height: 800,
    }))
    const optimal = computeOptimalPageScale({
      images: nineImages,
      grid: { rows: 3, cols: 3 },
      cellWidth: 60,
      imageCellHeight: 65,
      fixedWidthMm: 55,
      scaleMode: 'fixed_width',
      showFilename: true,
      captionReserveMm: 14,
      captionPosition: 'below',
    })
    expect(optimal.ok).toBe(true)
    expect(optimal.scale).toBeLessThanOrEqual(89)
    expect(optimal.scale).toBeGreaterThanOrEqual(87)

    // At the computed optimal scale, there should be no overflow or overlap
    const verification = detectPageConflicts({
      images: nineImages,
      grid: { rows: 3, cols: 3 },
      cellWidth: 60,
      imageCellHeight: 65,
      fixedWidthMm: 55,
      pageScale: optimal.scale / 100,
      scaleMode: 'fixed_width',
      showFilename: true,
      captionReserveMm: 14,
      captionPosition: 'below',
    })
    expect(verification.hasOverflow).toBe(false)
    expect(verification.hasOverlap).toBe(false)
  })

  it('computeOptimalPageScale returns 100 if 100 is already conflict-free', () => {
    const squareImages = [
      { path: 'sq1.png', width: 500, height: 500 },
      { path: 'sq2.png', width: 500, height: 500 },
    ]
    const optimal = computeOptimalPageScale({
      images: squareImages,
      grid: { rows: 2, cols: 1 },
      cellWidth: 180,
      imageCellHeight: 120,
      fixedWidthMm: 100,
      scaleMode: 'fixed_width',
    })
    expect(optimal.ok).toBe(true)
    expect(optimal.scale).toBe(100)
  })

  it('maps pairAlignToXY and flex styles correctly for pair alignment', () => {
    expect(pairAlignToXY('bottom')).toEqual({ x: 'center', y: 'bottom' })
    expect(pairAlignToXY('top')).toEqual({ x: 'center', y: 'top' })
    expect(pairAlignToXY('left')).toEqual({ x: 'left', y: 'center' })
    expect(pairAlignToXY('right')).toEqual({ x: 'right', y: 'center' })
    expect(pairAlignToXY('center')).toEqual({ x: 'center', y: 'center' })

    const justifyMap = { left: 'flex-start', center: 'center', right: 'flex-end' }
    const alignMap = { top: 'flex-start', center: 'center', bottom: 'flex-end' }

    // 2x1 page-gather
    const b = pairAlignToXY('bottom')
    expect(justifyMap[b.x]).toBe('center')
    expect(alignMap[b.y]).toBe('flex-end')

    const t = pairAlignToXY('top')
    expect(justifyMap[t.x]).toBe('center')
    expect(alignMap[t.y]).toBe('flex-start')
  })

  it('matches Rust backend golden fixture rectangles exactly (<0.05mm tolerance)', () => {
    const report = detectPageConflicts({
      images: [
        { path: 'img0.png', width: 1600, height: 1200 },
        { path: 'img1.png', width: 1600, height: 1200 },
      ],
      grid: { rows: 2, cols: 1 },
      cellWidth: 186.0,
      imageCellHeight: 131.3,
      fixedWidthMm: 160.0,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
      dpi: 300,
      pageWidth: 210,
      pageHeight: 297,
      marginMm: 12.0,
      showFilename: true,
      captionPosition: 'below',
      captionReserveMm: 5.2,
      pairMode: 'page-gather',
    })

    const boxes = report.imageBoxes
    expect(boxes).toHaveLength(2)

    // Image 0: drawW=160, drawH=120, gathered to bottom of cell 0
    expect(Math.abs(boxes[0].w - 160.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].h - 120.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].x - 25.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].y - 23.3)).toBeLessThan(0.05)

    // Image 1: drawW=160, drawH=120, gathered to top of cell 1
    expect(Math.abs(boxes[1].w - 160.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].h - 120.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].x - 25.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].y - 148.5)).toBeLessThan(0.05)
  })

  it('matches Rust backend golden fixture: 2x1 page-spread with pageScale 1.25 and annotations (<0.05mm)', () => {
    const report = detectPageConflicts({
      images: [
        { path: 'img0.png', width: 1500, height: 1000, title: '标题 1', description: '说明文字 1' },
        { path: 'img1.png', width: 1500, height: 1000, title: '标题 2', description: '说明文字 2' },
      ],
      grid: { rows: 2, cols: 1 },
      cellWidth: 186.0,
      imageCellHeight: 131.3,
      fixedWidthMm: 120.0,
      pageScale: 1.25,
      scaleMode: 'fixed_width',
      dpi: 300,
      pageWidth: 210,
      pageHeight: 297,
      marginMm: 12.0,
      showFilename: true,
      captionPosition: 'below',
      captionReserveMm: 5.2,
      pairMode: 'page-spread',
    })

    const boxes = report.imageBoxes
    expect(boxes).toHaveLength(2)

    // Image 0: drawW=150, drawH=100, top-aligned in cell 0
    expect(Math.abs(boxes[0].w - 150.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].h - 100.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].x - 30.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].y - 12.0)).toBeLessThan(0.05)

    // Image 1: drawW=150, drawH=100, bottom-aligned in cell 1
    expect(Math.abs(boxes[1].w - 150.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].h - 100.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].x - 30.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].y - 179.8)).toBeLessThan(0.05)
  })

  it('matches Rust backend golden fixture: 1x2 side-by-side with caption above (<0.05mm)', () => {
    const report = detectPageConflicts({
      images: [
        { path: 'left.png', width: 800, height: 1200 },
        { path: 'right.png', width: 800, height: 1200 },
      ],
      grid: { rows: 1, cols: 2 },
      cellWidth: 93.0,
      imageCellHeight: 265.0,
      fixedWidthMm: 80.0,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
      dpi: 300,
      pageWidth: 210,
      pageHeight: 297,
      marginMm: 12.0,
      showFilename: true,
      captionPosition: 'above',
      captionReserveMm: 8.0,
      pairMode: 'page-gather',
    })

    const boxes = report.imageBoxes
    expect(boxes).toHaveLength(2)

    // Image 0: right-aligned in col 0
    expect(Math.abs(boxes[0].w - 80.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].h - 120.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].x - 25.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[0].y - 92.5)).toBeLessThan(0.05)

    // Image 1: left-aligned in col 1
    expect(Math.abs(boxes[1].w - 80.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].h - 120.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].x - 105.0)).toBeLessThan(0.05)
    expect(Math.abs(boxes[1].y - 92.5)).toBeLessThan(0.05)
  })

  it('extracts basename for caption width to prevent long directory paths from triggering overlap', () => {
    const report = detectPageConflicts({
      images: [
        {
          path: '/very/long/nested/directory/path/that/used/to/cause/massive/false/caption/width/overflow/img.png',
          width: 1000,
          height: 1000,
        },
      ],
      grid: { rows: 1, cols: 1 },
      cellWidth: 186.0,
      imageCellHeight: 150.0,
      fixedWidthMm: 100.0,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
      marginMm: 12.0,
      showFilename: true,
      captionReserveMm: 8.0,
      fontSizePt: 8,
    })
    expect(report.captionBoxes[0].w).toBeLessThan(50)
  })

  it('detects caption width based on custom title and description annotations', () => {
    const report = detectPageConflicts({
      images: [
        {
          path: '/path/to/short.png',
          title: '自定义非常长长长长长长长长的图片标题文字',
          description: '现场见证人：李四；拍摄地点：北京市海淀区中关村南大街1号',
          width: 800,
          height: 600,
        },
      ],
      grid: { rows: 1, cols: 1 },
      cellWidth: 186.0,
      imageCellHeight: 150.0,
      fixedWidthMm: 100.0,
      pageScale: 1.0,
      scaleMode: 'fixed_width',
      marginMm: 12.0,
      showFilename: true,
      captionReserveMm: 12.0,
      fontSizePt: 8,
      noteFontSizePt: 8,
    })
    expect(report.captionBoxes.length).toBe(1)
    // Custom description and title are substantially wider than short.png
    expect(report.captionBoxes[0].w).toBeGreaterThan(60)
  })

  it('returns failed result when 30% scale still has conflicts due to extreme image dimensions', () => {
    const optimal = computeOptimalPageScale({
      images: [{ width: 200, height: 4000 }], // Extreme 1:20 portrait image
      grid: { rows: 1, cols: 1 },
      cellWidth: 100,
      imageCellHeight: 80,
      fixedWidthMm: 90,
      scaleMode: 'fixed_width',
      dpi: 300,
      pageWidth: 210,
      pageHeight: 297,
      marginMm: 12,
      showFilename: false,
      captionPosition: 'below',
      captionReserveMm: 0,
      fontSizePt: 8,
      pairMode: 'cell-center',
    })
    expect(optimal.ok).toBe(false)
    expect(optimal.scale).toBeNull()
    expect(optimal.reason).toContain('30%')
  })
})



it('marks both the encroaching portrait and the landscape whose caption is covered', () => {
  const report = detectPageConflicts({
    images: [
      { path: 'landscape.png', width: 1920, height: 1080, captionHeightMm: 5 },
      { path: 'portrait.png', width: 1080, height: 1920, captionHeightMm: 5 },
    ],
    grid: { rows: 2, cols: 1 }, cellWidth: 186, imageCellHeight: 129,
    fixedWidthMm: 160, pageScale: 0.8, captionPosition: 'below', captionReserveMm: 7.5, captionGapMm: 2,
  })
  expect(report.items[0].captionOverlap).toBe(true)
  expect(report.items[1].captionOverlap).toBe(true)
})
