import { describe, it, expect } from 'vitest'

describe('Template Editor Marks', () => {
  // Test mark overlap detection
  function isOverlapping(marks, start, end) {
    return marks.some(m => start < m.end && end > m.start)
  }

  it('should detect overlapping marks', () => {
    const marks = [
      { id: '1', start: 0, end: 5 },
      { id: '2', start: 10, end: 15 },
    ]
    expect(isOverlapping(marks, 3, 8)).toBe(true)
    expect(isOverlapping(marks, 6, 9)).toBe(false)
    expect(isOverlapping(marks, 12, 14)).toBe(true)
  })

  it('should allow adjacent marks', () => {
    const marks = [{ id: '1', start: 0, end: 5 }]
    expect(isOverlapping(marks, 5, 10)).toBe(false)
  })

  // Test field key generation
  function generateKey(label, existingKeys) {
    const base = label
      .toLowerCase()
      .replace(/[^a-z0-9\u4e00-\u9fff]/g, '_')
      .replace(/_+/g, '_')
      .replace(/^_|_$/g, '')
    let key = base || 'field'
    let i = 1
    while (existingKeys.includes(key)) {
      key = `${base}_${i}`
      i++
    }
    return key
  }

  it('should generate unique keys', () => {
    expect(generateKey('法院', [])).toBe('法院')
    expect(generateKey('法院', ['法院'])).toBe('法院_1')
    expect(generateKey('法院', ['法院', '法院_1'])).toBe('法院_2')
  })

  it('should handle empty label', () => {
    expect(generateKey('', [])).toBe('field')
  })
})
