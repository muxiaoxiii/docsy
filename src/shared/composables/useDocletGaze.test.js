import { describe, expect, it } from 'vitest'
import { GAZE_POSES, GAZE_CELL, nearestGazePose } from './docletGazePoses.js'
import { readFileSync } from 'node:fs'

describe('Doclet 紧凑跟随图集', () => {
  it('二维选择的每个姿态都有对应图集单元，解码像素少于旧版一半', () => {
    const data = readFileSync(new URL('../../assets/doclet-video/gaze-atlas.webp', import.meta.url))
    expect(data.toString('ascii', 12, 16)).toBe('VP8X')
    const width = data.readUIntLE(24, 3) + 1
    const height = data.readUIntLE(27, 3) + 1
    expect(width).toBe(1920)
    expect(height).toBe(Math.ceil(GAZE_POSES.length / 10) * 240)
    expect(width * height).toBeLessThanOrEqual((1920 * 5760) / 2)
    for (const pose of GAZE_POSES) expect(GAZE_CELL.has(nearestGazePose(pose.x, pose.y))).toBe(true)
  })
})
