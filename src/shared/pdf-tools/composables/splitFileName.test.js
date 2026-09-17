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

  it('cleans base names removing cjk spaces and converting fullwidth digits', () => {
    expect(
      formatSplitFileName({
        base: '国 家 知 识 产 权 局',
        suffix: '20260917',
      }),
    ).toBe('国家知识产权局-20260917')

    expect(
      formatSplitFileName({
        base: '复 审 无 效 宣 告 程 序 意 见 陈 述 书',
        suffix: '20260917',
      }),
    ).toBe('复审无效宣告程序意见陈述书-20260917')

    expect(
      formatSplitFileName({
        base: '证据６',
        suffix: '20260917',
      }),
    ).toBe('证据6-20260917')

    expect(
      formatSplitFileName({
        base: '证据１０',
        suffix: '20260917',
      }),
    ).toBe('证据10-20260917')

    expect(
      formatSplitFileName({
        base: '证据１７译文',
        suffix: '20260917',
      }),
    ).toBe('证据17译文-20260917')

    expect(
      formatSplitFileName({
        base: '证据 17 译文',
        suffix: '20260917',
      }),
    ).toBe('证据17译文-20260917')

    expect(
      formatSplitFileName({
        base: 'Exhibit 1 - Translation',
        suffix: '20260917',
      }),
    ).toBe('Exhibit 1 - Translation-20260917')
  })
})
