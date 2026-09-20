import { describe, expect, it } from 'vitest'
import {
  arrangeOptionsFor,
  controlsFromLayout,
  layoutFromControls,
  parseLayoutString,
  recommendForLayout,
} from './layoutControls.js'

describe('layoutControls', () => {
  it('maps per-page count + arrange mode to backend layout strings', () => {
    expect(layoutFromControls({ imagesPerPage: 2, arrangeMode: 'stack' }).layout).toBe('2x1')
    expect(layoutFromControls({ imagesPerPage: 2, arrangeMode: 'side' }).layout).toBe('1x2')
    expect(layoutFromControls({ imagesPerPage: 4, arrangeMode: 'grid' }).layout).toBe('4')
    expect(layoutFromControls({ imagesPerPage: 9, arrangeMode: 'grid' }).layout).toBe('3x3')
    expect(layoutFromControls({ imagesPerPage: 3, arrangeMode: 'stack' }).layout).toBe('3x1')
    expect(layoutFromControls({ imagesPerPage: 6, arrangeMode: 'flow', flow: true }).layout).toBe('6')
  })

  it('keeps flow mode options vertical only', () => {
    const flowOptions = arrangeOptionsFor(4, true)
    expect(flowOptions).toHaveLength(1)
    expect(flowOptions[0].value).toBe('stack')
    const tableOptions = arrangeOptionsFor(4, false)
    expect(tableOptions.map((o) => o.value)).toContain('grid')
  })

  it('round-trips controls from layout strings', () => {
    expect(controlsFromLayout('2x1', 2, 2, false)).toEqual({ imagesPerPage: 2, arrangeMode: 'stack' })
    expect(controlsFromLayout('1x2', 2, 2, false)).toEqual({ imagesPerPage: 2, arrangeMode: 'side' })
    expect(controlsFromLayout('4', 2, 2, false).imagesPerPage).toBe(4)
  })

  it('recommendForLayout shrinks unified width when per-page grid gets denser', () => {
    const images = Array.from({ length: 9 }, (_, i) => ({
      path: `/img/${i}.png`,
      width: 1920,
      height: 1080,
    }))
    const two = recommendForLayout({
      images,
      grid: parseLayoutString('2x1'),
      orientation: 'portrait',
      perPage: 2,
    })
    const nine = recommendForLayout({
      images,
      grid: parseLayoutString('3x3'),
      orientation: 'portrait',
      perPage: 9,
    })
    expect(nine.recommended_width_mm).toBeLessThan(two.recommended_width_mm)
    expect(nine.recommended_width_mm).toBeLessThanOrEqual(nine.safe_column_width_mm + 1)
  })

  it('recommendForLayout respects tall images via height constraint not only column width', () => {
    const squares = [
      { path: '/s1.png', width: 1000, height: 1000 },
      { path: '/s2.png', width: 1000, height: 1000 },
    ]
    const rec = recommendForLayout({
      images: squares,
      grid: { rows: 2, cols: 1 },
      orientation: 'portrait',
      perPage: 2,
      showFilename: true,
    })
    // 两张正方形上下排：宽度不能超过格高，否则溢出
    expect(rec.recommended_width_mm).toBeLessThanOrEqual(rec.image_cell_height_mm + 1)
    expect(rec.recommended_width_mm).toBeLessThan(180)
  })
})
