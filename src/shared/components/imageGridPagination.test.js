import { describe, expect, it } from 'vitest'
import { pageRangeForSize, pageSizeForFraction, percentagePageSizeOptions } from './imageGridPagination.js'

describe('pageRangeForSize', () => {
  it('uses the selected percentage-derived count as the actual page size', () => {
    expect(pageRangeForSize(97, 25, 1)).toMatchObject({ pageCount: 4, start: 0, end: 25, size: 25 })
    expect(pageRangeForSize(97, 25, 4)).toMatchObject({ pageCount: 4, start: 75, end: 97, size: 22 })
  })

  it('keeps all four percentage choices meaningful', () => {
    expect(pageRangeForSize(97, 49, 1)).toMatchObject({ pageCount: 2, end: 49 })
    expect(pageRangeForSize(97, 73, 1)).toMatchObject({ pageCount: 2, end: 73 })
    expect(pageRangeForSize(97, 97, 1)).toMatchObject({ pageCount: 1, end: 97 })
  })

  it('clamps invalid and out-of-range pages', () => {
    expect(pageRangeForSize(12, 6, 99)).toMatchObject({ page: 2, start: 6, end: 12 })
    expect(pageRangeForSize(0, 0, 0)).toMatchObject({ page: 1, pageCount: 1, start: 0, end: 0 })
  })
})

describe('percentagePageSizeOptions', () => {
  it('shows only the calculated image counts for 25%, 50%, 75% and 100%', () => {
    expect(percentagePageSizeOptions(97)).toEqual([
      { fraction: 0.25, size: 25, label: '25 张' },
      { fraction: 0.5, size: 49, label: '49 张' },
      { fraction: 0.75, size: 73, label: '73 张' },
      { fraction: 1, size: 97, label: '97 张' },
    ])
  })

  it('removes duplicate counts for small image sets', () => {
    expect(percentagePageSizeOptions(2).map((option) => option.label)).toEqual(['1 张', '2 张'])
  })

  it('maps a saved percentage to the nearest available count', () => {
    expect(pageSizeForFraction(97, 0.25)).toBe(25)
    expect(pageSizeForFraction(97, 0.76)).toBe(73)
  })
})
