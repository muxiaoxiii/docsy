import { describe, expect, it } from 'vitest'
import {
  batchSummaryText,
  formatFileSize,
  resolveOptimizedImport,
  savedPercent,
  sizeSavingText,
  summarizeOptimizedImports,
} from './pdfLosslessOptimize.js'

describe('pdfLosslessOptimize', () => {
  it('uses the optimized copy only when the backend reports a real change', () => {
    const result = {
      ok: true,
      data: {
        output_path: '/case/合同_optimized.pdf',
        input_size: 39845888,
        output_size: 3565158,
        changed: true,
      },
    }
    expect(resolveOptimizedImport('/case/合同.pdf', result)).toEqual({
      path: '/case/合同_optimized.pdf',
      optimized: true,
      inputSize: 39845888,
      outputSize: 3565158,
    })
  })

  it('falls back to the original path when unchanged, failed, or missing data', () => {
    const unchanged = {
      ok: true,
      data: { output_path: '/case/a.pdf', input_size: 100, output_size: 100, changed: false },
    }
    expect(resolveOptimizedImport('/case/a.pdf', unchanged)).toEqual({
      path: '/case/a.pdf',
      optimized: false,
      inputSize: 0,
      outputSize: 0,
    })
    expect(resolveOptimizedImport('/case/a.pdf', { ok: false, error: 'boom' }).path).toBe('/case/a.pdf')
    expect(resolveOptimizedImport('/case/a.pdf', null).optimized).toBe(false)
    expect(resolveOptimizedImport('/case/a.pdf', { ok: true, data: { changed: true } }).optimized).toBe(false)
  })

  it('summarizes only the files that were actually optimized', () => {
    const decisions = [
      { path: '/a.pdf', optimized: true, inputSize: 2000, outputSize: 1000 },
      { path: '/b.pdf', optimized: false, inputSize: 0, outputSize: 0 },
      { path: '/c.pdf', optimized: true, inputSize: 3000, outputSize: 500 },
    ]
    expect(summarizeOptimizedImports(decisions)).toEqual({ count: 2, inputSize: 5000, outputSize: 1500 })
    expect(summarizeOptimizedImports([])).toEqual({ count: 0, inputSize: 0, outputSize: 0 })
  })

  it('formats sizes and saving text for the compress notification', () => {
    expect(formatFileSize(39845888)).toBe('38.0MB')
    expect(formatFileSize(3565158)).toBe('3.4MB')
    expect(formatFileSize(512)).toBe('512B')
    expect(formatFileSize(2048)).toBe('2.0KB')
    expect(formatFileSize(0)).toBe('0B')
    expect(savedPercent(39845888, 3565158)).toBe(91)
    expect(savedPercent(0, 100)).toBeNull()
    expect(sizeSavingText(39845888, 3565158)).toBe('38.0MB → 3.4MB(节省 91%)')
  })

  it('builds batch summary with size saving and failed names', () => {
    const items = [
      { name: 'a.pdf', status: 'done', inputSize: 39845888, outputSize: 3565158 },
      { name: 'b.pdf', status: 'done', inputSize: 2048, outputSize: 1024 },
      { name: 'c.pdf', status: 'failed' },
    ]
    expect(batchSummaryText('压缩完成', items)).toBe('压缩完成 2/3：共 38.0MB → 3.4MB(节省 91%)；失败：c.pdf')
    const allDone = [
      { name: 'a.pdf', status: 'done', inputSize: 39845888, outputSize: 3565158 },
      { name: 'b.pdf', status: 'done', inputSize: 2048, outputSize: 1024 },
    ]
    expect(batchSummaryText('压缩完成', allDone)).toBe('压缩完成 2/2：共 38.0MB → 3.4MB(节省 91%)')
  })

  it('omits the size part for batch jobs without size data', () => {
    const items = [
      { name: 'a.md', status: 'done' },
      { name: 'b.docx', status: 'failed' },
    ]
    expect(batchSummaryText('转换完成', items)).toBe('转换完成 1/2；失败：b.docx')
    expect(batchSummaryText('转换完成', [])).toBe('转换完成 0/0')
  })
})
