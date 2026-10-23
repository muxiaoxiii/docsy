import { describe, expect, it } from 'vitest'
import { PAGE_NUMBER_FORMATS, pageNumberPresets, templateForTotal } from './pageNumberFormats.js'
import { renderPageNumberTemplate } from './composables/pdfPageNumberRules.js'

describe('page number formats and total toggle', () => {
  it('renders every offered format with the actual numbering renderer', () => {
    for (const preset of pageNumberPresets(true)) {
      const value = renderPageNumberTemplate(preset.value, 2, 35)
      expect(value).toContain('2')
      expect(value).toContain('35')
      expect(value).not.toMatch(/[{}]/)
    }
    expect(renderPageNumberTemplate('[{page}]', 2, 35, 'roman-upper')).toBe('[II]')
    expect(renderPageNumberTemplate('第{page}页 / 共{total}页', 2, 35, 'chinese')).toBe('第二页 / 共三十五页')
  })
  it('removes the entire total phrase and restores the exact original preset', () => {
    for (const preset of PAGE_NUMBER_FORMATS) {
      const without = templateForTotal(preset.withTotal, false)
      expect(without).toBe(preset.withoutTotal)
      expect(templateForTotal(without, true, preset.withTotal)).toBe(preset.withTotal)
    }
    expect(templateForTotal('第{page}页，共{total}页', false)).toBe('第{page}页')
  })
  it('keeps custom formats recoverable without leaving incomplete prose', () => {
    const custom = '证据页 {page} / 总计 {total} 页'
    expect(templateForTotal(custom, false)).toBe('{page}')
    expect(templateForTotal('{page}', true, custom)).toBe(custom)
    expect(templateForTotal('附件 {page}', false)).toBe('附件 {page}')
    expect(templateForTotal('{range}', false)).toBe('{page}')
    expect(templateForTotal('{page}', true, '{range}')).toBe('{range}')
    for (const enabled of [false, true]) {
      const presets = pageNumberPresets(enabled)
      expect(new Set(presets.map((p) => p.value)).size).toBe(presets.length)
    }
  })
})
