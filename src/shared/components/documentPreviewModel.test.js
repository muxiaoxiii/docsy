import { describe, expect, it } from 'vitest'
import { buildPreviewParagraphs } from './documentPreviewModel.js'

describe('buildPreviewParagraphs', () => {
  it('按 Word 段落分组并保留 run 格式', () => {
    const paragraphs = buildPreviewParagraphs([
      { id: 'a', part: 'word/document.xml', paragraphIndex: 0, text: '标题', bold: true },
      { id: 'b', part: 'word/document.xml', paragraphIndex: 1, text: '正文', underline: true },
    ])

    expect(paragraphs).toHaveLength(2)
    expect(paragraphs[0].segments[0]).toMatchObject({ text: '标题', bold: true })
    expect(paragraphs[1].segments[0]).toMatchObject({ text: '正文', underline: true })
  })

  it('叠加字段值时保留字段所在 run 的格式和前后原文', () => {
    const paragraphs = buildPreviewParagraphs(
      [{ id: 'a', paragraphIndex: 0, text: '姓名张三律师', italic: true }],
      [{ runId: 'a', start: 2, end: 4, label: '李四', type: 'text', filled: true }],
      'fill',
    )

    expect(paragraphs[0].segments.map((item) => item.text)).toEqual(['姓名', '李四', '律师'])
    expect(paragraphs[0].segments[1]).toMatchObject({ italic: true, overlay: { filled: true } })
  })

  it('相邻 overlay 分别保留标签和字段身份', () => {
    const paragraphs = buildPreviewParagraphs(
      [{ id: 'a', paragraphIndex: 0, text: 'abcdef' }],
      [
        { runId: 'a', start: 0, end: 3, label: '甲方', fieldId: 'first' },
        { runId: 'a', start: 3, end: 6, label: '乙方', fieldId: 'second' },
      ],
      'fill',
    )

    expect(paragraphs[0].segments.map((item) => item.text)).toEqual(['甲方', '乙方'])
    expect(paragraphs[0].segments.map((item) => item.overlay?.fieldId)).toEqual(['first', 'second'])
  })

  it('重叠 overlay 仅裁掉已覆盖的标签片段并保留后一个字段身份', () => {
    const paragraphs = buildPreviewParagraphs(
      [{ id: 'a', paragraphIndex: 0, text: 'abcdef' }],
      [
        { runId: 'a', start: 0, end: 5, label: 'FIRST', fieldId: 'first' },
        { runId: 'a', start: 2, end: 6, label: 'SECOND', fieldId: 'second' },
      ],
      'fill',
    )

    expect(paragraphs[0].segments.map((item) => item.text)).toEqual(['FIRST', 'ND'])
    expect(paragraphs[0].segments[1].overlay.fieldId).toBe('second')
    expect(paragraphs[0].segments[1]).toMatchObject({ start: 5, end: 6 })
  })
})
