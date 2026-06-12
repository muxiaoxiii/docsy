import { ref, computed } from 'vue'

let nextMarkId = 1

export function useMarks(session) {
  const activeMarkId = ref(null)

  const marks = computed({
    get: () => session.value?.marks || [],
    set: (val) => {
      if (session.value) session.value.marks = val
    },
  })

  const fields = computed({
    get: () => session.value?.fields || [],
    set: (val) => {
      if (session.value) session.value.fields = val
    },
  })

  const activeMark = computed(() =>
    marks.value.find((m) => m.id === activeMarkId.value) || null,
  )

  function addMark(start, end, fieldConfig) {
    if (!session.value) return null
    const id = nextMarkId++
    const fieldKey = fieldConfig.key || autoKey(fieldConfig.label, fields.value)

    const mark = { id, start, end, fieldKey }
    session.value.marks = [...session.value.marks, mark]

    const existingField = session.value.fields.find((f) => f.key === fieldKey)
    if (!existingField) {
      session.value.fields = [
        ...session.value.fields,
        { key: fieldKey, ...fieldConfig },
      ]
    }

    return mark
  }

  function updateMark(id, updates) {
    if (!session.value) return
    const mark = marks.value.find((m) => m.id === id)
    if (!mark) return

    if (updates.start !== undefined) mark.start = updates.start
    if (updates.end !== undefined) mark.end = updates.end

    if (updates.fieldConfig) {
      const field = session.value.fields.find((f) => f.key === mark.fieldKey)
      if (field) {
        Object.assign(field, updates.fieldConfig)
      }
    }

    session.value.marks = [...session.value.marks]
    session.value.fields = [...session.value.fields]
  }

  function removeMark(id) {
    if (!session.value) return
    const mark = marks.value.find((m) => m.id === id)
    if (!mark) return

    session.value.marks = session.value.marks.filter((m) => m.id !== id)

    const stillUsed = session.value.marks.some(
      (m) => m.fieldKey === mark.fieldKey,
    )
    if (!stillUsed) {
      session.value.fields = session.value.fields.filter(
        (f) => f.key !== mark.fieldKey,
      )
    }

    if (activeMarkId.value === id) activeMarkId.value = null
  }

  function selectMark(id) {
    activeMarkId.value = id
  }

  function clearSelection() {
    activeMarkId.value = null
  }

  function markOverlaps(start, end) {
    return marks.value.some((m) => m.start < end && m.end > start)
  }

  return {
    marks,
    fields,
    activeMarkId,
    activeMark,
    addMark,
    updateMark,
    removeMark,
    selectMark,
    clearSelection,
    markOverlaps,
  }
}

function autoKey(label, existingFields) {
  const base = (label || 'field')
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff]/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_|_$/g, '')
    || 'field'
  let key = base
  let i = 2
  while (existingFields.some((f) => f.key === key)) {
    key = `${base}_${i++}`
  }
  return key
}
