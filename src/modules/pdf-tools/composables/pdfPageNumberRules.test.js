import { describe, expect, it } from 'vitest'
import {
  effectivePageNumberRule,
  formatPageNumber,
  pageNumberOverlaysForFile,
  renderPageNumberTemplate,
} from './pdfPageNumberRules.js'

describe('PDF page number rules', () => {
  it('formats supported number styles with safe fallback', () => {
    expect(formatPageNumber(4, 'chinese')).toBe('四')
    expect(formatPageNumber(101, 'chinese')).toBe('一百零一')
    expect(formatPageNumber(2000, 'chinese')).toBe('二千')
    expect(formatPageNumber(14, 'roman-upper')).toBe('XIV')
    expect(formatPageNumber(3, 'circled')).toBe('③')
    expect(formatPageNumber(21, 'circled')).toBe('21')
  })

  it('applies the last matching exception', () => {
    const rule = effectivePageNumberRule(
      {
        style: 'arabic',
        overrides: [
          { start: 1, end: 2, action: 'exclude' },
          { start: 2, end: 2, action: 'override', style: 'roman-upper' },
        ],
      },
      2,
      2,
    )
    expect(rule.style).toBe('roman-upper')
    expect(rule.action).toBe('override')
  })

  it('builds separate overlay ranges around excluded pages', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 5, pageStart: 6 },
      {
        enabled: true,
        totalPages: 10,
        sequence: 'continuous',
        template: '-{page}-',
        style: 'arabic',
        overrides: [{ start: 8, end: 8, action: 'exclude' }],
      },
    )
    expect(overlays).toHaveLength(2)
    expect(overlays[0]).toMatchObject({ pageStart: 1, pageEnd: 2, numberOffset: 0 })
    expect(overlays[1]).toMatchObject({ pageStart: 4, pageEnd: 5, numberOffset: 0 })
    expect(renderPageNumberTemplate('-{page}-', 2, 10, 'circled')).toBe('-②-')
  })
})
