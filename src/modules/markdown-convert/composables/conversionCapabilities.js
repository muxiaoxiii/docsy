// Keep this list aligned with OfficeOutputFormat::parse in markdown/mod.rs.
// PDF is an input-only text extraction path, not a Markdown export format.
export const MARKDOWN_OUTPUT_FORMATS = Object.freeze([
  { value: 'docx', label: 'Word' },
  { value: 'xlsx', label: 'Excel' },
  { value: 'pptx', label: 'PowerPoint' },
  { value: 'html', label: '网页' },
])

export const MARKDOWN_CONVERSION_DESCRIPTION =
  'Markdown 可生成 Word、Excel、PowerPoint 或 HTML；Office/HTML 可提取为 Markdown，PDF 仅提取已有文本层。'
export const MARKDOWN_SCENE_DESCRIPTION =
  'Markdown 导出 Word / Excel / PPT / HTML；Office、HTML、PDF 可提取为 Markdown，文件转换结果保存在源文件旁。'
export const PDF_CONVERSION_LIMIT = 'PDF 仅支持文本提取，不含 OCR、版面还原或表格识别；本页暂不导出 PDF。'
export const OFFICE_EXTRACTION_LIMIT = 'Office 转 Markdown 提取可表达的内容和结构，不保证保留原文档的精确版式。'

export function markdownOutputError(format = 'docx') {
  if (MARKDOWN_OUTPUT_FORMATS.some((option) => option.value === format)) return ''
  return format === 'pdf'
    ? '当前 MD 转换不支持导出 PDF；可导出 Word、Excel、PowerPoint 或 HTML。PDF 导入仅提取已有文本层。'
    : `不支持的 Markdown 输出格式：${String(format)}。可导出 Word、Excel、PowerPoint 或 HTML。`
}

export function markdownMediaHint(format, styleHint = '') {
  switch (format) {
    case 'docx':
      return `${styleHint ? styleHint + '；' : ''}支持的公式生成可编辑 Word 公式，不支持的公式回退为图片；Mermaid 图表以图片嵌入。`
    case 'xlsx':
      return '数学公式与 Mermaid 图以图片放入独立工作表，不会转换为 Excel 计算公式或原生图表。'
    case 'pptx':
      return '数学公式与 Mermaid 图以图片放入独立幻灯片，不会转换为可编辑的 PowerPoint 公式或原生图表。'
    case 'html':
      return '数学公式与 Mermaid 图以渲染图片嵌入 HTML 网页。'
    default:
      return markdownOutputError(format)
  }
}
