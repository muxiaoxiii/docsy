import { describe, expect, it } from 'vitest'
import { GAZE_POSES, gazeTarget, nearestGazePose } from './docletGazePoses.js'

describe('Doclet 二维姿态选择', () => {
  it('保留中心死区和远近程度，不只按鼠标夹角挑视频进度', () => {
    expect(gazeTarget(0.05, -0.05)).toEqual({ x: 0, y: 0 })
    expect(gazeTarget(0.3, 0)).toEqual({ x: 0.3, y: 0 })
    expect(gazeTarget(3, 0)).toEqual({ x: 1, y: 0 })
    expect(nearestGazePose(0, 0)).toBe(0)
    expect(nearestGazePose(1, 0)).toBe(85)
    expect(nearestGazePose(-1, 0)).toBe(175)
    expect(nearestGazePose(0, -1)).toBe(55)
  })
  it('跟随素材不再包含闭眼、开头等待和中间重复正面段', () => {
    expect(new Set(GAZE_POSES.map((pose) => pose.frame)).size).toBe(GAZE_POSES.length)
    for (const pose of GAZE_POSES) {
      expect(pose.frame === 0 || pose.frame >= 51).toBe(true)
      expect(
        [
          [92, 100],
          [116, 124],
          [131, 145],
          [186, 194],
          [211, 239],
        ].some(([a, b]) => pose.frame >= a && pose.frame <= b),
      ).toBe(false)
    }
  })
})
