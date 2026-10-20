import { describe, expect, it } from 'vitest'
import { DOCLET_CLICK_MOTIONS, randomDocletMotion } from './docletInteractions.js'
describe('Doclet 随机点击动作', () => {
  it('每个随机区间均可选中动作，并避免与上一次重复', () => {
    for (const previous of DOCLET_CLICK_MOTIONS) {
      const selected = new Set()
      for (let i = 0; i < DOCLET_CLICK_MOTIONS.length - 1; i++) {
        const next = randomDocletMotion(previous, () => (i + .5) / (DOCLET_CLICK_MOTIONS.length - 1))
        expect(next).not.toBe(previous)
        selected.add(next)
      }
      expect(selected.size).toBe(DOCLET_CLICK_MOTIONS.length - 1)
    }
  })
})
