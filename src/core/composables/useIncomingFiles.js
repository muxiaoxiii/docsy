import { computed, getCurrentInstance, onActivated, onDeactivated, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useWorkspaceStore } from '../../stores/workspace.js'
import { FILE_TRANSFER_TARGETS } from '../fileTransfers.js'
import { userFacingError } from '../tauriBridge.js'

// Acknowledge only the exact batch that was actually accepted. Failed or
// cancelled imports stay visible and can be retried without asking for paths.
export function useIncomingFiles(routeName, receive, { ready = () => true, busy = () => false } = {}) {
  const error = ref(''),
    receiving = ref(false),
    active = ref(true)
  const blocked = computed(() => !ready() || busy())
  if (!getCurrentInstance()) return { pending: ref(null), error, receiving, blocked, retry: async () => {}, discard: () => {} }
  const route = useRoute(),
    store = useWorkspaceStore()
  let attempted = ''
  const pending = computed(() => {
    const incoming = store.fileTransfer
    const target = FILE_TRANSFER_TARGETS.find((t) => t.id === incoming?.target)
    return target?.route.name === routeName ? incoming : null
  })
  async function consume(force = false) {
    const batch = pending.value
    if (
      !batch ||
      !active.value ||
      route.name !== routeName ||
      !ready() ||
      busy() ||
      receiving.value ||
      (!force && attempted === batch.id)
    )
      return
    attempted = batch.id
    receiving.value = true
    error.value = ''
    try {
      const accepted = await receive(batch)
      if (accepted === false) error.value = '尚未导入；可以继续导入，或取消这次流转。'
      else store.clearFileTransfer(batch.id)
    } catch (e) {
      error.value = userFacingError(e, '导入失败，文件路径已保留，可重试')
    } finally {
      receiving.value = false
    }
  }
  onMounted(() => {
    void consume()
  })
  onActivated(() => {
    active.value = true
    void consume()
  })
  onDeactivated(() => {
    active.value = false
  })
  watch(
    () => [pending.value?.id, ready(), busy(), receiving.value, route.name],
    () => {
      void consume()
    },
    { flush: 'post' },
  )
  return {
    pending,
    error,
    receiving,
    blocked,
    retry: () => consume(true),
    discard: () => {
      if (pending.value) store.clearFileTransfer(pending.value.id)
    },
  }
}
