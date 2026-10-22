import { describe, expect, it } from 'vitest'
import { dragCrop } from './imageCrop.js'
describe('manual crop geometry', () => {
  it('keeps crop dimensions when moving against a source boundary', () => {
    expect(dragCrop([0.2, 0.1, 0.8, 0.6], 'move', 0.5, -0.5)).toEqual([0.39999999999999997, 0, 1, 0.5])
  })
  it('does not invert edges or let a corner leave the source', () => {
    expect(dragCrop([0.2, 0.2, 0.8, 0.8], 'nw', -1, -1)).toEqual([0, 0, 0.8, 0.8])
    const box = dragCrop([0.2, 0.2, 0.8, 0.8], 'se', -2, -2, 0.001, 0.002)
    expect(box[2] - box[0]).toBeCloseTo(0.001)
    expect(box[3] - box[1]).toBeCloseTo(0.002)
  })
})
