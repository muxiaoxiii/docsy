import { defineStore } from 'pinia'
import { ref } from 'vue'

const TRANSFER_KEY = 'docsy.workspace.media-transfer'
const FRAME_DRAFT_KEY = 'docsy.workspace.frame-selection-draft'

function readSessionValue(key) {
  if (typeof window === 'undefined') return null
  try {
    return JSON.parse(window.sessionStorage.getItem(key) || 'null')
  } catch {
    return null
  }
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const transferValue = readSessionValue(TRANSFER_KEY)
  const draftValue = readSessionValue(FRAME_DRAFT_KEY)
  const mediaTransfer = ref(Array.isArray(transferValue?.paths) && transferValue.paths.length ? transferValue : null)
  const frameSelectionDraft = ref(Array.isArray(draftValue?.items) && draftValue.items.length ? draftValue : null)

  function sendImagesToLayout(paths, sourceSessionId = null, context = {}) {
    const normalized = [...new Set((paths || []).filter(Boolean))]
    mediaTransfer.value = normalized.length
      ? {
          paths: normalized,
          sourceSessionId,
          sourceKind: context.sourceKind || 'images',
          sourceLabel: context.sourceLabel || '',
          sourceStem: context.sourceStem || '',
          selection: Array.isArray(context.selection)
            ? context.selection
                .filter((item) => item?.path)
                .map((item) => ({
                  path: item.path,
                  decision: item.decision || 'review',
                  reason: item.reason || '',
                }))
            : [],
          createdAt: new Date().toISOString(),
        }
      : null
    if (typeof window !== 'undefined') {
      try {
        if (mediaTransfer.value) {
          window.sessionStorage.setItem(TRANSFER_KEY, JSON.stringify(mediaTransfer.value))
        } else {
          window.sessionStorage.removeItem(TRANSFER_KEY)
        }
      } catch {
        window.sessionStorage.removeItem(TRANSFER_KEY)
      }
    }
  }

  function clearMediaTransfer() {
    mediaTransfer.value = null
    if (typeof window !== 'undefined') window.sessionStorage.removeItem(TRANSFER_KEY)
  }

  function setFrameSelectionDraft(draft) {
    frameSelectionDraft.value = Array.isArray(draft?.items) && draft.items.length ? draft : null
    if (typeof window === 'undefined') return

    const trySave = () => {
      const serialized = frameSelectionDraft.value ? JSON.stringify(frameSelectionDraft.value) : ''
      if (serialized && serialized.length <= 3_000_000) {
        window.sessionStorage.setItem(FRAME_DRAFT_KEY, serialized)
      } else {
        window.sessionStorage.removeItem(FRAME_DRAFT_KEY)
      }
    }

    try {
      trySave()
    } catch (e) {
      console.warn('存储草稿失败，清理旧草稿后重试', e)
      window.sessionStorage.removeItem(FRAME_DRAFT_KEY)
      try {
        trySave()
      } catch (e2) {
        console.warn('清理后存储仍失败', e2)
        window.sessionStorage.removeItem(FRAME_DRAFT_KEY)
      }
    }
  }

  function clearFrameSelectionDraft() {
    setFrameSelectionDraft(null)
  }

  return {
    mediaTransfer,
    frameSelectionDraft,
    sendImagesToLayout,
    clearMediaTransfer,
    setFrameSelectionDraft,
    clearFrameSelectionDraft,
  }
})
