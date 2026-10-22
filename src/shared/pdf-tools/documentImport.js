export const IMAGE_DOCUMENT_EXTENSIONS = ['jpg', 'jpeg', 'png', 'webp', 'bmp']
export const DOCUMENT_FILTERS = [
  { name: 'PDF、Word 与图片', extensions: ['pdf', 'docx', 'doc', 'docm', ...IMAGE_DOCUMENT_EXTENSIONS] },
  { name: 'PDF', extensions: ['pdf'] },
  { name: 'Word', extensions: ['docx', 'doc', 'docm'] },
  { name: '图片', extensions: IMAGE_DOCUMENT_EXTENSIONS },
]
export function documentKind(path) {
  const ext = String(path || '').split('.').at(-1).toLowerCase()
  if (ext === 'pdf') return 'pdf'
  if (['docx', 'doc', 'docm'].includes(ext)) return 'word'
  if (IMAGE_DOCUMENT_EXTENSIONS.includes(ext)) return 'image'
  return null
}
export const MANUAL_PDF_HINT = '请用 Word 或 WPS 将原文档另存为 PDF，再选择转换后的 PDF 继续；原文件和已添加文件都会保留。'

export async function convertImportDocument(path, invoke) {
  const kind = documentKind(path)
  if (kind === 'pdf') return path
  if (!kind) throw new Error('不支持的文件类型')
  const result = await invoke(kind === 'word' ? 'convert_word_to_pdf' : 'convert_image_to_pdf', { input: path })
  if (!result.ok) throw new Error(result.error || '转换失败')
  const output = result.data?.output_path || result.data
  if (typeof output !== 'string' || !/\.pdf$/i.test(output)) throw new Error('转换未返回有效的 PDF 文件')
  return output
}

/** Append imports without resetting existing page ranges, rules or ordering. */
export function appendUniqueDocuments(existing, incoming, pathOf = item => item.path) {
  const seen = new Set(existing.flatMap(item => [pathOf(item), item.sourcePath].filter(Boolean)))
  return [...existing, ...incoming.filter(item => {
    const paths = [pathOf(item), item.sourcePath].filter(Boolean)
    if (paths.some(path => seen.has(path))) return false
    paths.forEach(path => seen.add(path))
    return true
  })]
}

/** Keep mixed-file order; consecutive images become a reviewed PDF batch. */
export async function reviewImageImports(paths, review) {
  const result = []
  for (let index = 0; index < paths.length;) {
    if (documentKind(paths[index]) !== 'image') { result.push(paths[index++]); continue }
    const group = []
    while (index < paths.length && documentKind(paths[index]) === 'image') group.push(paths[index++])
    const pdf = await review(group)
    if (pdf) result.push(pdf)
  }
  return result
}
