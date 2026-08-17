import { describe, expect, it } from 'vitest'
import {
  compareDetectedTextRows,
  detectedElementFromCandidate,
  mergeExistingElements,
  mergeRangeSelection,
} from './existingPdfElements.js'

const candidate = {
  text: '1/3',
  normalizedText: '{page}/{total}',
  region: 'header',
  pageRange: { start: 1, end: 3 },
  bbox: { x0: 500, y0: 10, x1: 540, y1: 20, page: 1 },
  source: 'content-text',
}

describe('existingPdfElements', () => {
  it('keeps a top page number classified as a page number', () => {
    const element = detectedElementFromCandidate(candidate, 'pageNumber')
    expect(element.kind).toBe('pageNumber')
    expect(element.pageStart).toBe(1)
  })

  it('preserves a user decision when detection is refreshed', () => {
    const old = { ...detectedElementFromCandidate(candidate, 'pageNumber'), decision: 'ignore' }
    const refreshed = detectedElementFromCandidate({ ...candidate, confidence: 1 }, 'pageNumber')
    expect(mergeExistingElements([old], [refreshed])[0].decision).toBe('ignore')
  })

  it('sorts latin-led detected texts first and keeps 证据X families adjacent', () => {
    const row = (text, fileName = 'a.pdf', pageStart = 1) => ({
      fileName,
      element: { detectedText: text, pageStart },
    })
    const sorted = [
      row('证据2', 'b.pdf'),
      row('CN 105829563 A', 'c.pdf'),
      row('证据1译文', 'a.pdf'),
      row('证据1', 'a.pdf'),
      row('JP 2017-186663 A 2017.10.12', 'a.pdf'),
      row('证据2译文', 'b.pdf'),
    ].sort(compareDetectedTextRows)
    expect(sorted.map((r) => r.element.detectedText)).toEqual([
      'CN 105829563 A',
      'JP 2017-186663 A 2017.10.12',
      '证据1',
      '证据1译文',
      '证据2',
      '证据2译文',
    ])
  })

  it('orders 证据 numbers numerically rather than lexicographically', () => {
    const row = (text) => ({ fileName: 'a.pdf', element: { detectedText: text, pageStart: 1 } })
    const sorted = [row('证据10'), row('证据2'), row('证据1')].sort(compareDetectedTextRows)
    expect(sorted.map((r) => r.element.detectedText)).toEqual(['证据1', '证据2', '证据10'])
  })

  it('keeps both shift-selection endpoints selected', () => {
    const rows = ['a', 'b', 'c', 'd', 'e'].map((key) => ({ key }))
    expect(mergeRangeSelection(['a', 'e'], rows, 1, 3)).toEqual(['a', 'e', 'b', 'c', 'd'])
    expect(mergeRangeSelection([], rows, 3, 1)).toEqual(['b', 'c', 'd'])
  })
})
