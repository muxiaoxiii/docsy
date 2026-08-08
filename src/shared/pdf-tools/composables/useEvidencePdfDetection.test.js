import { describe, expect, it } from 'vitest'
import { headerFooterDetectionZoneMm } from './useEvidencePdfDetection.js'

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
