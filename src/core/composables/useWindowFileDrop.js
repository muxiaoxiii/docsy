import { getCurrentInstance, onActivated, onBeforeUnmount, onDeactivated, onMounted } from 'vue'
import { getCurrentWebview } from '@tauri-apps/api/webview'

export function useWindowFileDrop({ onEnter, onLeave, onDrop, onError } = {}) {
  const instance = getCurrentInstance()
  let unlisten = null
  let disposed = false
  let active = true

  onMounted(async () => {
    disposed = false
    try {
      const dispose = await getCurrentWebview().onDragDropEvent(async (event) => {
        if (disposed || !active) return
        const element = instance?.proxy?.$el
        if (element instanceof globalThis.Element && !element.getClientRects().length) return
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
      if (disposed) {
        dispose()
      } else {
        unlisten = dispose
      }
    } catch (error) {
      if (!disposed) onError?.(error)
    }
  })

  onActivated(() => {
    active = true
  })

  onDeactivated(() => {
    active = false
    onLeave?.()
  })

  onBeforeUnmount(() => {
    disposed = true
    unlisten?.()
    unlisten = null
  })
}
