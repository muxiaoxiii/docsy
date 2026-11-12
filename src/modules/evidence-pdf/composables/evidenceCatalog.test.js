import { describe, expect, it } from 'vitest'
import { catalogFromMerge, catalogRows } from './evidenceCatalog.js'

describe('证据目录页序', () => {
  it('按列表顺序计算实际页码，双面补白不计入证据页段', () => {
    const files = [{ name: '合同.pdf', pages: 3 }, { name: '照片.pdf', pages: 2 }, { name: '票据.pdf', pages: 1 }]
    expect(catalogRows(files, true).map(r => r.pageRange)).toEqual(['1–3', '5–6', '7'])
    expect(catalogRows(files.reverse()).map(r => r.pageRange)).toEqual(['1', '2–3', '4–6'])
  })
  it('页数未知时不生成目录', () => expect(() => catalogRows([{ pages: 0 }])).toThrow())
})


describe('合并完成后的目录', () => {
  const files = [{ path: '/a.pdf', name: '甲.pdf', pages: 99 }, { path: '/b.pdf', name: '乙.pdf', pages: 99 }]
  const results = [{ inputPath: '/b.pdf', pages: 3 }, { inputPath: '/a.pdf', pages: 2 }]
  it('采用实际处理结果的顺序和页数，并计入双面补白', () => {
    expect(catalogFromMerge(results, files, true, 6)).toEqual([
      { number: '1', name: '乙', pageRange: '1–3' }, { number: '2', name: '甲', pageRange: '5–6' },
    ])
  })
  it('实际总页数不符、缺少结果或重复结果时拒绝导出', () => {
    expect(() => catalogFromMerge(results, files, true, 5)).toThrow()
    expect(() => catalogFromMerge(results.slice(1), files, true, 2)).toThrow()
    expect(() => catalogFromMerge([results[0], results[0]], files, false, 6)).toThrow()
  })
})
