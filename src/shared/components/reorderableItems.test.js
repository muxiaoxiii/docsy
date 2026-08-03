import { describe, expect, it } from 'vitest'
import { moveItem } from './reorderableItems.js'

describe('moveItem', () => {
  it('moves an item forward without losing entries', () => {
    expect(moveItem(['a', 'b', 'c'], 0, 2)).toEqual(['b', 'c', 'a'])
  })

  it('moves an item backward', () => {
    expect(moveItem(['a', 'b', 'c'], 2, 0)).toEqual(['c', 'a', 'b'])
  })

  it('returns a copy for invalid positions', () => {
    const source = ['a', 'b']
    const result = moveItem(source, -1, 1)
    expect(result).toEqual(source)
    expect(result).not.toBe(source)
  })
})
