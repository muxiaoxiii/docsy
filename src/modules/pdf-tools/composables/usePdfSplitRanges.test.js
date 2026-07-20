import { describe, expect, it } from 'vitest'
import { splitRangeWarnings } from './usePdfSplitRanges.js'

describe('PDF split range helpers', () => {
  it('returns no warnings for full continuous ranges', () => {
    expect(
      splitRangeWarnings(
        [
          { name: '证据1', pageStart: 1, pageEnd: 2 },
          { name: '证据2', pageStart: 3, pageEnd: 5 },
        ],
        5,
      ),
    ).toEqual([])
  })

  it('detects gaps and overlaps', () => {
    expect(
      splitRangeWarnings(
        [
          { name: '证据1', pageStart: 1, pageEnd: 3 },
          { name: '证据2', pageStart: 3, pageEnd: 4 },
          { name: '证据3', pageStart: 6, pageEnd: 6 },
        ],
        6,
      ),
    ).toEqual(['页段「证据2」与前面的页段存在重叠', '第 5-5 页未包含在拆分页段中'])
  })

  it('detects invalid names and page bounds', () => {
    expect(
      splitRangeWarnings(
        [
          { name: '', pageStart: 1, pageEnd: 1 },
          { name: '反向', pageStart: 4, pageEnd: 2 },
          { name: '越界', pageStart: 5, pageEnd: 12 },
        ],
        10,
      ),
    ).toEqual([
      '页段「未命名」缺少名称或起止页',
      '页段「反向」起始页大于结束页',
      '页段「越界」结束页超过总页数',
      '第 1-4 页未包含在拆分页段中',
    ])
  })

  it('normalizes numeric strings from form inputs', () => {
    expect(
      splitRangeWarnings(
        [
          { name: '证据1', pageStart: '1', pageEnd: '2' },
          { name: '证据2', pageStart: '3', pageEnd: '3' },
        ],
        '3',
      ),
    ).toEqual([])
  })
})
