export function fileName(path) {
  return (
    String(path || '')
      .split(/[\\/]/)
      .pop() || path
  )
}

export function parentDir(path) {
  const value = String(path || '')
  const idx = Math.max(value.lastIndexOf('/'), value.lastIndexOf('\\'))
  return idx >= 0 ? value.slice(0, idx) : '.'
}

export function stripExtension(name, extensionPattern) {
  return String(name || '').replace(extensionPattern, '')
}

export function stripPdf(name) {
  return stripExtension(name, /\.pdf$/i)
}

export function getExtension(path) {
  const name = fileName(path)
  const idx = name.lastIndexOf('.')
  return idx >= 0 ? name.slice(idx + 1).toLowerCase() : ''
}
