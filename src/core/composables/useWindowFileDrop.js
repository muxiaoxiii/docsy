import { onBeforeUnmount, onMounted } from 'vue'
import { getCurrentWebview } from '@tauri-apps/api/webview'

export function useWindowFileDrop({ onEnter, onLeave, onDrop, onError } = {}) {
  let unlisten = null

  onMounted(async () => {
    try {
      unlisten = await getCurrentWebview().onDragDropEvent(async (event) => {
        const type = event.payload.type
        if (type === 'enter' || type === 'over') {
          onEnter?.(event.payload.paths || [])
          return
        }
        if (type === 'drop') {
          onLeave?.()
          await onDrop?.(event.payload.paths || [])
          return
        }
        onLeave?.()
      })
    } catch (error) {
      onError?.(error)
    }
  })

  onBeforeUnmount(() => {
    unlisten?.()
    unlisten = null
  })
}
