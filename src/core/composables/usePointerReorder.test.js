import { describe, expect, it, vi } from 'vitest'
import { reorderTargetIndex, usePointerReorder } from './usePointerReorder.js'

describe('reorderTargetIndex', () => {
  it('moves the first item after the last item', () => {
    expect(reorderTargetIndex(0, 2, 'after', 3)).toBe(2)
  })

  it('moves the last item before the first item', () => {
    expect(reorderTargetIndex(2, 0, 'before', 3)).toBe(0)
  })

  it('accounts for removing the source before insertion', () => {
    expect(reorderTargetIndex(0, 2, 'before', 3)).toBe(1)
    expect(reorderTargetIndex(2, 0, 'after', 3)).toBe(1)
  })
})

describe('usePointerReorder lifecycle', () => {
  it('commits one reorder between start and end', () => {
    const events = []
    const onReorder = vi.fn(({ from, to }) => events.push(`reorder:${from}:${to}`))
    const reorder = usePointerReorder({
      itemCount: () => 3,
      onStart: ({ from }) => events.push(`start:${from}`),
      onReorder,
      onEnd: ({ reordered }) => events.push(`end:${reordered}`),
    })

    reorder.start(0, { button: 0, pointerId: 1, preventDefault: vi.fn() })
    reorder.dragOver.value = 2
    reorder.dragPlacement.value = 'after'
    reorder.finish({ pointerId: 1 })

    expect(events).toEqual(['start:0', 'reorder:0:2', 'end:true'])
    expect(onReorder).toHaveBeenCalledTimes(1)
    expect(reorder.dragFrom.value).toBe(-1)
  })

  it('reports cancellation without committing a reorder', () => {
    const onReorder = vi.fn()
    const onCancel = vi.fn()
    const reorder = usePointerReorder({ itemCount: 2, onReorder, onCancel })

    reorder.start(1, { button: 0, pointerId: 4, preventDefault: vi.fn() })
    reorder.reset()

    expect(onReorder).not.toHaveBeenCalled()
    expect(onCancel).toHaveBeenCalledWith({ from: 1 })
  })
})
