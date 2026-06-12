import { computed } from 'vue'

export function usePreview(session) {
  const segments = computed(() => {
    if (!session.value) return []
    const text = session.value.plain_text || ''
    const marks = session.value.marks || []
    if (!text) return []

    const sorted = [...marks].sort((a, b) => a.start - b.start)
    const result = []
    let cursor = 0

    for (const mark of sorted) {
      const mStart = Math.max(mark.start, 0)
      const mEnd = Math.min(mark.end, text.length)
      if (mStart > mEnd || mStart < cursor) continue

      if (mStart > cursor) {
        result.push({
          type: 'text',
          text: text.slice(cursor, mStart),
          start: cursor,
          end: mStart,
        })
      }

      result.push({
        type: 'mark',
        text: text.slice(mStart, mEnd),
        start: mStart,
        end: mEnd,
        markId: mark.id,
        fieldKey: mark.fieldKey,
      })

      cursor = mEnd
    }

    if (cursor < text.length) {
      result.push({
        type: 'text',
        text: text.slice(cursor),
        start: cursor,
        end: text.length,
      })
    }

    return result
  })

  const paragraphs = computed(() => {
    const paras = []
    let current = []
    let offset = 0

    for (const seg of segments.value) {
      const parts = seg.text.split('\n')
      for (let i = 0; i < parts.length; i++) {
        if (i > 0) {
          paras.push(current)
          current = []
        }
        if (parts[i]) {
          const partStart = seg.start + offset
          current.push({
            ...seg,
            text: parts[i],
            start: partStart,
            end: partStart + parts[i].length,
          })
          offset += parts[i].length
        }
        offset += 1
      }
    }
    if (current.length) paras.push(current)
    return paras
  })

  return { segments, paragraphs }
}
