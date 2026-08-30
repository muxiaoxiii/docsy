<template>
  <aside class="next-page-thumbnail" aria-label="下一页预览">
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
        :scale="0.35"
        compact
        @error="(message) => emit('error', message)"
      />
    </button>
    <div v-else class="thumbnail-end">已是最后一页</div>
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
let timer = null

const nextPage = computed(() => Math.max(1, Number(props.page || 1)) + 1)
const hasNextPage = computed(() => Boolean(props.filePath) && nextPage.value <= Math.max(1, Number(props.maxPage || 1)))

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
})
</script>

<style scoped>
.next-page-thumbnail {
  flex: 0 0 clamp(120px, 12vw, 150px);
  min-width: 120px;
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
  min-height: 120px;
  padding: 12px;
  border: 1px dashed var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  color: var(--docsy-text-muted);
  font-size: 12px;
  place-items: center;
  text-align: center;
}
</style>
