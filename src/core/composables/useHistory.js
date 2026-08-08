import { ref, computed } from 'vue'

/**
 * Generic undo/redo history composable.
 *
 * @param {Object} options
 * @param {Function} options.snapshot - Returns a deep-cloned snapshot of current state
 * @param {Function} options.restore  - Restores state from a snapshot
 * @param {number}   [options.maxSteps=20] - Max history depth
 * @param {Function} [options.onChange] - Called after undo/redo restores state
 * @returns {Object} { push, undo, redo, canUndo, canRedo, clear, length }
 */
export function useHistory({ snapshot, restore, maxSteps = 20, onChange }) {
  const stack = ref([])
  const index = ref(-1)

  /** Call after every user action to record a new snapshot. */
  function push() {
    const snap = snapshot()
    if (snap === undefined || snap === null) return
    // Truncate redo branch
    stack.value = stack.value.slice(0, index.value + 1)
    stack.value.push(snap)
    // Enforce max depth
    if (stack.value.length > maxSteps) {
      stack.value.shift()
    }
    index.value = stack.value.length - 1
  }

  function undo() {
    if (!canUndo.value) return
    index.value--
    restore(stack.value[index.value])
    onChange?.()
  }

  function redo() {
    if (!canRedo.value) return
    index.value++
    restore(stack.value[index.value])
    onChange?.()
  }

  const canUndo = computed(() => index.value > 0)
  const canRedo = computed(() => index.value < stack.value.length - 1)
  const length = computed(() => stack.value.length)

  function clear() {
    stack.value = []
    index.value = -1
  }

  return { push, undo, redo, canUndo, canRedo, clear, length }
}

/**
 * Convenience: build a snapshot/restore pair from a reactive object's getter/setter.
 *
 * Usage:
 *   const params = useHistory(forwardRef(headerParams, v => Object.assign(headerParams, v)))
 *
 * Or inline:
 *   useHistory({
 *     snapshot: () => ({ ...headerParams }),
 *     restore: (s) => Object.assign(headerParams, s),
 *   })
 */
export function forwardRef(getter, setter) {
  return {
    snapshot: () => JSON.parse(JSON.stringify(getter())),
    restore: (snap) => setter(JSON.parse(JSON.stringify(snap))),
  }
}
