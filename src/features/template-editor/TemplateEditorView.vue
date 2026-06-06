<template>
  <div class="template-builder">
    <div v-if="!docxLoaded" class="empty">
      <div class="empty-content">
        <h3>模板制作</h3>
        <p>从一份现有的 Word 文档制作 Docsy 模板。</p>
        <el-button type="primary" @click="pickFile">选择 Word 文件</el-button>
      </div>
    </div>

    <template v-else>
      <TemplateEditorToolbar
        :filename="filename"
        :manifest-name="manifest.name"
        :preview-mode="previewMode"
        :mark-count="marks.length"
        @update:manifest-name="updateManifestName"
        @switch-preview-mode="onSwitchPreviewMode"
        @insert-field="insertField"
        @edit-fixed-text="editSelectedText"
        @reset="reload"
        @save="save"
      />

      <div class="builder-body">
        <TemplatePreviewPane
          ref="previewPaneRef"
          :html="previewHtml"
          :editable="previewMode === 'edit'"
          :plain-text="plainText"
          @selection-change="onSelectionChange"
          @selection-clear="onSelectionClear"
          @direct-text-edit="onDirectTextEdit"
          @direct-text-edits="onDirectTextEdits"
          @direct-edit-invalid="onDirectEditInvalid"
        />

        <MarkAside
          :marks="marks"
          :editing-index="editingMarkIndex"
          @edit-mark="editMark"
          @remove-mark="removeMark"
        />

        <MarkPopover
          :visible="popoverVisible"
          :anchor="popoverAnchor"
          :pending="pending"
          :is-editing="editingMarkIndex >= 0"
          @confirm-mark="onConfirmMark"
          @cancel-mark="cancelPending"
          @reuse-field="onReuseField"
          @choose-location="onChooseLocation"
          @update:pending="pending = $event"
        />
      </div>
    </template>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useTemplateEditorSession } from "./composables/useTemplateEditorSession.js";
import {
  computePreviewHtml,
  computeLabelHtml,
  computeEditableTextHtml,
} from "./composables/useTemplatePreview.js";
import {
  buildPendingMark,
  confirmPendingMark,
  applyLocationChoice as applyLocationToMark,
  removeMark as removeMarkFromList,
} from "./composables/useTemplateMarks.js";
import {
  resolveEditableTextRange,
  assertRangeIsFixedText,
  shiftMarksAfterTextEdit,
  applyTextReplacement,
} from "./composables/useTemplateTextEdit.js";
import { codePointSlice } from "./utils/textRange.js";
import { saveTemplateSession } from "./composables/useTemplateSave.js";
import TemplateEditorToolbar from "./components/TemplateEditorToolbar.vue";
import TemplatePreviewPane from "./components/TemplatePreviewPane.vue";
import MarkAside from "./components/MarkAside.vue";
import MarkPopover from "./components/MarkPopover.vue";

const emit = defineEmits(["templates-changed"]);

const props = defineProps({
  editTemplateId: { type: String, default: null },
});

// Session 管理
const {
  session,
  loadFromFile,
  loadFromTemplateId,
  resetSession,
  updateSession,
  markDirty,
  clearDirty,
} = useTemplateEditorSession();

// 从 session 派生的计算属性
const docxLoaded = computed(() => session.value.status === "ready");
const filename = computed(() => session.value.sourceFilename);
const docxBase64 = computed(() => session.value.sourceDocxBase64);
const previewHtml = computed(() => session.value.preview.html);
const originalHtml = computed(() => session.value.preview.originalHtml);
const plainText = computed(() => session.value.plainText);
const marks = computed({
  get: () => session.value.marks,
  set: (val) => updateSession({ marks: val }),
});
const manifest = computed({
  get: () => session.value.manifest,
  set: (val) => updateSession({ manifest: val }),
});

// UI 状态（不属于 session）
const previewMode = ref("marked");
const pending = ref(null);
const popoverVisible = ref(false);
const popoverAnchor = ref(null);
const editingMarkIndex = ref(-1);
const previewPaneRef = ref(null);
const selectedSnapshot = ref(null);

// 初始化
onMounted(async () => {
  if (props.editTemplateId) {
    const ok = await loadFromTemplateId(props.editTemplateId);
    if (ok) {
      highlightMarks();
    }
  }
});

// 选择文件（create 模式）
async function pickFile() {
  const ok = await loadFromFile();
  if (ok) {
    selectedSnapshot.value = null;
    pending.value = null;
    popoverVisible.value = false;
    previewMode.value = "marked";
  }
}

// 预览选区变化
function onSelectionChange(selection) {
  selectedSnapshot.value = selection;
  const result = buildPendingMark(selection.text, plainText.value, marks.value);
  if (!result) {
    ElMessage.warning("无法在原文中定位选中文本，请重新选择");
    return;
  }
  popoverAnchor.value = {
    getBoundingClientRect: () => selection.anchorRect,
  };
  pending.value = result;
  popoverVisible.value = true;
}

function onSelectionClear() {
  popoverVisible.value = false;
  if (!window.getSelection()?.toString()?.trim()) {
    selectedSnapshot.value = null;
  }
}

// 预览模式切换
function onSwitchPreviewMode(mode) {
  previewMode.value = mode;
  highlightMarks();
}

function updateManifestName(name) {
  updateSession({
    manifest: {
      ...session.value.manifest,
      name,
    },
  });
  markDirty();
}

// MarkPopover 事件
function onConfirmMark(pendingMark) {
  pending.value = pendingMark;
  confirmPending();
}

function onReuseField(key) {
  if (!pending.value || !key) return;
  const src = marks.value.find((m) => m.key === key);
  if (!src) return;
  pending.value = { ...pending.value, ...src };
}

function onChooseLocation(key) {
  if (!pending.value || !key) return;
  applyLocationToMark(pending.value, key, plainText.value, marks.value);
}

function cancelPending() {
  pending.value = null;
  popoverVisible.value = false;
  editingMarkIndex.value = -1;
  window.getSelection()?.removeAllRanges();
}

function editMark(i) {
  const m = marks.value[i];
  if (!m) return;
  editingMarkIndex.value = i;
  pending.value = { ...m };
  const el = document.querySelector(`.mark-item:nth-child(${i + 2})`);
  if (el) {
    popoverAnchor.value = {
      getBoundingClientRect: () => el.getBoundingClientRect(),
    };
  }
  popoverVisible.value = true;
}

function confirmPending() {
  if (!pending.value) return;
  const result = confirmPendingMark(pending.value, marks.value, editingMarkIndex.value);
  if (!result) {
    ElMessage.warning("请填写标签");
    return;
  }
  marks.value = result;
  pending.value = null;
  popoverVisible.value = false;
  editingMarkIndex.value = -1;
  markDirty();
  highlightMarks();
}

function insertField() {
  if (previewMode.value === "edit") {
    ElMessage.warning("正文编辑模式下不能新增字段，请切回原文标记");
    return;
  }
  if (!pending.value) {
    const snapshot = previewPaneRef.value?.getSelectionSnapshot?.() || selectedSnapshot.value;
    if (snapshot) {
      onSelectionChange(snapshot);
    }
  }
  if (!pending.value) {
    ElMessage.warning("请先在预览中选中要标记为字段的文本");
    return;
  }
  // pending 已经由 onSelectionChange 设置，直接进入标记流程
}

async function onDirectTextEdit(change) {
  if (!change || (change.start === change.end && !change.replacement)) return;
  const range = { start: change.start, end: change.end };
  if (!assertRangeIsFixedText(range, marks.value)) {
    ElMessage.warning("字段标签不能直接编辑，请先取消标记");
    highlightMarks();
    return;
  }

  try {
    const result = await applyTextReplacement(
      docxBase64.value,
      range,
      change.replacement
    );
    const shiftedMarks = shiftMarksAfterTextEdit(marks.value, range, result.delta);

    updateSession({
      sourceDocxBase64: result.sourceDocxBase64,
      plainText: result.plainText,
      marks: shiftedMarks,
      preview: {
        ...session.value.preview,
        html: result.previewHtml,
        originalHtml: result.originalHtml,
      },
    });
    selectedSnapshot.value = null;
    pending.value = null;
    popoverVisible.value = false;
    markDirty();
    highlightMarks();
  } catch (err) {
    ElMessage.error(`修改失败：${err}`);
    highlightMarks();
  }
}

async function onDirectTextEdits(edits) {
  const sorted = [...(edits || [])].sort((a, b) => b.start - a.start);
  if (!sorted.length) return;
  for (const edit of sorted) {
    // eslint-disable-next-line no-await-in-loop
    await onDirectTextEdit(edit);
  }
}

function onDirectEditInvalid(message) {
  ElMessage.warning(message || "这次编辑无法应用，请重新编辑固有字");
  highlightMarks();
}

async function editSelectedText() {
  const snapshot = previewPaneRef.value?.getSelectionSnapshot?.() || selectedSnapshot.value;
  if (!snapshot?.text) {
    ElMessage.warning("请先选中要修改的固有文字");
    return;
  }
  const text = snapshot.text;

  // 1. 解析可编辑范围
  const choices = resolveEditableTextRange(plainText.value, text, marks.value);
  if (!choices.length) {
    ElMessage.warning("选区可能包含字段字或无法定位，请只选择未标记的固有字");
    return;
  }

  // 2. 多位置候选时让用户选择
  let choice = choices[0];
  if (choices.length > 1) {
    try {
      const { value } = await ElMessageBox.prompt(
        choices.map((c) => c.label).join("\n"),
        "选择要编辑的位置",
        {
          inputValue: "1",
          inputPattern: new RegExp(`^[1-${choices.length}]$`),
          inputErrorMessage: `请输入 1-${choices.length} 之间的序号`,
          confirmButtonText: "确定",
          cancelButtonText: "取消",
        }
      );
      choice = choices[Number(value) - 1] || choice;
    } catch {
      return;
    }
  }

  // 3. 检查是否是固有字（不与 mark 重叠）
  if (!assertRangeIsFixedText(choice, marks.value)) {
    ElMessage.warning("不能直接编辑字段字，请先取消标记");
    return;
  }

  // 4. 输入替换文字
  let replacement;
  try {
    const res = await ElMessageBox.prompt("输入新的固有文字", "编辑固有字", {
      inputValue: codePointSlice(plainText.value, choice.start, choice.end),
      confirmButtonText: "替换",
      cancelButtonText: "取消",
    });
    replacement = res.value ?? "";
  } catch {
    return;
  }

  // 5. 执行替换
  try {
    const result = await applyTextReplacement(docxBase64.value, choice, replacement);
    const shiftedMarks = shiftMarksAfterTextEdit(marks.value, choice, result.delta);

    updateSession({
      sourceDocxBase64: result.sourceDocxBase64,
      plainText: result.plainText,
      marks: shiftedMarks,
      preview: {
        ...session.value.preview,
        html: result.previewHtml,
        originalHtml: result.originalHtml,
      },
    });
    selectedSnapshot.value = null;
    pending.value = null;
    popoverVisible.value = false;
    markDirty();
    highlightMarks();
    ElMessage.success("已修改固有字");
  } catch (err) {
    ElMessage.error(`修改失败：${err}`);
  }
}

function highlightMarks() {
  const html = previewMode.value === "labels"
    ? computeLabelHtml(plainText.value, marks.value)
    : previewMode.value === "edit"
      ? computeEditableTextHtml(plainText.value, marks.value)
      : computePreviewHtml(session.value);
  updateSession({
    preview: { ...session.value.preview, html },
  });
  if (previewMode.value !== "edit") {
    window.getSelection()?.removeAllRanges();
  }
}

async function removeMark(i) {
  const m = marks.value[i];
  if (!m) return;
  const markText = m.text || codePointSlice(plainText.value, m.start, m.end);
  let nextMarks = removeMarkFromList(i, marks.value);

  if (isPlaceholderText(markText)) {
    let replacement;
    try {
      const res = await ElMessageBox.prompt(
        "旧模板没有保存这个字段原来的文字。请输入取消标记后要保留在模板里的固有字；留空表示删除这段文字。",
        "取消标记",
        {
          inputValue: suggestedRestoreText(m),
          confirmButtonText: "确定",
          cancelButtonText: "取消",
        }
      );
      replacement = res.value ?? "";
    } catch {
      return;
    }

    try {
      const result = await applyTextReplacement(
        docxBase64.value,
        { start: m.start, end: m.end },
        replacement
      );
      nextMarks = shiftMarksAfterTextEdit(nextMarks, m, result.delta);
      updateSession({
        sourceDocxBase64: result.sourceDocxBase64,
        plainText: result.plainText,
        marks: nextMarks,
        preview: {
          ...session.value.preview,
          html: result.previewHtml,
          originalHtml: result.originalHtml,
        },
      });
    } catch (err) {
      ElMessage.error(`取消标记失败：${err}`);
      return;
    }
  } else {
    updateSession({ marks: nextMarks });
  }

  selectedSnapshot.value = null;
  pending.value = null;
  popoverVisible.value = false;
  editingMarkIndex.value = -1;
  markDirty();
  highlightMarks();
}

function isPlaceholderText(text) {
  return /^\{\{[#*]?\w+\}\}$/.test(String(text || "").trim());
}

function suggestedRestoreText(mark) {
  if (mark.label && mark.label !== mark.key) return mark.label;
  return "";
}

function reload() {
  selectedSnapshot.value = null;
  pending.value = null;
  popoverVisible.value = false;
  editingMarkIndex.value = -1;
  resetSession();
}

async function save() {
  if (!manifest.value.name) {
    ElMessage.warning("请填写模板名称");
    return;
  }
  try {
    const { result, marks: savedMarks, dictionaries, manifest: savedManifest } =
      await saveTemplateSession(session.value);
    updateSession({ marks: savedMarks, dictionaries, manifest: savedManifest });
    clearDirty();
    highlightMarks();
    ElMessage.success(`已保存：${result.name}`);
    emit("templates-changed");
  } catch (err) {
    ElMessage.error(`保存失败：${err}`);
  }
}
</script>

<style scoped>
.template-builder {
  height: 100%;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  display: flex;
  flex-direction: column;
}
.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.empty-content {
  text-align: center;
}
.empty-content h3 {
  margin: 0 0 8px;
}
.empty-content p {
  color: #6b7280;
  margin: 0 0 16px;
}
.builder-body {
  flex: 1;
  overflow: auto;
  padding: 24px;
  position: relative;
  display: flex;
  gap: 16px;
}
</style>
