import { describe, expect, it } from 'vitest'
import {
  imagePages,
  joinImagePage,
  reorderPageImage,
  splitImagePage,
  autoImageGrid,
  fitImageGrid,
} from './imagePages.js'
const sources = () => ['a', 'b', 'c'].map((path) => ({ path, pageId: path, percent: 100 }))
describe('explicit image pages', () => {
  it('adjusts the other grid control without using capacity to split or reject a page', () => {
    expect(fitImageGrid(2, { rows: 1, cols: 1 }, 'rows')).toEqual({ rows: 1, cols: 2 })
    expect(fitImageGrid(2, { rows: 1, cols: 1 }, 'cols')).toEqual({ rows: 2, cols: 1 })
    for (let count = 1; count <= 16; count++) {
      for (let rows = 1; rows <= 4; rows++)
        for (let cols = 1; cols <= 4; cols++) {
          for (const changed of ['rows', 'cols']) {
            const grid = fitImageGrid(count, { rows, cols }, changed)
            expect(grid.rows * grid.cols).toBeGreaterThanOrEqual(count)
            expect(grid.rows).toBeLessThanOrEqual(4)
            expect(grid.cols).toBeLessThanOrEqual(4)
          }
        }
    }
  })
  it('starts one image per page and an ordinary reorder keeps that separation', () => {
    expect(imagePages(sources()).map((page) => page.images.length)).toEqual([1, 1, 1])
    const moved = reorderPageImage(sources(), 0, 1, 'new')
    expect(imagePages(moved).map((page) => page.images.map((image) => image.path))).toEqual([['b'], ['a'], ['c']])
  })
  it('joins only on an explicit page drop, and can split back without losing edits', () => {
    const joined = joinImagePage(sources(), 'a', 'b')
    expect(imagePages(joined).map((page) => page.images.map((image) => image.path))).toEqual([['b', 'a'], ['c']])
    const split = splitImagePage(joined, 'a', 'separate')
    expect(imagePages(split).map((page) => page.images.map((image) => image.path))).toEqual([['b'], ['a'], ['c']])
    expect(split.find((image) => image.path === 'a').percent).toBe(100)
    expect(sources()[0].pageId).toBe('a')
  })
  it('moving a member across pages detaches it instead of changing the target page composition', () => {
    const joined = joinImagePage(sources(), 'a', 'b')
    expect(
      imagePages(reorderPageImage(joined, 1, 2, 'detached')).map((page) => page.images.map((image) => image.path)),
    ).toEqual([['b'], ['c'], ['a']])
    expect(autoImageGrid(2)).toEqual({ rows: 2, cols: 1 })
    expect(autoImageGrid(2, true)).toEqual({ rows: 1, cols: 2 })
  })
})
