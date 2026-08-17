import { ref } from 'vue'

export function reorderTargetIndex(from, target, placement, itemCount) {
  if (from < 0 || target < 0 || itemCount <= 0 || from >= itemCount || target >= itemCount) return -1
  let insertion = target + (placement === 'after' ? 1 : 0)
  if (from < insertion) insertion -= 1
  return Math.max(0, Math.min(itemCount - 1, insertion))
}

export function usePointerReorder({
  itemCount,
  onReorder,
  onStart,
  onEnd,
  onCancel,
  itemAttribute = 'data-reorder-index',
}) {
  const dragFrom = ref(-1)
  const dragOver = ref(-1)
  const dragPlacement = ref('before')
  let activePointerId = null
  let activeHandle = null

  function start(index, event) {
    if (event?.button !== undefined && event.button !== 0) return
    dragFrom.value = index
    dragOver.value = index
    dragPlacement.value = 'before'
    activePointerId = event?.pointerId ?? null
    activeHandle = event?.currentTarget || null
    activeHandle?.setPointerCapture?.(activePointerId)
    event?.preventDefault?.()
    onStart?.({ from: index })
  }

  function move(event) {
    if (dragFrom.value < 0 || (activePointerId !== null && event?.pointerId !== activePointerId)) return
    const element = document.elementFromPoint(event.clientX, event.clientY)?.closest?.(`[${itemAttribute}]`)
    if (!element) return
    const index = Number(element.getAttribute(itemAttribute))
    if (!Number.isInteger(index)) return
    const rect = element.getBoundingClientRect()
    dragOver.value = index
    dragPlacement.value = event.clientY >= rect.top + rect.height / 2 ? 'after' : 'before'
    event.preventDefault()
  }

  function finish(event) {
    if (dragFrom.value < 0 || (activePointerId !== null && event?.pointerId !== activePointerId)) return
    const from = dragFrom.value
    const count = Number(typeof itemCount === 'function' ? itemCount() : itemCount || 0)
    const to = reorderTargetIndex(from, dragOver.value, dragPlacement.value, count)
    const reordered = to >= 0 && to !== from
    try {
      if (reordered) onReorder?.({ from, to })
    } finally {
      clearState()
      onEnd?.({ from, to, reordered })
    }
  }

  function reset() {
    const from = dragFrom.value
    const wasDragging = from >= 0
    clearState()
    if (wasDragging) onCancel?.({ from })
  }

  function clearState() {
    if (activeHandle && activePointerId !== null) {
      try {
        activeHandle.releasePointerCapture?.(activePointerId)
      } catch {
        // The browser may release capture automatically when the pointer ends.
      }
    }
    dragFrom.value = -1
    dragOver.value = -1
    dragPlacement.value = 'before'
    activePointerId = null
    activeHandle = null
  }

  function itemClasses(index) {
    return {
      'is-reorder-dragging': dragFrom.value === index,
      'is-reorder-before': dragOver.value === index && dragFrom.value !== index && dragPlacement.value === 'before',
      'is-reorder-after': dragOver.value === index && dragFrom.value !== index && dragPlacement.value === 'after',
    }
  }

  return { dragFrom, dragOver, dragPlacement, start, move, finish, reset, itemClasses }
}
