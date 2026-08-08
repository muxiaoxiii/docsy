import { describe, expect, it } from 'vitest'
import { expandSplitNameTokens, formatDateToken, formatSequenceToken, formatSplitFileName } from './splitFileName.js'

describe('split file name helpers', () => {
  it('formats date tokens without overlapping replacements', () => {
    expect(formatDateToken('YYYYMMDD', '20260420')).toBe('20260420')
    expect(formatDateToken('YYYY-MM-DD', '20260420')).toBe('2026-04-20')
    expect(formatDateToken('DD', '20260420')).toBe('20')
  })

  it('formats sequence tokens by hash width', () => {
    expect(formatSequenceToken('#', 8)).toBe('9')
    expect(formatSequenceToken('##', 8)).toBe('09')
    expect(formatSequenceToken('###', 8)).toBe('009')
  })

  it('expands date and sequence tokens in one name part', () => {
    expect(expandSplitNameTokens('证据[##]-[DD]', 8, '20260420')).toBe('证据09-20')
    expect(expandSplitNameTokens('证据[中文序号]', 10, '20260420')).toBe('证据十一')
  })

  it('uses the selected separator consistently for all parts', () => {
    expect(
      formatSplitFileName({
        base: '证据9-1',
        index: 1,
        prefix: '405案',
        suffix: '[YYYYMMDD]',
        dateValue: '20260420',
        separator: '',
      }),
    ).toBe('405案证据9-120260420')

    expect(
      formatSplitFileName({
        base: '证据9-1',
        index: 1,
        prefix: '405案',
        suffix: '[YYYYMMDD]',
        dateValue: '20260420',
        separator: '-',
      }),
    ).toBe('405案-证据9-1-20260420')
  })
})
