import { describe, expect, it } from 'vitest'
import { batchSplitRanges, validRangeSubdivision } from './batchSplitRanges.js'

describe('batch range subdivision', () => {
  const source = { name: '证据', pageStart: 6, pageEnd: 890 }
  it('divides a range into the requested number without losing or duplicating pages', () => {
    for (const count of [2, 3, 100, 885]) {
      const ranges = batchSplitRanges(source, { count })
      expect(ranges).toHaveLength(count)
      expect(validRangeSubdivision(source, ranges)).toBe(true)
      expect(
        Math.max(...ranges.map((r) => r.pageEnd - r.pageStart)) -
          Math.min(...ranges.map((r) => r.pageEnd - r.pageStart)),
      ).toBeLessThanOrEqual(1)
    }
  })
  it('uses page counts with a final shorter interval', () => {
    const ranges = batchSplitRanges({ ...source, pageEnd: 32 }, { mode: 'pages', pages: 10 })
    expect(ranges.map((r) => [r.pageStart, r.pageEnd])).toEqual([
      [6, 15],
      [16, 25],
      [26, 32],
    ])
  })
  it('accepts a list and preserves all remaining pages as the final segment', () => {
    const ranges = batchSplitRanges(source, { mode: 'lengths', lengths: '1，4\n27', prefix: '材料' })
    expect(ranges.map((r) => [r.pageStart, r.pageEnd])).toEqual([
      [6, 6],
      [7, 10],
      [11, 37],
      [38, 890],
    ])
    expect(ranges[0].name).toBe('材料-1')
    expect(validRangeSubdivision(source, ranges)).toBe(true)
  })
  it('rejects invalid counts and subdivisions instead of overlapping the last page', () => {
    expect(() => batchSplitRanges({ pageStart: 890, pageEnd: 890 }, { count: 2 })).toThrow()
    expect(() => batchSplitRanges(source, { mode: 'lengths', lengths: '0,3' })).toThrow()
    expect(() => batchSplitRanges(source, { mode: 'lengths', lengths: '886' })).toThrow()
    expect(
      validRangeSubdivision(source, [
        { pageStart: 6, pageEnd: 20 },
        { pageStart: 20, pageEnd: 890 },
      ]),
    ).toBe(false)
    expect(source.pageEnd).toBe(890)
  })
})

it('adds omitted evidence boundaries by their exact PDF starts, retaining the original first name', () => {
  const source = { name: '证据7', pageStart: 112, pageEnd: 306 }
  const rows = batchSplitRanges(source, { mode: 'starts', starts: '262, 208, 208', prefix: '新增证据' })
  expect(rows.map((row) => [row.name, row.pageStart, row.pageEnd])).toEqual([
    ['证据7', 112, 207],
    ['新增证据1', 208, 261],
    ['新增证据2', 262, 306],
  ])
  expect(source).toEqual({ name: '证据7', pageStart: 112, pageEnd: 306 })
  expect(() => batchSplitRanges(source, { mode: 'starts', starts: '112' })).toThrow()
  expect(() => batchSplitRanges(source, { mode: 'starts', starts: '400' })).toThrow()
})
