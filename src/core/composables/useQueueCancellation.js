import { onBeforeUnmount, onMounted, ref } from 'vue'

export function useQueueCancellation() {
  const cancelled = ref(false)
  const cancel = () => { cancelled.value = true }
  const reset = () => { cancelled.value = false }
  onMounted(() => window.addEventListener('docsy-cancel-requested', cancel))
  onBeforeUnmount(() => {
    cancel()
    window.removeEventListener('docsy-cancel-requested', cancel)
  })
  return { cancelled, cancel, reset }
}
