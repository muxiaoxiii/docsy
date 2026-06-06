<template>
  <div class="pdf-unlock">
    <el-alert
      v-if="qpdfStatus && !qpdfStatus.ok"
      :title="qpdfStatus.error || 'qpdf 不可用'"
      type="error"
      show-icon
      :closable="false"
      class="qpdf-alert"
    />
    <el-alert
      v-else-if="qpdfStatus && qpdfStatus.version"
      :title="`已检测到 qpdf ${qpdfStatus.version}`"
      type="success"
      show-icon
      :closable="false"
      class="qpdf-alert"
    />

    <div class="dropzone" @click="pickFiles">
      <div class="dropzone-icon">🔓</div>
      <div class="dropzone-text">点击选择 PDF 文件</div>
      <div class="dropzone-hint">支持单个或多个 PDF</div>
    </div>

    <el-table
      v-if="files.length"
      :data="files"
      class="files-table"
      size="small"
      border
    >
      <el-table-column prop="name" label="文件" min-width="240" show-overflow-tooltip />
      <el-table-column label="状态" width="160">
        <template #default="{ row }">
          <el-tag
            :type="statusTag(row.status)"
            size="small"
            effect="light"
          >
            {{ statusLabel(row) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="加密" width="100">
        <template #default="{ row }">
          <el-tag
            v-if="row.encrypted === true"
            type="warning"
            size="small"
            effect="plain"
          >
            🔒 已加密
          </el-tag>
          <el-tag
            v-else-if="row.encrypted === false"
            type="success"
            size="small"
            effect="plain"
          >
            🔓 未加密
          </el-tag>
          <el-tag v-else type="info" size="small" effect="plain">未知</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="140">
        <template #default="{ row, $index }">
          <el-button
            v-if="row.outputPath"
            size="small"
            link
            type="primary"
            @click="openOutput(row)"
          >
            打开
          </el-button>
          <el-button
            size="small"
            link
            type="danger"
            @click="removeRow($index)"
          >
            移除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="actions">
      <el-button
        type="primary"
        :loading="running"
        :disabled="!canRun"
        @click="runUnlock"
      >
        开始解锁
      </el-button>
      <el-button :disabled="running || !files.length" @click="clearAll">
        清空
      </el-button>
      <span v-if="resultText" class="result-text">{{ resultText }}</span>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";

const qpdfStatus = ref(null);
const files = ref([]);
const running = ref(false);
const resultText = ref("");

const canRun = computed(
  () =>
    !running.value &&
    files.value.length > 0 &&
    qpdfStatus.value?.ok === true
);

onMounted(async () => {
  try {
    qpdfStatus.value = await invoke("check_qpdf");
  } catch (err) {
    qpdfStatus.value = { ok: false, error: String(err) };
  }
});

async function pickFiles() {
  const selected = await openDialog({
    multiple: true,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!selected) return;
  const list = Array.isArray(selected) ? selected : [selected];
  for (const path of list) {
    if (files.value.some((f) => f.path === path)) continue;
    const row = {
      path,
      name: basename(path),
      status: "pending",
      encrypted: null,
      outputPath: null,
      error: null,
    };
    files.value.push(row);
    inspectRow(row);
  }
}

async function inspectRow(row) {
  try {
    const res = await invoke("inspect_pdf", { input: row.path });
    row.encrypted = res.encrypted;
    if (res.encrypted === false) {
      row.status = "skipped";
    }
  } catch (err) {
    row.encrypted = null;
  }
}

async function runUnlock() {
  if (!canRun.value) return;
  running.value = true;
  resultText.value = "";

  let success = 0;
  let skipped = 0;
  for (const f of files.value) {
    if (f.status === "success") {
      success += 1;
      continue;
    }
    if (f.encrypted === false) {
      f.status = "skipped";
      skipped += 1;
      continue;
    }
    f.status = "running";
    try {
      const res = await invoke("unlock_pdf", { input: f.path });
      f.outputPath = res.outputPath;
      f.status = "success";
      success += 1;
    } catch (err) {
      f.status = "failed";
      f.error = String(err);
    }
  }

  running.value = false;
  const total = files.value.length;
  const handled = success;
  const failed = total - success - skipped;
  if (failed === 0 && handled === total - skipped) {
    const parts = [];
    if (handled) parts.push(`解锁成功 ${handled}`);
    if (skipped) parts.push(`未加密跳过 ${skipped}`);
    resultText.value = parts.join("，") || "无需处理";
    ElMessage.success(resultText.value);
  } else if (handled === 0 && failed > 0) {
    resultText.value = `全部失败 ${failed}/${total}`;
    ElMessage.error(resultText.value);
  } else {
    resultText.value = `成功 ${handled}，失败 ${failed}，跳过 ${skipped}`;
    ElMessage.warning(resultText.value);
  }
}

function clearAll() {
  files.value = [];
  resultText.value = "";
}

function removeRow(i) {
  files.value.splice(i, 1);
}

async function openOutput(row) {
  if (!row.outputPath) return;
  try {
    await invoke("open_path", { path: row.outputPath });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}

function basename(p) {
  return p.split(/[\\/]/).pop() ?? p;
}

function statusTag(s) {
  return (
    {
      pending: "info",
      running: "warning",
      success: "success",
      failed: "danger",
      skipped: "info",
    }[s] ?? "info"
  );
}

function statusLabel(row) {
  if (row.status === "failed") return row.error?.slice(0, 20) || "失败";
  return (
    {
      pending: "等待",
      running: "处理中",
      success: "成功",
      failed: "失败",
      skipped: "跳过",
    }[row.status] ?? row.status
  );
}
</script>

<style scoped>
.pdf-unlock {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.qpdf-alert {
  margin-bottom: 4px;
}
.dropzone {
  border: 2px dashed #cbd5e1;
  border-radius: 12px;
  padding: 36px;
  text-align: center;
  cursor: pointer;
  background: #fafbfc;
  transition: all 0.15s ease;
}
.dropzone:hover {
  border-color: #4f8cff;
  background: #f0f6ff;
}
.dropzone-icon {
  font-size: 36px;
  margin-bottom: 8px;
}
.dropzone-text {
  font-size: 16px;
  font-weight: 600;
  color: #374151;
}
.dropzone-hint {
  font-size: 12px;
  color: #6b7280;
  margin-top: 4px;
}
.files-table {
  width: 100%;
}
.actions {
  display: flex;
  align-items: center;
  gap: 12px;
}
.result-text {
  color: #4b5563;
  font-size: 13px;
}
</style>
