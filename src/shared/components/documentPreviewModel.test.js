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
})
