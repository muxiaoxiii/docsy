import { describe, expect, it } from 'vitest'
import { catalogRows } from './evidenceCatalog.js'

describe('证据目录页序', () => {
  it('按列表顺序计算实际页码，双面补白不计入证据页段', () => {
    const files = [{ name: '合同.pdf', pages: 3 }, { name: '照片.pdf', pages: 2 }, { name: '票据.pdf', pages: 1 }]
    expect(catalogRows(files, true).map(r => r.pageRange)).toEqual(['1–3', '5–6', '7'])
    expect(catalogRows(files.reverse()).map(r => r.pageRange)).toEqual(['1', '2–3', '4–6'])
  })
  it('页数未知时不生成目录', () => expect(() => catalogRows([{ pages: 0 }])).toThrow())
})
