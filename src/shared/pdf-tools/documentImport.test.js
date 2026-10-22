import { describe, expect, it, vi } from 'vitest'
import { appendUniqueDocuments, convertImportDocument, documentKind, reviewImageImports } from './documentImport.js'
describe('mixed document import and recovery', () => {
  it('routes Word and images to their own converters while preserving a manual PDF', async () => {
    const invoke = vi.fn(async (command) => ({
      ok: true,
      data: command.includes('word') ? '/out/word.pdf' : '/out/image.pdf',
    }))
    expect(await convertImportDocument('/src/a.DOCX', invoke)).toBe('/out/word.pdf')
    expect(await convertImportDocument('/src/a.png', invoke)).toBe('/out/image.pdf')
    expect(await convertImportDocument('/manual/a.pdf', invoke)).toBe('/manual/a.pdf')
    expect(invoke.mock.calls.map(([command]) => command)).toEqual(['convert_word_to_pdf', 'convert_image_to_pdf'])
    expect(documentKind('/src/a.tiff')).toBeNull()
    await expect(
      convertImportDocument('/src/a.doc', async () => ({ ok: false, error: 'engine unavailable' })),
    ).rejects.toThrow('engine unavailable')
  })
  it('keeps existing rules and order when the manual PDF is added after a failed import', () => {
    const original = { path: '/a.pdf', sourcePath: '/a.docx', pageStart: 3, customRules: [1] }
    const result = appendUniqueDocuments(
      [original],
      [{ path: '/manual/b.pdf' }, { path: '/a.pdf' }, { path: '/a-new.pdf', sourcePath: '/a.docx' }],
    )
    expect(result).toEqual([original, { path: '/manual/b.pdf' }])
    expect(result[0]).toBe(original)
  })
  it('reviews consecutive image groups without changing surrounding document order', async () => {
    const review = vi.fn().mockResolvedValueOnce('/one.pdf').mockResolvedValueOnce(null)
    expect(await reviewImageImports(['/a.docx', '/1.png', '/2.jpg', '/b.pdf', '/3.png'], review)).toEqual([
      '/a.docx',
      '/one.pdf',
      '/b.pdf',
    ])
    expect(review.mock.calls).toEqual([[['/1.png', '/2.jpg']], [['/3.png']]])
  })
})
