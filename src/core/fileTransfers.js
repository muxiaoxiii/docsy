import { MARKDOWN_INPUT_EXTENSIONS } from '../modules/markdown-convert/composables/conversionCapabilities.js'

const PDF = ['pdf'],
  DOCUMENTS = ['pdf', 'doc', 'docx', 'docm', 'png', 'jpg', 'jpeg', 'webp', 'bmp']
const IMAGES = ['png', 'jpg', 'jpeg', 'webp', 'bmp']
export const FILE_TRANSFER_TARGETS = Object.freeze([
  { id: 'pdf-merge', label: 'PDF 合并', route: { name: 'pdf-tools', params: { tab: 'merge' } }, extensions: DOCUMENTS },
  { id: 'evidence-merge', label: '分项证据合并', route: { name: 'evidence-pdf' }, extensions: DOCUMENTS },
  {
    id: 'pdf-crop',
    label: 'PDF 页面裁剪',
    route: { name: 'pdf-tools', params: { tab: 'crop' } },
    extensions: PDF,
    single: true,
  },
  {
    id: 'pdf-extract',
    label: '提取 PDF 页面',
    route: { name: 'pdf-tools', params: { tab: 'extract' } },
    extensions: PDF,
    single: true,
  },
  {
    id: 'pdf-split',
    label: 'PDF 拆分',
    route: { name: 'pdf-tools', params: { tab: 'split' } },
    extensions: PDF,
    single: true,
  },
  { id: 'pdf-compress', label: 'PDF 压缩', route: { name: 'pdf-tools', params: { tab: 'compress' } }, extensions: PDF },
  {
    id: 'long-slice',
    label: '长图处理',
    route: { name: 'pdf-tools', params: { tab: 'long-slice' } },
    extensions: [...PDF, ...IMAGES],
  },
  { id: 'markdown', label: 'MD 转换', route: { name: 'markdown-convert' }, extensions: MARKDOWN_INPUT_EXTENSIONS },
  { id: 'image-layout', label: '图片排版', route: { name: 'image-paddler' }, extensions: IMAGES },
])

export function transferPaths(paths) {
  return [
    ...new Set(
      (Array.isArray(paths) ? paths : [paths]).filter((p) => typeof p === 'string' && p.trim() && !p.includes('\0')),
    ),
  ]
}
export function transferTargets(paths, exclude = '') {
  const files = transferPaths(paths)
  if (!files.length) return []
  return FILE_TRANSFER_TARGETS.filter(
    (t) =>
      t.id !== exclude &&
      (!t.single || files.length === 1) &&
      files.every((p) => t.extensions.includes(p.split('.').pop().toLowerCase())),
  )
}
export function createFileTransfer(paths, target, source = '') {
  const files = transferPaths(paths)
  const match = transferTargets(files).find((t) => t.id === target)
  if (!match) throw new Error('这些文件不能一起导入所选工具')
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    target,
    paths: files,
    source,
    createdAt: new Date().toISOString(),
  }
}
