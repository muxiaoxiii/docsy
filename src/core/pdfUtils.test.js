import { describe, expect, it } from 'vitest'
import { insertRangeAtPage, parsePageSelection, smartSetRangeEnd, smartSetRangeStart } from './pdfUtils.js'

function segment(start, end, name = '') {
  return { name, pageStart: start, pageEnd: end }
}

function ranges(items) {
  return items.map((item) => [item.pageStart, item.pageEnd])
}

describe('PDF utility helpers', () => {
  it('parses comma separated pages and ranges', () => {
    expect(parsePageSelection('3,7,12-15', 20)).toEqual([3, 7, 12, 13, 14, 15])
  })

  it('deduplicates pages while preserving order', () => {
    expect(parsePageSelection('2,1-3,3', 10)).toEqual([2, 1, 3])
  })

  it('rejects invalid or out-of-bounds input', () => {
    expect(parsePageSelection('5-2', 10)).toEqual([])
    expect(parsePageSelection('1,12', 10)).toEqual([])
    expect(parsePageSelection('abc', 10)).toEqual([])
  })
})

describe('smartSetRangeStart', () => {
  it('keeps previous segment contiguous when start moves forward', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    smartSetRangeStart(items, 1, 8, 20)
    expect(ranges(items)).toEqual([
      [1, 7],
      [8, 10],
      [11, 20],
    ])
    expect(items[1].name).toBe('B')
  })

  it('steals pages from previous segment when start moves backward', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    smartSetRangeStart(items, 1, 4, 20)
    expect(ranges(items)).toEqual([
      [1, 3],
      [4, 10],
      [11, 20],
    ])
  })

  it('absorbs previous segment entirely when start reaches its first page', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    const index = smartSetRangeStart(items, 1, 1, 20)
    expect(index).toBe(0)
    expect(ranges(items)).toEqual([
      [1, 10],
      [11, 20],
    ])
    expect(items[0].name).toBe('B')
  })

  it('clamps start to the first page', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B')]
    smartSetRangeStart(items, 1, 0, 10)
    expect(ranges(items)).toEqual([[1, 10]])
    expect(items[0].name).toBe('B')
  })
})

describe('smartSetRangeEnd', () => {
  it('keeps next segment contiguous when end moves backward', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    smartSetRangeEnd(items, 1, 8, 20)
    expect(ranges(items)).toEqual([
      [1, 5],
      [6, 8],
      [9, 20],
    ])
  })

  it('absorbs next segment entirely when end reaches its last page', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    smartSetRangeEnd(items, 0, 10, 20)
    expect(ranges(items)).toEqual([
      [1, 10],
      [11, 20],
    ])
  })

  it('absorbs multiple following segments when end extends far', () => {
    const items = [segment(1, 5, 'A'), segment(6, 7, 'B'), segment(8, 10, 'C'), segment(11, 20, 'D')]
    smartSetRangeEnd(items, 0, 15, 20)
    expect(ranges(items)).toEqual([
      [1, 15],
      [16, 20],
    ])
  })

  it('clamps end to total pages', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B')]
    smartSetRangeEnd(items, 0, 99, 10)
    expect(ranges(items)).toEqual([[1, 10]])
    expect(items.length).toBe(1)
  })
})

describe('insertRangeAtPage', () => {
  it('extracts one page at a boundary between two segments', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B'), segment(11, 20, 'C')]
    const index = insertRangeAtPage(items, 6, 20, { name: '新段', extra: { source: 'manual' } })
    expect(index).toBe(1)
    expect(items.map((item) => [item.name, item.pageStart, item.pageEnd])).toEqual([
      ['A', 1, 5],
      ['新段', 6, 6],
      ['B', 7, 10],
      ['C', 11, 20],
    ])
    expect(items[1].source).toBe('manual')
  })

  it('extracts the first page as its own segment', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B')]
    insertRangeAtPage(items, 1, 10, { name: '目录' })
    expect(ranges(items)).toEqual([
      [1, 1],
      [2, 5],
      [6, 10],
    ])
    expect(items[0].name).toBe('目录')
  })

  it('reuses an existing boundary for quick split commands', () => {
    const items = [segment(1, 5, 'A'), segment(6, 10, 'B')]
    const index = insertRangeAtPage(items, 6, 10, { name: '新段', reuseBoundary: true })
    expect(index).toBe(1)
    expect(items).toEqual([segment(1, 5, 'A'), segment(6, 10, 'B')])
  })

  it('splits inside a segment: new segment takes the tail', () => {
    const items = [segment(1, 10, 'A'), segment(11, 20, 'B')]
    const index = insertRangeAtPage(items, 7, 20, { name: '尾部' })
    expect(index).toBe(1)
    expect(items.map((item) => [item.name, item.pageStart, item.pageEnd])).toEqual([
      ['A', 1, 6],
      ['尾部', 7, 10],
      ['B', 11, 20],
    ])
  })

  it('new segment at the tail of the last segment ends at the last page', () => {
    const items = [segment(1, 20, 'A')]
    const index = insertRangeAtPage(items, 15, 20, { name: '尾部' })
    expect(index).toBe(1)
    expect(ranges(items)).toEqual([
      [1, 14],
      [15, 20],
    ])
  })

  it('does nothing when the page is already a single-page segment', () => {
    const items = [segment(1, 1, '目录'), segment(2, 10, 'A')]
    const index = insertRangeAtPage(items, 1, 10, { name: '续段' })
    expect(index).toBe(-1)
    expect(items.length).toBe(2)
  })

  it('returns -1 when the page is the last single-page segment', () => {
    const items = [segment(1, 10, 'A'), segment(11, 11, 'B')]
    const index = insertRangeAtPage(items, 11, 11, { name: '新段' })
    expect(index).toBe(-1)
    expect(items.length).toBe(2)
  })

  it('fills an empty plan with a full-range segment', () => {
    const items = []
    const index = insertRangeAtPage(items, 3, 20, { name: '全文' })
    expect(index).toBe(0)
    expect(ranges(items)).toEqual([[3, 20]])
  })
})
