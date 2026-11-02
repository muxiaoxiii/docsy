import { afterEach, describe, expect, it, vi } from 'vitest'
import { joinSelectedHeaderTokens, splitHeaderTokens } from './headerTextSelection.js'

afterEach(() => vi.unstubAllGlobals())

describe('selectable header text', () => {
  it('preserves Chinese, punctuation, whitespace, numbers and line breaks exactly', () => {
    const source = ' 证据文件1 - CN112186046A（补充）\n第 2 页 '
    for (const mode of ['word', 'character']) {
      const tokens = splitHeaderTokens(source, mode)
      expect(tokens.length).toBeGreaterThan(1)
      expect(tokens.join('')).toBe(source)
      expect(
        joinSelectedHeaderTokens(
          tokens,
          tokens.map((_, index) => index),
        ),
      ).toBe(source)
    }
    expect(splitHeaderTokens('')).toEqual([])
  })

  it('keeps grapheme clusters such as joined emoji and combining marks intact', () => {
    const source = '证👨‍👩‍👧‍👦e\u0301据'
    expect(splitHeaderTokens(source, 'character')).toEqual(['证', '👨‍👩‍👧‍👦', 'e\u0301', '据'])
    expect(splitHeaderTokens(source, 'word').join('')).toBe(source)
  })

  it('uses original position instead of click order and distinguishes repeated tokens', () => {
    const tokens = ['证据', '1', '、', '证据', '2']
    expect(joinSelectedHeaderTokens(tokens, [4, 3, 0])).toBe('证据证据2')
    expect(joinSelectedHeaderTokens(tokens, new Set([3, 1]))).toBe('1证据')
    expect(joinSelectedHeaderTokens(tokens, [3, 3, 100, -1])).toBe('证据')
    expect(joinSelectedHeaderTokens(tokens, [])).toBe('')
  })

  it('falls back without losing text when segmentation is missing or unsupported', () => {
    const source = 'A👨‍👩‍👧‍👦e\u0301证据'
    vi.stubGlobal('Intl', {})
    expect(splitHeaderTokens(source, 'word').join('')).toBe(source)
    expect(splitHeaderTokens(source, 'character').join('')).toBe(source)
    vi.stubGlobal('Intl', {
      Segmenter: class {
        constructor() {
          throw new Error('unsupported')
        }
      },
    })
    expect(splitHeaderTokens(source).join('')).toBe(source)
  })
})
