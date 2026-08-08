<template>
  <div class="document-preview" @mouseup="onMouseUp" @keyup="onKeyUp">
    <template v-for="run in runs" :key="run.id">
      <span
        v-if="getOverlay(run)"
        class="preview-overlay"
        :class="[`overlay-${getOverlay(run)?.type || 'text'}`, { filled: getOverlay(run)?.filled }]"
        :data-run-id="run.id"
        :data-start="getOverlay(run)?.start"
        :data-end="getOverlay(run)?.end"
        @click="onOverlayClick(getOverlay(run))"
      >
        {{ getOverlay(run)?.label || run.text }}
      </span>
      <span
        v-else
        class="preview-run"
        :class="formatClasses(run)"
        :data-run-id="run.id"
        :data-start="0"
        :data-end="run.text.length"
      >{{ run.text }}</span>
    </template>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  runs: { type: Array, default: () => [] },
  overlays: { type: Array, default: () => [] },
  mode: { type: String, default: 'fields' }, // 'original' | 'fields' | 'fill'
})

const emit = defineEmits(['select', 'click-overlay'])

// Build overlay lookup: runId → overlay
const overlayMap = computed(() => {
  const map = new Map()
  for (const ov of props.overlays) {
    map.set(ov.runId, ov)
  }
  return map
})

function getOverlay(run) {
  if (props.mode === 'original') return null
  return overlayMap.value.get(run.id) || null
}

function formatClasses(run) {
  return {
    'fmt-bold': run.bold,
    'fmt-italic': run.italic,
    'fmt-underline': run.underline,
  }
}

function onMouseUp() {
  const sel = window.getSelection()
  if (!sel || sel.isCollapsed) return
  emit('select', {
    text: sel.toString(),
    range: sel.getRangeAt(0),
  })
}

function onKeyUp() {
  // Keyboard selection support
}

function onOverlayClick(overlay) {
  if (overlay?.clickable !== false) {
    emit('click-overlay', overlay)
  }
}
</script>

<style scoped>
.document-preview {
  font-family: 'SimSun', '宋体', serif;
  font-size: 14px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-all;
  padding: 12px;
  background: var(--docsy-surface, #fff);
  border: 1px solid var(--docsy-border-subtle, #e4e7ed);
  border-radius: 6px;
  max-height: 500px;
  overflow-y: auto;
  user-select: text;
}

.preview-run {
  /* Base run styling */
}

.preview-overlay {
  padding: 1px 4px;
  border-radius: 3px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.overlay-text {
  background: rgba(64, 158, 255, 0.15);
  color: var(--el-color-primary);
}

.overlay-date {
  background: rgba(103, 194, 58, 0.15);
  color: var(--el-color-success);
}

.overlay-list,
.overlay-party_list {
  background: rgba(230, 162, 60, 0.15);
  color: var(--el-color-warning);
}

.overlay-reference {
  background: rgba(144, 147, 153, 0.15);
  color: var(--el-color-info);
}

.overlay-checkbox {
  background: rgba(245, 108, 108, 0.15);
  color: var(--el-color-danger);
}

.overlay-filled {
  background: rgba(103, 194, 58, 0.2);
  color: var(--el-color-success);
}

.fmt-bold { font-weight: bold; }
.fmt-italic { font-style: italic; }
.fmt-underline { text-decoration: underline; }
</style>
