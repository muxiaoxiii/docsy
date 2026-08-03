import { describe, expect, it } from 'vitest'
import { detectedElementFromCandidate, mergeExistingElements } from './existingPdfElements.js'

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
})
