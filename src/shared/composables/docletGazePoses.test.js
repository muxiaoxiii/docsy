import { describe, expect, it } from 'vitest'
import { GAZE_LAYOUT, GAZE_CELL, gazeTarget } from './docletGazePoses.js'

describe('Doclet 有效方向', () => {
  it('中心保持正视，远近不排队，斜向取最近有效方向', () => {
    expect(gazeTarget(0.05, -0.05)).toBe(null)
    expect(gazeTarget(0.3, 0)).toBe('right')
    expect(gazeTarget(3, 0)).toBe('right')
    expect(gazeTarget(-1, 0)).toBe('left')
    expect(gazeTarget(0.2, -1)).toBe('up')
    expect(gazeTarget(-0.2, 1)).toBe('down')
  })
  it('各方向拥有真实连续的出发与回正片段，全部帧都在图集中', () => {
    expect(GAZE_CELL.has(GAZE_LAYOUT.center)).toBe(true)
    for (const route of Object.values(GAZE_LAYOUT.routes)) {
      for (const frames of Object.values(route)) {
        for (let i = 0; i < frames.length; i++) {
          expect(GAZE_CELL.has(frames[i])).toBe(true)
          if (i) expect(frames[i] - frames[i - 1]).toBe(1)
        }
      }
    }
  })
})
