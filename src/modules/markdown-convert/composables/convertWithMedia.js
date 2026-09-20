import { tauriCallSafe } from '../../../core/tauriBridge.js'

export async function convertWithMedia(command, args, onProgress = () => {}) {
  const prepared = await tauriCallSafe(
    'prepare_markdown_media',
    command === 'convert_markdown_text'
      ? { text: args.text }
      : { input: args.input, inputEncoding: args.inputEncoding },
  )
  if (!prepared.ok) return prepared
  try {
    const total = prepared.data?.items?.length || 0
    onProgress(0, total)
    let renderedMedia
    if (prepared.data?.items?.length) {
      const { renderMarkdownMedia } = await import('./renderMarkdownMedia.js')
      renderedMedia = await renderMarkdownMedia(prepared.data, onProgress)
    }
    onProgress(total, total)
    return await tauriCallSafe(command, renderedMedia ? { ...args, renderedMedia } : args)
  } catch (error) {
    return { ok: false, error: String(error.message || error) }
  }
}
