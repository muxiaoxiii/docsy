import { ref } from 'vue'

export function reorderTargetIndex(from, target, placement, itemCount) {
  if (from < 0 || target < 0 || itemCount <= 0 || from >= itemCount || target >= itemCount) return -1
  let insertion = target + (placement === 'after' ? 1 : 0)
  if (from < insertion) insertion -= 1
  return Math.max(0, Math.min(itemCount - 1, insertion))
}

export function usePointerReorder({ itemCount, onReorder, itemAttribute = 'data-reorder-index' }) {
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
    const count = Number(typeof itemCount === 'function' ? itemCount() : itemCount || 0)
    const to = reorderTargetIndex(dragFrom.value, dragOver.value, dragPlacement.value, count)
    if (to >= 0 && to !== dragFrom.value) onReorder?.({ from: dragFrom.value, to })
    reset()
  }

  function reset() {
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
