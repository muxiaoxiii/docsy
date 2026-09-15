import { describe, expect, it } from 'vitest'
import { safeImageWidth, effectiveImageWidth, layoutOptionLabel } from './layoutPreview.js'

describe('image layout preview', () => {
  it('changes actual width below the limit and exposes the cap', () => {
    const maximum = safeImageWidth([{ width: 100, height: 200 }], 186, 100)
    expect(maximum).toBe(50)
    expect(effectiveImageWidth(30, maximum)).toBe(30)
    expect(effectiveImageWidth(40, maximum)).toBe(40)
    expect(effectiveImageWidth(160, maximum)).toBe(50)
  })
  it('respects column width and the tallest image across the whole document', () => {
    expect(safeImageWidth([{ width: 100, height: 100 }], 62, 250)).toBe(62)
    expect(
      safeImageWidth(
        [
          { width: 100, height: 50 },
          { width: 100, height: 400 },
        ],
        186,
        100,
      ),
    ).toBe(25)
  })
  it('never promises horizontal layout in paragraph mode', () => {
    expect(layoutOptionLabel('1x3', false)).toBe('3 张（左右）')
    expect(layoutOptionLabel('1x3', true)).toBe('3 张（上下）')
    expect(layoutOptionLabel('custom', true)).toContain('上下')
  })
})
