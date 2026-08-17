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
    // 新结构用 overrides 表达例外，不再携带旧 action 字段
    expect(rule.action).toBeUndefined()
  })

  it('builds separate overlay ranges around excluded pages', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 5, pageStart: 6 },
      {
        enabled: true,
        totalPages: 10,
        allFiles: [{ pages: 5, pageStart: 6 }],
        pageNumberStart: 6,
        sequence: 'continuous',
        template: '-{page}-',
        style: 'arabic',
        exceptions: [
          { scope: { type: 'global', start: 8, end: 8 }, overrides: { enabled: false, count: true } },
        ],
      },
    )
    expect(overlays).toHaveLength(2)
    expect(overlays[0]).toMatchObject({ pageStart: 1, pageEnd: 2, numberOffset: 0 })
    expect(overlays[1]).toMatchObject({ pageStart: 4, pageEnd: 5, numberOffset: 0 })
    expect(renderPageNumberTemplate('-{page}-', 2, 10, 'circled')).toBe('-②-')
  })

  it('filters exceptions by scope fileIds', () => {
    const baseRule = {
      style: 'arabic',
      exceptions: [
        { scope: { type: 'global', start: 1, end: 99, fileIds: ['target.pdf'] }, overrides: { enabled: false } },
      ],
    }
    const matched = effectivePageNumberRule(baseRule, 1, 1, { id: 'target.pdf' })
    expect(matched.enabled).toBe(false)
    const other = effectivePageNumberRule(baseRule, 1, 1, { id: 'other.pdf' })
    expect(other.enabled).toBeUndefined()
  })

  it('skips excluded pages in numbering when exception sets count off', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 2, pageStart: 3 },
      {
        enabled: true,
        allFiles: [
          { pages: 2, pageStart: 1 },
          { pages: 2, pageStart: 3 },
        ],
        pageNumberStart: 1,
        sequence: 'continuous',
        totalMode: 'combined',
        template: '{page}',
        style: 'arabic',
        exceptions: [
          { scope: { type: 'global', start: 2, end: 2 }, overrides: { enabled: false, count: false } },
        ],
      },
    )
    // file2 第 1 页物理页码 3，前面计数页只有 file1 第 1 页 → 显示编号 2
    expect(overlays[0]).toMatchObject({ pageStart: 1, pageEnd: 2, numberOffset: -1 })
    expect(renderPageNumberTemplate('{page}', 3 - 1, 3, 'arabic')).toBe('2')
  })

  it('keeps excluded pages in numbering by default', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 2, pageStart: 3 },
      {
        enabled: true,
        allFiles: [
          { pages: 2, pageStart: 1 },
          { pages: 2, pageStart: 3 },
        ],
        pageNumberStart: 1,
        sequence: 'continuous',
        totalMode: 'combined',
        template: '{page}',
        style: 'arabic',
        exceptions: [
          { scope: { type: 'global', start: 2, end: 2 }, overrides: { enabled: false } },
        ],
      },
    )
    // file1 第 2 页被排除但未关闭计数 → 仍占用编号，file2 第 1 页显示编号 3
    expect(overlays[0]).toMatchObject({ pageStart: 1, pageEnd: 2, numberOffset: 0 })
    expect(renderPageNumberTemplate('{page}', 3 + 0, 4, 'arabic')).toBe('3')
  })

  it('uses per-file total pages when totalMode is per-file', () => {
    const overlays = pageNumberOverlaysForFile(
      { id: 'f1', pages: 2, pageStart: 1 },
      {
        enabled: true,
        allFiles: [
          { id: 'f1', pages: 2, pageStart: 1 },
          { id: 'f2', pages: 4, pageStart: 3 },
        ],
        pageNumberStart: 1,
        sequence: 'continuous',
        totalMode: 'per-file',
        template: '{page}/{total}',
        style: 'arabic',
      },
    )
    expect(overlays[0].numberTotal).toBe(2)
  })

  it('honors a custom starting number with global page offset', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 1, pageStart: 1 },
      {
        enabled: true,
        allFiles: [{ pages: 1, pageStart: 1 }],
        pageNumberStart: 181,
        sequence: 'continuous',
        totalMode: 'combined',
        template: '第{page}页',
        style: 'arabic',
      },
    )
    expect(overlays[0]).toMatchObject({ numberOffset: 180, numberTotal: 181 })
    expect(renderPageNumberTemplate('第{page}页', 1 + 180, 181, 'arabic')).toBe('第181页')
  })

  it('splits overlays when an exception overrides style', () => {
    const overlays = pageNumberOverlaysForFile(
      { pages: 3, pageStart: 1 },
      {
        enabled: true,
        allFiles: [{ pages: 3, pageStart: 1 }],
        pageNumberStart: 1,
        sequence: 'continuous',
        template: '{page}',
        style: 'arabic',
        exceptions: [
          { scope: { type: 'global', start: 2, end: 2 }, overrides: { style: 'roman-upper' } },
        ],
      },
    )
    expect(overlays).toHaveLength(3)
    expect(overlays[0].numberStyle).toBe('arabic')
    expect(overlays[1].numberStyle).toBe('roman-upper')
    expect(overlays[2].numberStyle).toBe('arabic')
  })
})
