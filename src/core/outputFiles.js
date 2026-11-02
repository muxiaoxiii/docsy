import { fileName } from './filePath.js'

export function commonSourceDirectory(files) {
  const paths = files.flatMap((file) =>
    typeof file === 'string' ? [file] : file.sourcePaths?.length ? file.sourcePaths : [file.sourcePath || file.path],
  )
  if (!paths.length || paths.some((path) => !path)) return ''
  const directories = paths.map((path) => {
    const value = String(path),
      index = Math.max(value.lastIndexOf('/'), value.lastIndexOf('\\'))
    if (index < 0) return ''
    return index === 0 || (index === 2 && /^[A-Za-z]:/.test(value)) ? value.slice(0, index + 1) : value.slice(0, index)
  })
  const key = (dir) => {
    const normalized = dir.replace(/\\/g, '/').replace(/\/$/, '')
    return /^[A-Za-z]:/.test(dir) || /^[/\\]{2}/.test(dir) ? normalized.toLowerCase() : normalized
  }
  return directories[0] && directories.every((dir) => key(dir) === key(directories[0])) ? directories[0] : ''
}

export function pdfNameFromSource(path) {
  return `${fileName(path).replace(/\.[^.]+$/, '')}.pdf`
}

export function pdfOutputFileName(value, fallback = '合并文件.pdf') {
  const name = String(value || '').trim() || fallback
  if (/[<>:"/\\|?*]/.test(name) || [...name].some((char) => char.charCodeAt(0) < 32) || /\.$/.test(name))
    throw new Error('文件名不能包含路径或 < > : " / \\ | ? *，也不能以句点结尾')
  const result = /\.pdf$/i.test(name) ? name : `${name}.pdf`
  const stem = result.slice(0, -4)
  if (!stem || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(stem)) throw new Error('请输入可用的 PDF 文件名')
  return result
}

export function joinOutputPath(directory, name) {
  const separator = directory.includes('\\') && !directory.includes('/') ? '\\' : '/'
  return `${directory}${/[/\\]$/.test(directory) ? '' : separator}${name}`
}
