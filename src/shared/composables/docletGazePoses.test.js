import { describe, expect, it } from 'vitest'
import { GAZE_LAYOUT, GAZE_CELL, gazeTarget } from './docletGazePoses.js'

describe('Doclet 有效方向', () => {
  it('中心保持正视，远近不排队，八方向都有实际映射', () => {
    expect(gazeTarget(0.05, -0.05)).toBe(null)
    expect(gazeTarget(0.3, 0)).toBe('right')
    expect(gazeTarget(3, 0)).toBe('right')
    expect(gazeTarget(-1, 0)).toBe('left')
    expect(gazeTarget(0.2, -1)).toBe('up')
    expect(gazeTarget(-0.2, 1)).toBe('down')
    expect(gazeTarget(-1, -1)).toBe('up-left')
    expect(gazeTarget(-1, 1)).toBe('down-left')
    expect(gazeTarget(1, -1)).toBe('up-right')
    expect(gazeTarget(1, 1)).toBe('down-right')
  })
  it('新旧素材拼接八方向，所有帧都在图集中且跳过旧片闭眼段', () => {
    expect(GAZE_CELL.has(GAZE_LAYOUT.center)).toBe(true)
    expect(Object.keys(GAZE_LAYOUT.routes)).toHaveLength(8)
    for (const route of Object.values(GAZE_LAYOUT.routes)) {
      for (const frames of Object.values(route)) {
        for (let i = 0; i < frames.length; i++) {
          expect(GAZE_CELL.has(frames[i])).toBe(true)
          if (frames[i] >= 1000) {
            const oldFrame = frames[i] - 1000
            expect(
              [
                [92, 100],
                [186, 194],
              ].some(([a, b]) => oldFrame >= a && oldFrame <= b),
            ).toBe(false)
          }
        }
      }
    }
  })
})
