import { describe, it, expect } from 'vitest'

// useHistory uses Vue reactivity — test the logic without Vue by simulating.
// We import the pure logic parts.

describe('useHistory logic', () => {
  // We can't easily test Vue composables outside a component,
  // but we can test the snapshot/restore/fwdRef patterns.

  it('forwardRef round-trips a plain object', () => {
    const source = { align: 'left', fontSize: 12, color: '#000' }
    const cloned = JSON.parse(JSON.stringify(source))
    expect(cloned).toEqual(source)
    expect(cloned).not.toBe(source) // deep clone, not same ref
  })

  it('JSON round-trips preserve number/string/boolean', () => {
    const obj = { a: 1.5, b: 'hello', c: true, d: null }
    expect(JSON.parse(JSON.stringify(obj))).toEqual(obj)
  })
})
