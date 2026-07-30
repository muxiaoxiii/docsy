import { describe, expect, it } from 'vitest'
import { parsePageSelection } from './pdfUtils.js'

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
