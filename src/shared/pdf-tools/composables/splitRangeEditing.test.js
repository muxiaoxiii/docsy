import { describe, expect, it } from 'vitest'
import { editSplitRangeBoundary, splitRangeRowKey, unassignedSplitRanges } from './splitRangeEditing.js'
import { splitRangeWarnings } from './usePdfSplitRanges.js'

const fixture = () => [
  { name: '甲', pageStart: 1, pageEnd: 4 },
  { name: '乙', pageStart: 5, pageEnd: 10 },
  { name: '丙', pageStart: 11, pageEnd: 20 },
]
describe('non-destructive manual boundary editing', () => {
  it('never consumes, inserts or reorders rows when a boundary crosses multiple segments', () => {
    for (const boundary of ['start', 'end'])
      for (const page of [1, 3, 5, 8, 12, 20]) {
        const rows = fixture(),
          original = [...rows],
          untouched = [{ ...rows[0] }, { ...rows[2] }]
        const ids = rows.map(splitRangeRowKey)
        expect(editSplitRangeBoundary(rows, 1, page, 20, boundary)).toBe(1)
        expect(rows).toHaveLength(3)
        rows.forEach((row, index) => expect(row).toBe(original[index]))
        expect(rows.map(splitRangeRowKey)).toEqual(ids)
        expect(rows[0]).toMatchObject(untouched[0])
        expect(rows[2]).toMatchObject(untouched[1])
        expect(rows[1][boundary === 'start' ? 'pageStart' : 'pageEnd']).toBe(page)
      }
  })
  it('reports temporary overlaps, gaps and reversed endpoints instead of silently repairing them', () => {
    const rows = fixture()
    editSplitRangeBoundary(rows, 1, 18, 20, 'start')
    expect(splitRangeWarnings(rows, 20)).toContain('页段「乙」起始页大于结束页')
    editSplitRangeBoundary(rows, 1, 20, 20, 'end')
    expect(rows.map((row) => row.name)).toEqual(['甲', '乙', '丙'])
    expect(splitRangeWarnings(rows, 20).some((message) => message.includes('重叠'))).toBe(true)
    expect(unassignedSplitRanges(rows, 20)).toEqual([{ pageStart: 5, pageEnd: 10 }])
  })
  it('keeps the visible output order independent from editable page numbers', () => {
    const rows = fixture().reverse(),
      original = [...rows]
    editSplitRangeBoundary(rows, 1, 1, 20, 'start')
    expect(rows.map((row) => row.name)).toEqual(['丙', '乙', '甲'])
    rows.forEach((row, index) => expect(row).toBe(original[index]))
  })
})
