<template>
  <aside ref="thumbnailRef" class="next-page-thumbnail" :style="thumbnailStyle" aria-label="下一页预览">
    <button
      v-if="hasNextPage"
      type="button"
      class="thumbnail-button"
      :aria-label="`前往第 ${nextPage} 页`"
      @click="emit('select', nextPage)"
    >
      <span class="thumbnail-label">P.{{ nextPage }}（下一页）</span>
      <PdfJsPreview
        v-if="renderedPage"
        :file-path="filePath"
        :page="renderedPage"
        :scale="0.55"
        compact
        @error="(message) => emit('error', message)"
      />
    </button>
    <div v-else class="thumbnail-end">已是最后一页</div>
    <div
      class="thumbnail-resize-handle"
      role="separator"
      aria-label="调整下一页预览大小；向左拖动放大，向右拖动缩小"
      aria-orientation="vertical"
      @pointerdown="startResize"
    />
  </aside>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import PdfJsPreview from './PdfJsPreview.vue'

const props = defineProps({
  filePath: { type: String, default: '' },
  page: { type: Number, default: 1 },
  maxPage: { type: Number, default: 1 },
  debounceMs: { type: Number, default: 100 },
})

const emit = defineEmits(['select', 'error'])
const renderedPage = ref(0)
const thumbnailRef = ref(null)
const widthRatio = ref(loadWidthRatio())
let timer = null
let stopResize = null

const nextPage = computed(() => Math.max(1, Number(props.page || 1)) + 1)
const hasNextPage = computed(() => Boolean(props.filePath) && nextPage.value <= Math.max(1, Number(props.maxPage || 1)))
const thumbnailStyle = computed(() => ({
  flexBasis: widthRatio.value >= 0.5 ? 'calc(50% - 6px)' : `${Math.round(widthRatio.value * 1000) / 10}%`,
}))

function loadWidthRatio() {
  const stored = window.localStorage.getItem('docsy.nextPagePreview.widthRatio')
  if (stored === null) return 0.3
  const value = Number(stored)
  return Number.isFinite(value) ? Math.min(0.5, Math.max(0.2, value)) : 0.3
}

function startResize(event) {
  if (event.button !== 0 || !thumbnailRef.value?.parentElement) return
  event.preventDefault()
  event.stopPropagation()
  const parentWidth = Math.max(1, thumbnailRef.value.parentElement.getBoundingClientRect().width)
  const startWidth = thumbnailRef.value.getBoundingClientRect().width
  const startX = event.clientX
  const update = (pointerEvent) => {
    const nextWidth = startWidth - (pointerEvent.clientX - startX)
    widthRatio.value = Math.min(0.5, Math.max(0.2, nextWidth / parentWidth))
  }
  const finish = () => {
    window.removeEventListener('pointermove', update)
    window.removeEventListener('pointerup', finish)
    window.removeEventListener('pointercancel', finish)
    window.localStorage.setItem('docsy.nextPagePreview.widthRatio', String(widthRatio.value))
    stopResize = null
  }
  stopResize?.()
  stopResize = finish
  window.addEventListener('pointermove', update)
  window.addEventListener('pointerup', finish, { once: true })
  window.addEventListener('pointercancel', finish, { once: true })
}

watch(
  () => [props.filePath, props.page, props.maxPage],
  () => {
    if (timer) window.clearTimeout(timer)
    if (!hasNextPage.value) {
      renderedPage.value = 0
      return
    }
    renderedPage.value = 0
    timer = window.setTimeout(
      () => {
        renderedPage.value = nextPage.value
        timer = null
      },
      Math.max(0, Number(props.debounceMs || 0)),
    )
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  if (timer) window.clearTimeout(timer)
  stopResize?.()
})
</script>

<style scoped>
.next-page-thumbnail {
  position: relative;
  flex: 0 0 clamp(190px, 24vw, 260px);
  min-width: 120px;
  max-width: calc(50% - 6px);
}

.thumbnail-button {
  display: block;
  width: 100%;
  padding: 8px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-elevated) 92%, transparent);
  box-shadow: 0 8px 22px rgba(48, 41, 32, 0.1);
  color: var(--docsy-text);
  cursor: pointer;
  text-align: left;
  transition:
    border-color 0.16s ease,
    transform 0.16s ease;
}

.thumbnail-button:hover {
  border-color: var(--docsy-primary);
  transform: translateY(-1px);
}

.thumbnail-label {
  display: block;
  margin-bottom: 6px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-weight: 600;
}

.thumbnail-end {
  display: grid;
  min-height: 220px;
  padding: 12px;
  border: 1px dashed var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  color: var(--docsy-text-muted);
  font-size: 12px;
  place-items: center;
  text-align: center;
}

.thumbnail-resize-handle {
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 18px;
  height: 18px;
  cursor: nwse-resize;
  opacity: 0.72;
  touch-action: none;
}

.thumbnail-resize-handle::before {
  position: absolute;
  right: 0;
  bottom: 5px;
  width: 16px;
  height: 3px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--docsy-primary) 72%, transparent);
  content: '';
  transform: rotate(45deg);
  transform-origin: right center;
}

.thumbnail-resize-handle:hover {
  opacity: 1;
}
</style>
