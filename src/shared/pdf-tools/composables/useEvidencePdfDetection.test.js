import { describe, expect, it } from 'vitest'
import { headerFooterDetectionZoneMm, isFirstPageEvidenceCandidate } from './useEvidencePdfDetection.js'

describe('headerFooterDetectionZoneMm', () => {
  it('keeps detection wider than the cleanup band', () => {
    expect(headerFooterDetectionZoneMm(18)).toBe(25)
    expect(headerFooterDetectionZoneMm(32)).toBe(32)
  })

  it('bounds invalid and excessive values', () => {
    expect(headerFooterDetectionZoneMm(0)).toBe(25)
    expect(headerFooterDetectionZoneMm(100)).toBe(60)
  })
})

describe('isFirstPageEvidenceCandidate', () => {
  it('accepts backend-tagged evidence labels on page 1', () => {
    expect(
      isFirstPageEvidenceCandidate({
        labels: ['evidence-label'],
        text: '证据１',
        normalizedText: '证据1',
        pageRange: { start: 1, end: 1 },
      }),
    ).toBe(true)
  })

  it('matches 证据/对比文件 patterns without a backend label', () => {
    expect(
      isFirstPageEvidenceCandidate({ text: '证据２', normalizedText: '证据2', pageRange: { start: 1, end: 1 } }),
    ).toBe(true)
    expect(isFirstPageEvidenceCandidate({ text: '对比文件3', pageRange: { start: 1, end: 1 } })).toBe(true)
    expect(isFirstPageEvidenceCandidate({ text: '证据一', pageRange: { start: 1, end: 1 } })).toBe(true)
  })

  it('rejects labels not starting on page 1', () => {
    expect(
      isFirstPageEvidenceCandidate({
        labels: ['evidence-label'],
        text: '证据2',
        pageRange: { start: 2, end: 2 },
      }),
    ).toBe(false)
  })

  it('rejects non-label text', () => {
    expect(isFirstPageEvidenceCandidate({ text: '证据目录', pageRange: { start: 1, end: 1 } })).toBe(false)
    expect(isFirstPageEvidenceCandidate({ text: '附 页', pageRange: { start: 1, end: 1 } })).toBe(false)
    expect(isFirstPageEvidenceCandidate(null)).toBe(false)
  })
})
