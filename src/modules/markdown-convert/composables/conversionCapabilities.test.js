import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { MARKDOWN_OUTPUT_FORMATS, markdownOutputError, markdownMediaHint } from './conversionCapabilities.js'

describe('Markdown 导出能力与原生实现一致', () => {
  it('界面只提供原生解析器实际接受的输出格式', () => {
    const source = readFileSync(new URL('../../../../src-tauri/src/markdown/mod.rs', import.meta.url), 'utf8')
    const parser = source.split('impl OfficeOutputFormat {')[1].split('fn extension')[0]
    const accepted = [...parser.matchAll(/"([a-z0-9]+)"\s*=>\s*Ok\(Self::/g)].map((match) => match[1])
    expect(accepted.length).toBeGreaterThan(0)
    expect(MARKDOWN_OUTPUT_FORMATS.map((option) => option.value).sort()).toEqual(accepted.sort())
    for (const format of accepted) expect(markdownOutputError(format)).toBe('')
    expect(markdownOutputError('pdf')).toContain('不支持导出 PDF')
    expect(markdownOutputError('odt')).toContain('不支持的 Markdown 输出格式')
  })
  it('按输出格式说明公式的可编辑性与图片回退', () => {
    expect(markdownMediaHint('docx')).toContain('不支持的公式回退为图片')
    expect(markdownMediaHint('xlsx')).toContain('不会转换为 Excel 计算公式')
    expect(markdownMediaHint('pptx')).toContain('不会转换为可编辑')
    expect(markdownMediaHint('html')).toContain('图片')
  })
})
