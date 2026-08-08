export function moveItem(items, from, to) {
  if (!Array.isArray(items)) return []
  const result = [...items]
  if (from === to || from < 0 || to < 0 || from >= result.length || to >= result.length) return result
  const [item] = result.splice(from, 1)
  result.splice(to, 0, item)
  return result
}
