import { describe, expect, it } from 'vitest'
import { reorderTargetIndex } from './usePointerReorder.js'

describe('reorderTargetIndex', () => {
  it('moves the first item after the last item', () => {
    expect(reorderTargetIndex(0, 2, 'after', 3)).toBe(2)
  })

  it('moves the last item before the first item', () => {
    expect(reorderTargetIndex(2, 0, 'before', 3)).toBe(0)
  })

  it('accounts for removing the source before insertion', () => {
    expect(reorderTargetIndex(0, 2, 'before', 3)).toBe(1)
    expect(reorderTargetIndex(2, 0, 'after', 3)).toBe(1)
  })
})
