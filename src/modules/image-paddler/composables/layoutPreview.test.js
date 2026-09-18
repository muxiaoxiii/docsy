import { describe, expect, it } from 'vitest'
import {
  safeImageWidth,
  safeColumnWidth,
  effectiveImageWidth,
  effectivePageWidth,
  pairOffsets,
  detectPageConflicts,
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
})
