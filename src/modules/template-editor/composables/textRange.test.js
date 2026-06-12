import { describe, it, expect } from 'vitest'

describe('Text Range Utilities', () => {
  function getSelectedText(plainText, selectionStart, selectionEnd) {
    return plainText.slice(selectionStart, selectionEnd)
  }

  function findTextInParagraphs(paragraphs, searchText) {
    let offset = 0
    for (const para of paragraphs) {
      const idx = para.text.indexOf(searchText)
      if (idx !== -1) {
        return { paragraphIndex: para.index, charOffset: offset + idx }
      }
      offset += para.text.length + 1 // +1 for newline
    }
    return null
  }

  it('should extract selected text', () => {
    const text = '北京知识产权法院：'
    expect(getSelectedText(text, 0, 7)).toBe('北京知识产权法')
    expect(getSelectedText(text, 0, 8)).toBe('北京知识产权法院')
  })

  it('should find text position in paragraphs', () => {
    const paragraphs = [
      { index: 0, text: '律师事务所函' },
      { index: 1, text: '北京知识产权法院：' },
      { index: 2, text: '原告日本制铁株式会社' },
    ]
    const result = findTextInParagraphs(paragraphs, '知识产权')
    expect(result).toEqual({ paragraphIndex: 1, charOffset: 9 })
  })

  it('should return null for missing text', () => {
    const paragraphs = [{ index: 0, text: 'hello' }]
    expect(findTextInParagraphs(paragraphs, 'world')).toBeNull()
  })
})
