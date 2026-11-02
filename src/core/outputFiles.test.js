import { describe, expect, it } from 'vitest'
import { commonSourceDirectory, joinOutputPath, pdfNameFromSource, pdfOutputFileName } from './outputFiles.js'
describe('output location and filename defaults', () => {
  it('uses the immediate common folder, never an unrelated common ancestor', () => {
    expect(commonSourceDirectory(['/case/a.pdf', '/case/b.docx'])).toBe('/case')
    expect(commonSourceDirectory(['/case/one/a.pdf', '/case/two/b.pdf'])).toBe('')
    expect(commonSourceDirectory([])).toBe('')
    expect(commonSourceDirectory(['/a.pdf', '/b.pdf'])).toBe('/')
    expect(joinOutputPath('/', '合并.pdf')).toBe('/合并.pdf')
  })
  it('compares Windows separators/case and preserves drive roots', () => {
    expect(commonSourceDirectory(['C:\\Case\\a.pdf', 'c:/case/b.pdf'])).toBe('C:\\Case')
    expect(commonSourceDirectory(['C:\\a.pdf', 'C:\\b.pdf'])).toBe('C:\\')
    expect(joinOutputPath('C:\\', 'a.pdf')).toBe('C:\\a.pdf')
    expect(commonSourceDirectory(['/Case/a.pdf', '/case/b.pdf'])).toBe('')
  })
  it('uses original Word/image folders rather than conversion scratch folders', () => {
    expect(
      commonSourceDirectory([
        { path: '/tmp/a.pdf', sourcePath: '/case/a.docx' },
        { path: '/tmp/img.pdf', sourcePaths: ['/case/1.jpg', '/case/2.png'] },
      ]),
    ).toBe('/case')
    expect(commonSourceDirectory([{ path: '/tmp/img.pdf', sourcePaths: ['/one/1.jpg', '/two/2.png'] }])).toBe('')
  })
  it('keeps input stems and consistently makes a leaf PDF filename', () => {
    expect(pdfNameFromSource('/case/合同.v2.docx')).toBe('合同.v2.pdf')
    expect(pdfOutputFileName('新证据')).toBe('新证据.pdf')
    expect(pdfOutputFileName('')).toBe('合并文件.pdf')
    for (const value of ['../source.pdf', 'C:\\a.pdf', 'a/b', 'bad?.pdf', 'CON.pdf'])
      expect(() => pdfOutputFileName(value)).toThrow()
  })
})
