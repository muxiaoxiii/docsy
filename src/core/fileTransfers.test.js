import { describe, expect, it } from 'vitest'
import { createFileTransfer, transferPaths, transferTargets } from './fileTransfers.js'
import { MARKDOWN_INPUT_EXTENSIONS } from '../modules/markdown-convert/composables/conversionCapabilities.js'

describe('file transfer compatibility', () => {
  it('offers only destinations that accept every selected output', () => {
    const ids = transferTargets(['/a.pdf', '/b.docx']).map((t) => t.id)
    expect(ids).toEqual(['pdf-merge', 'evidence-merge', 'markdown'])
    expect(transferTargets(['/a.png', '/b.pdf']).some((t) => t.id === 'image-layout')).toBe(false)
    expect(transferTargets(['/a.pdf', '/b.pdf']).some((t) => t.single)).toBe(false)
  })
  it('keeps exact generated paths, removes duplicates and validates destinations', () => {
    expect(transferPaths(['/产物/证据 1.PDF', '/产物/证据 1.PDF', null])).toEqual(['/产物/证据 1.PDF'])
    expect(createFileTransfer(['/产物/证据 1.PDF'], 'pdf-crop').paths).toEqual(['/产物/证据 1.PDF'])
    expect(() => createFileTransfer(['/a.png'], 'pdf-crop')).toThrow()
    expect(transferTargets(['/a.exe'])).toEqual([])
    expect(transferTargets([], 'pdf-merge')).toEqual([])
  })
  it('shares the Markdown input capability list and excludes the current destination', () => {
    for (const ext of MARKDOWN_INPUT_EXTENSIONS)
      expect(transferTargets([`/a.${ext}`]).some((t) => t.id === 'markdown')).toBe(true)
    expect(transferTargets(['/a.pdf'], 'pdf-crop').some((t) => t.id === 'pdf-crop')).toBe(false)
  })
})
