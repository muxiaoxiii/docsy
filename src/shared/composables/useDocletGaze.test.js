import { describe, expect, it } from 'vitest'
import { GAZE_LAYOUT, GAZE_CELL } from './docletGazePoses.js'
import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'

describe('Doclet 新素材图集', () => {
  it('布局与实际图集一致，全部方向有帧，解码占用低于原图集一半', () => {
    const data = readFileSync(new URL('../../assets/doclet-video/gaze-atlas.webp', import.meta.url))
    expect(data.toString('ascii', 12, 16)).toBe('VP8X')
    const width = data.readUIntLE(24, 3) + 1
    const height = data.readUIntLE(27, 3) + 1
    expect(width).toBe(GAZE_LAYOUT.columns * GAZE_LAYOUT.width)
    expect(height).toBe(Math.ceil(GAZE_LAYOUT.frames.length / GAZE_LAYOUT.columns) * GAZE_LAYOUT.height)
    expect(width * height).toBeLessThan((1920 * 5760) / 2)
    expect(GAZE_CELL.size).toBe(GAZE_LAYOUT.frames.length)
    for (const source of GAZE_LAYOUT.sources) {
      const video = readFileSync(new URL(`../../../${source.path}`, import.meta.url))
      expect(createHash('sha256').update(video).digest('hex')).toBe(source.sha256)
    }
  })
})
