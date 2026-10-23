import { listen } from '@tauri-apps/api/event'

export const DESKTOP_ONLY_MESSAGE = '当前为浏览器界面预览，本地 PDF 预览与文件处理请在 Docsy 桌面应用中使用。'
export function hasDesktopRuntime() {
  return typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__?.invoke === 'function'
}
export async function listenDesktop(event, handler, options) {
  if (!hasDesktopRuntime()) return () => {}
  return listen(event, handler, options)
}
