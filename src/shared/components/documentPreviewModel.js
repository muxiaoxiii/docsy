function chars(value) {
  return Array.from(String(value || ''))
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, Number(value) || 0))
}

export function buildPreviewParagraphs(runs, overlays = [], mode = 'fields') {
  const overlaysByRun = new Map()
  if (mode !== 'original') {
    for (const overlay of overlays || []) {
      if (!overlay?.runId) continue
      const list = overlaysByRun.get(overlay.runId) || []
      list.push(overlay)
      overlaysByRun.set(overlay.runId, list)
    }
  }

  const paragraphs = []
  let current = null
  for (const [runOrder, run] of (runs || []).entries()) {
    const paragraphKey = `${run.part || 'word/document.xml'}:${run.paragraphIndex ?? 0}`
    if (!current || current.key !== paragraphKey) {
      current = { key: paragraphKey, segments: [] }
      paragraphs.push(current)
    }

    const source = chars(run.text)
    const runOverlays = [...(overlaysByRun.get(run.id) || [])].sort((a, b) => (a.start ?? 0) - (b.start ?? 0))

    let cursor = 0
    for (const [overlayOrder, overlay] of runOverlays.entries()) {
      const overlayStart = clamp(overlay.start ?? 0, 0, source.length)
      const overlayEnd = clamp(overlay.end ?? source.length, overlayStart, source.length)
      const start = Math.max(cursor, overlayStart)
      const end = Math.max(start, overlayEnd)
      if (end <= cursor) continue
      if (start > cursor) {
        current.segments.push(makeSegment(run, source.slice(cursor, start).join(''), cursor, start, null, runOrder))
      }
      current.segments.push(
        makeSegment(
          run,
          visibleOverlayLabel(overlay, source, overlayStart, overlayEnd, start, end),
          start,
          end,
          overlay,
          `${runOrder}-${overlayOrder}`,
        ),
      )
      cursor = end
    }
    if (cursor < source.length || (!source.length && !runOverlays.length)) {
      current.segments.push(makeSegment(run, source.slice(cursor).join(''), cursor, source.length, null, runOrder))
    }
  }
  return paragraphs
}

function visibleOverlayLabel(overlay, source, overlayStart, overlayEnd, visibleStart, visibleEnd) {
  const fallback = source.slice(overlayStart, overlayEnd).join('')
  const label = chars(overlay.label ?? fallback)
  const sourceLength = overlayEnd - overlayStart
  if (visibleStart === overlayStart || sourceLength <= 0) return label.join('')

  // Overlapping ranges share a linear preview. Clip the already-covered part
  // from the later label instead of rendering the full replacement again.
  const visibleOffset = visibleStart - overlayStart
  const labelStart = Math.min(label.length, Math.floor((visibleOffset / sourceLength) * label.length))
  const visibleLength = visibleEnd - visibleStart
  const labelEnd = Math.min(
    label.length,
    Math.max(labelStart, Math.ceil(((visibleOffset + visibleLength) / sourceLength) * label.length)),
  )
  return label.slice(labelStart, labelEnd).join('')
}

function makeSegment(run, text, start, end, overlay, order) {
  return {
    id: `${run.id || 'run'}:${start}:${end}:${order}`,
    text,
    runId: run.id || '',
    start,
    end,
    overlay,
    bold: Boolean(run.bold),
    italic: Boolean(run.italic),
    underline: Boolean(run.underline),
  }
}
