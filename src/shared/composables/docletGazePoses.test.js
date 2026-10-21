import { describe, expect, it } from 'vitest'
import { GAZE_BLINK_RANGES, gazeSourceFrame } from './docletGazePoses.js'
import { gazePosition } from './useDocletGaze.js'
const isBlink = frame => GAZE_BLINK_RANGES.some(([start, end]) => frame >= start && frame <= end)
describe('Doclet 视线停靠姿态', () => {
  it('整个源片以及任意方向均不会停在闭眼帧', () => {
    for (let frame = 0; frame < 240; frame++) {
      expect(isBlink(gazeSourceFrame(frame / 5))).toBe(false)
    }
    for (let degrees = 0; degrees < 360; degrees += .25) {
      const radians = degrees * Math.PI / 180
      expect(isBlink(gazeSourceFrame(gazePosition(Math.sin(radians), -Math.cos(radians))))).toBe(false)
    }
  })
  it('只替换闭眼片段，保留左右方向及中间正面的睁眼姿态', () => {
    for (const frame of [0, 55, 65, 85, 110, 130, 155, 175, 205, 225]) {
      expect(gazeSourceFrame(frame / 5)).toBe(frame)
    }
    expect(gazeSourceFrame(25 / 5)).toBe(19)
    expect(gazeSourceFrame(238 / 5)).toBe(0)
  })
})
