<template>
  <article class="document-preview" @mouseup="onMouseUp" @keyup="onKeyUp">
    <p v-for="paragraph in paragraphs" :key="paragraph.key" class="preview-paragraph">
      <span
        v-for="segment in paragraph.segments"
        :key="segment.id"
        :class="segmentClasses(segment)"
        :data-run-id="segment.runId"
        :data-start="segment.start"
        :data-end="segment.end"
        @click="onOverlayClick(segment.overlay)"
      >
        {{ segment.text }}
      </span>
    </p>
  </article>
</template>

<script setup>
import { computed } from 'vue'
import { buildPreviewParagraphs } from './documentPreviewModel.js'

const props = defineProps({
  runs: { type: Array, default: () => [] },
  overlays: { type: Array, default: () => [] },
  mode: { type: String, default: 'fields' }, // 'original' | 'fields' | 'fill'
})

const emit = defineEmits(['select', 'click-overlay'])

const paragraphs = computed(() => buildPreviewParagraphs(props.runs, props.overlays, props.mode))

function segmentClasses(segment) {
  const overlay = segment.overlay
  return [
    overlay ? 'preview-overlay' : 'preview-run',
    overlay ? `overlay-${overlay.type || 'text'}` : '',
    {
      filled: Boolean(overlay?.filled),
      'fmt-bold': segment.bold,
      'fmt-italic': segment.italic,
      'fmt-underline': segment.underline,
    },
  ]
}

function onMouseUp() {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return
  emit('select', {
    text: sel.toString(),
    range: sel.getRangeAt(0),
  })
}

function onKeyUp() {
  const sel = window.getSelection()
  if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return
  emit('select', {
    text: sel.toString(),
    range: sel.getRangeAt(0),
  })
}

function onOverlayClick(overlay) {
  if (overlay && overlay.clickable !== false) {
    emit('click-overlay', overlay)
  }
}
</script>

<style scoped>
.document-preview {
  font-family: 'SimSun', '宋体', serif;
  font-size: 14px;
  line-height: 1.8;
  word-break: break-all;
  padding: 22px 26px;
  background: var(--docsy-preview-paper, #fff);
  border: 1px solid var(--docsy-border-subtle, #e4e7ed);
  border-radius: var(--docsy-radius);
  box-shadow: 0 12px 32px rgba(48, 41, 32, 0.08);
  max-height: 500px;
  overflow-y: auto;
  user-select: text;
}

.preview-paragraph {
  min-height: 1.8em;
  margin: 0;
  white-space: pre-wrap;
}

.preview-run {
  /* Base run styling */
}

.preview-overlay {
  padding: 1px 4px;
  border-radius: var(--docsy-radius);
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

.preview-overlay.filled {
  background: rgba(103, 194, 58, 0.2);
  color: var(--el-color-success);
}

.fmt-bold {
  font-weight: bold;
}
.fmt-italic {
  font-style: italic;
}
.fmt-underline {
  text-decoration: underline;
}
</style>
