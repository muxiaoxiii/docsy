const OVERLAY_COLLISION_RE = /^第\s*(\d+)\s*页的“(.+?)”与其他页眉页脚位置重叠，将按实际位置叠加渲染$/

/**
 * Summarize processing warnings by type for completion notifications.
 * File rows still retain their full per-page diagnostics; this keeps a large
 * evidence batch from producing an unreadable notification.
 */
export function formatProcessingWarningSummary(results, maxGroups = 5) {
  const grouped = new Map()

  for (const result of results || []) {
    const name =
      String(result?.inputPath || '')
        .split(/[/\\]/)
        .pop() || '未知文件'
    for (const warning of result?.warnings || []) {
      const text = String(warning || '').trim()
      if (!text) continue
      const collision = text.match(OVERLAY_COLLISION_RE)
      const key = collision ? `collision:${collision[2]}` : `warning:${text}`
      const group = grouped.get(key) || {
        type: collision ? 'collision' : 'warning',
        text: collision ? collision[2] : text,
        pages: 0,
        files: new Set(),
      }
      group.files.add(name)
      if (collision) group.pages += 1
      grouped.set(key, group)
    }
  }

  const lines = [...grouped.values()].map((group) => {
    if (group.type === 'collision') {
      return `位置重叠：${group.files.size} 个文件、${group.pages} 页的“${group.text}”。已按原位置写入。`
    }
    return `${group.files.size} 个文件：${group.text}`
  })

  if (lines.length <= maxGroups) return lines
  return [...lines.slice(0, maxGroups), `其余 ${lines.length - maxGroups} 类提示请在文件列表中查看`]
}
