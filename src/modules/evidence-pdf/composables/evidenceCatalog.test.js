import { describe, expect, it } from 'vitest'
import { catalogFromMerge, catalogRows, catalogNumberRange, separatorPageJobs } from './evidenceCatalog.js'

describe('证据目录页序', () => {
  it('按列表顺序计算实际页码，双面补白不计入证据页段', () => {
    const files = [{ name: '合同.pdf', pages: 3 }, { name: '照片.pdf', pages: 2 }, { name: '票据.pdf', pages: 1 }]
    expect(catalogRows(files, true).map(r => r.pageRange)).toEqual(['1–4', '5–6', '7–8'])
    expect(catalogRows(files.reverse()).map(r => r.pageRange)).toEqual(['1', '2–3', '4–6'])
  })
  it('页数未知时不生成目录', () => expect(() => catalogRows([{ pages: 0 }])).toThrow())
})


describe('合并完成后的目录', () => {
  const files = [{ path: '/a.pdf', name: '甲.pdf', pages: 2 }, { path: '/b.pdf', name: '乙.pdf', pages: 3 }]
  const results = [{ inputPath: '/b.pdf', pages: 3 }, { inputPath: '/a.pdf', pages: 2 }]
  it('采用实际处理结果的顺序和页数，并计入双面补白', () => {
    expect(catalogFromMerge(results, files, true, 6, true)).toEqual([
      { number: '1', name: '乙', pageRange: '1–4' }, { number: '2', name: '甲', pageRange: '5–6' },
    ])
  })
  it('实际总页数不符、缺少结果或重复结果时拒绝导出', () => {
    expect(catalogFromMerge(results, files, true, 6, false).map(row => row.pageRange)).toEqual(['1–3', '4–5'])
    expect(() => catalogFromMerge(results, files, true, 5)).toThrow()
    expect(() => catalogFromMerge(results.slice(1), files, true, 2)).toThrow()
    expect(() => catalogFromMerge([results[0], results[0]], files, false, 6)).toThrow()
  })
})


it('空白页沿用适用于该页的页眉、页脚和页码规则', () => {
  const files = [{ name: '甲', pages: 3 }]
  const resolved = [{ job: { pageStart: 1, totalPages: 4,
    header: { text: '证据一', pageStart: 1, pageEnd: 4 },
    footer: { text: '仅首页', pageStart: 1, pageEnd: 1 },
    extraOverlays: [{ artifactKind: 'PageNumber', text: '{page}', pageStart: 1, pageEnd: 4, numberOffset: 0 }],
  } }]
  const [job] = separatorPageJobs(files, resolved)
  expect(job.pageStart).toBe(4)
  expect(job.header).toMatchObject({ text: '证据一', pageStart: 1, pageEnd: 1 })
  expect(job.footer).toBeNull()
  expect(job.extraOverlays[0]).toMatchObject({ artifactKind: 'PageNumber', pageStart: 1, pageEnd: 1 })
})


it('目录沿用页码样式和实际起始编号', () => {
  expect(catalogNumberRange([{ pageStart: 1, pageEnd: 3, numberOffset: 3, numberStyle: 'roman-upper' }], 1)).toBe('IV–VI')
})
