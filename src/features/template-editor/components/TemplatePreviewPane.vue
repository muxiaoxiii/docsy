<template>
  <div
    ref="previewEl"
    class="preview"
    :class="{ 'preview-editable': editable }"
    :contenteditable="editable ? 'true' : 'false'"
    spellcheck="false"
    @mouseup="onSelect"
    @beforeinput="onBeforeInput"
    @focus="onFocus"
    @blur="onBlur"
    v-html="html"
  />
</template>

<script setup>
import { ref } from "vue";

const props = defineProps({
  html: { type: String, default: "" },
  editable: { type: Boolean, default: false },
  plainText: { type: String, default: "" },
});

const emit = defineEmits([
  "selection-change",
  "selection-clear",
  "direct-text-edit",
  "direct-text-edits",
  "direct-edit-invalid",
]);

const previewEl = ref(null);
const fixedSnapshots = ref(new Map());
const tokenSnapshot = ref("");

function getSelectionSnapshot() {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed) return null;
  const text = sel.toString();
  if (!text.trim()) return null;
  if (!previewEl.value || !previewEl.value.contains(sel.anchorNode)) return null;

  const sRange = sel.getRangeAt(0);
  return {
    text,
    anchorRect: sRange.getBoundingClientRect(),
  };
}

function onSelect() {
  if (props.editable) return;
  const snapshot = getSelectionSnapshot();
  if (!snapshot) {
    emit("selection-clear");
    return;
  }

  emit("selection-change", snapshot);
}

function onFocus() {
  if (!props.editable) return;
  fixedSnapshots.value = collectFixedTextSnapshot();
  tokenSnapshot.value = collectTokenSnapshot();
}

function onBlur() {
  if (!props.editable) return;
  const before = fixedSnapshots.value;
  const after = collectFixedTextSnapshot();
  const beforeTokens = tokenSnapshot.value;
  const afterTokens = collectTokenSnapshot();
  fixedSnapshots.value = new Map();
  tokenSnapshot.value = "";

  if (beforeTokens !== afterTokens) {
    emit("direct-edit-invalid", "字段标签不能直接编辑");
    return;
  }

  const edits = [];
  for (const [key, prev] of before.entries()) {
    if (!after.has(key)) {
      emit("direct-edit-invalid", "正文结构变化过大，请重新编辑这一处文字");
      return;
    }
    const next = after.get(key);
    if (prev.text === next.text) continue;
    edits.push({
      start: prev.start,
      end: prev.end,
      replacement: next.text,
    });
  }

  if (!edits.length) return;
  if (edits.length === 1) {
    emit("direct-text-edit", edits[0]);
    return;
  }
  emit("direct-text-edits", edits);
}

function onBeforeInput(event) {
  if (!props.editable) return;
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount || !previewEl.value) return;
  const range = sel.getRangeAt(0);
  const tokens = previewEl.value.querySelectorAll(".docsy-field-token");
  for (const token of tokens) {
    if (range.intersectsNode(token)) {
      event.preventDefault();
      emit("direct-edit-invalid", "字段标签不能直接编辑");
      return;
    }
  }
  const node = sel.anchorNode?.nodeType === Node.TEXT_NODE
    ? sel.anchorNode.parentElement
    : sel.anchorNode;
  if (node && previewEl.value.contains(node) && !node.closest?.(".docsy-fixed-text")) {
    event.preventDefault();
  }
}

function collectFixedTextSnapshot() {
  const root = previewEl.value;
  const map = new Map();
  if (!root) return map;
  for (const el of root.querySelectorAll(".docsy-fixed-text")) {
    const start = Number(el.dataset.start || 0);
    const end = Number(el.dataset.end || start);
    map.set(`${start}-${end}`, {
      start,
      end,
      text: el.textContent || "",
    });
  }
  return map;
}

function collectTokenSnapshot() {
  const root = previewEl.value;
  if (!root) return "";
  return Array.from(root.querySelectorAll(".docsy-field-token"))
    .map((el) => `${el.dataset.key}:${el.dataset.start}-${el.dataset.end}:${el.textContent}`)
    .join("|");
}

defineExpose({ getSelectionSnapshot });
</script>

<style scoped>
.preview {
  flex: 1;
  background: #ffffff;
  font-family: "宋体", "SimSun", serif;
  font-size: 16px;
  line-height: 1.8;
  color: #1f2937;
  user-select: text;
  min-width: 0;
  outline: none;
}
.preview-editable {
  cursor: text;
  border: 1px solid #bfdbfe;
  border-radius: 6px;
  padding: 12px;
}
.preview :deep(.docsy-edit-doc) {
  white-space: pre-wrap;
  word-break: break-word;
  font-family: "宋体", "SimSun", serif;
  line-height: 1.8;
}
.preview :deep(.docsy-fixed-text) {
  white-space: pre-wrap;
}
.preview :deep(.docsy-fixed-text):focus {
  outline: 1px solid #bfdbfe;
}
.preview :deep(.docsy-marked) {
  background: #fff3cd;
  border-bottom: 2px solid #f59e0b;
  cursor: pointer;
  padding: 0 1px;
  border-radius: 2px;
}
.preview :deep(.docsy-marked):hover {
  background: #ffe69c;
}
.preview :deep(.docsy-label-preview) {
  color: #1d4ed8;
  background: #dbeafe;
  border: 1px solid #93c5fd;
  border-radius: 4px;
  padding: 0 3px;
  font-weight: 600;
}
.preview :deep(.docsy-field-token) {
  color: #1d4ed8;
  background: #dbeafe;
  border: 1px solid #93c5fd;
  border-radius: 4px;
  padding: 0 3px;
  font-weight: 600;
  cursor: default;
  user-select: all;
}
.preview :deep(.docsy-plain-label-preview) {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: "宋体", "SimSun", serif;
  line-height: 1.8;
}
</style>
