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
      const start = clamp(overlay.start ?? 0, cursor, source.length)
      const end = clamp(overlay.end ?? source.length, start, source.length)
      if (start > cursor) {
        current.segments.push(makeSegment(run, source.slice(cursor, start).join(''), cursor, start, null, runOrder))
      }
      current.segments.push(
        makeSegment(
          run,
          String(overlay.label ?? source.slice(start, end).join('')),
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
