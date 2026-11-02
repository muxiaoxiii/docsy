/** Split a header into selectable pieces while retaining all original text. */
export function splitHeaderTokens(text, granularity = 'word') {
  const value = String(text ?? '')
  if (!value) return []
  const mode = granularity === 'character' ? 'grapheme' : 'word'
  if (typeof Intl !== 'undefined' && typeof Intl.Segmenter === 'function') {
    try {
      const segmenter = new Intl.Segmenter('zh-Hans', { granularity: mode })
      return Array.from(segmenter.segment(value), (segment) => segment.segment)
    } catch {
      // Older runtimes can expose Segmenter without the requested locale/mode.
    }
  }
  // Code-point fallback avoids splitting surrogate pairs; concatenation always
  // reproduces the original, including marks and joined emoji sequences.
  return Array.from(value)
}

/** Join selected pieces in source order, independently of the order clicked. */
export function joinSelectedHeaderTokens(tokens, selectedIndices) {
  const selected = new Set(selectedIndices ?? [])
  return (tokens || []).filter((_, index) => selected.has(index)).join('')
}
