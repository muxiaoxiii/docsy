import { describe, expect, it } from 'vitest'
import {
  arrangeOptionsFor,
  controlsFromLayout,
  importRecommendationDrifts,
  layoutFromControls,
  layoutSlotNumbers,
  parseLayoutString,
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
    expect(controlsFromLayout('1x4', 2, 2, false).arrangeMode).toBe('side')
    expect(controlsFromLayout('4x1', 2, 2, false).arrangeMode).toBe('stack')
  })

  it('places consecutive inputs down columns for N order, including a partial page', () => {
    expect(layoutSlotNumbers({ rows: 2, cols: 3 }, 'n')).toEqual([1, 3, 5, 2, 4, 6])
    expect(layoutSlotNumbers({ rows: 2, cols: 3 }, 'reverse_n')).toEqual([5, 3, 1, 6, 4, 2])
    expect(layoutSlotNumbers({ rows: 2, cols: 3 }, 'n', 4)).toEqual([1, 3, 4, 2])
  })

  it('parses custom and legacy count layouts', () => {
    expect(parseLayoutString('custom', 2, 3)).toEqual({ rows: 2, cols: 3 })
    expect(parseLayoutString('4')).toEqual({ rows: 2, cols: 2 })
  })

  it('detects import recommendation drift vs current grid', () => {
    const importRec = { layout: '2x1' }
    expect(importRecommendationDrifts(importRec, { rows: 2, cols: 1 }, 2)).toBe(false)
    expect(importRecommendationDrifts(importRec, { rows: 3, cols: 3 }, 9)).toBe(true)
  })
})
