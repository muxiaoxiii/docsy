import { describe, expect, it } from 'vitest'
import { gazeFrame, gazePosition, gazeDistance } from './useDocletGaze.js'

describe('Doclet 视频视线方向', () => {
  it('中心有稳定死区，鼠标左右上下对应不同的视频姿态', () => {
    expect(gazeFrame(0, 0)).toBe(0)
    expect(gazeFrame(.05, -.05)).toBe(0)
    expect(gazeFrame(1, 0)).toBe(17)
    expect(gazeFrame(-1, 0)).toBe(36)
    expect(gazeFrame(0, -1)).toBe(8)
    expect(gazeFrame(0, 1)).toBe(26)
  })
  it('保留细分姿态索引，跨接缝走较短路径', () => {
    const phase = gazePosition(1, -.2)
    expect(phase).not.toBe(Math.round(phase))
    expect(gazeDistance(47.9, .1)).toBeCloseTo(.2)
    expect(gazeDistance(.1, 47.9)).toBeCloseTo(-.2)
    expect(gazeDistance(8, 12)).toBe(4)
  })
  it('整圈方向均落在实际的 48 帧姿态集中，并在正上方平滑衔接', () => {
    for (let degree = 0; degree < 360; degree++) {
      const angle = degree * Math.PI / 180
      const frame = gazeFrame(Math.sin(angle), -Math.cos(angle))
      expect(Number.isInteger(frame)).toBe(true)
      expect(frame).toBeGreaterThanOrEqual(0)
      expect(frame).toBeLessThan(48)
    }
    expect(gazeFrame(-.001, -1)).toBe(gazeFrame(.001, -1))
  })
})
