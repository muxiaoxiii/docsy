<template>
  <div class="pdf-evidence">
    <el-alert
      v-if="qpdfStatus && !qpdfStatus.ok"
      :title="qpdfStatus.error || 'qpdf 不可用，PDF 合并功能不可运行'"
      type="error"
      show-icon
      :closable="false"
    />

    <!-- 第一步：选择母文件夹 + 扫描 -->
    <section class="tool-section">
      <div class="section-head">
        <div>
          <h3>1. 选择母文件夹</h3>
          <p>{{ scanSummary }}</p>
          <el-alert
            v-for="(w, i) in scanWarnings"
            :key="i"
            :title="w"
            type="warning"
            show-icon
            :closable="false"
            style="margin-top: 6px"
          />
        </div>
        <div class="head-actions">
          <el-button @click="pickRoot">选择文件夹</el-button>
          <el-button :disabled="!scan" @click="setAll(true)">全选</el-button>
          <el-button :disabled="!scan" @click="invertSelection">反选</el-button>
          <el-button :disabled="!scan" @click="setAll(false)">不选</el-button>
        </div>
      </div>

      <el-table
        v-if="scan?.groups?.length"
        :data="scan.groups"
        size="small"
        border
        row-key="id"
        class="groups-table"
      >
        <el-table-column type="expand">
          <template #default="{ row }">
            <el-table :data="row.files" size="small" border>
              <el-table-column width="56">
                <template #default="{ row: file }">
                  <el-checkbox v-model="selected[file.path]" />
                </template>
              </el-table-column>
              <el-table-column prop="name" label="文件" min-width="240" show-overflow-tooltip />
              <el-table-column prop="extension" label="类型" width="90" />
              <el-table-column prop="path" label="路径" min-width="360" show-overflow-tooltip />
            </el-table>
          </template>
        </el-table-column>
        <el-table-column prop="name" label="子文件夹" min-width="220" />
        <el-table-column label="文件数" width="100">
          <template #default="{ row }">{{ row.files.length }}</template>
        </el-table-column>
        <el-table-column label="已选" width="100">
          <template #default="{ row }">{{ selectedCount(row.files) }}</template>
        </el-table-column>
      </el-table>

      <div class="actions">
        <el-button
          type="primary"
          :disabled="!canBuildGroups"
          :loading="buildingGroups"
          @click="buildGroupPdfs"
        >
          按子文件夹合成 PDF
        </el-button>
        <span v-if="groupResultText" class="muted">{{ groupResultText }}</span>
      </div>
    </section>

    <!-- 第二步：整理 PDF 身份 -->
    <section class="tool-section">
      <div class="section-head">
        <div>
          <h3>2. 整理 PDF 身份</h3>
          <p>包含第一步生成的子文件夹 PDF，以及母文件夹中原有 PDF。可拖拽调整顺序、增删文件和批量命名。</p>
        </div>
        <div class="head-actions">
          <el-button :disabled="!scan" @click="loadRootPdfs">带入母文件夹 PDF</el-button>
          <el-button :disabled="!scan" @click="loadStepPdfs">带入第一步结果</el-button>
          <el-button @click="addPdfFiles">手动添加 PDF</el-button>
        </div>
      </div>

      <div class="rename-bar">
        <el-select v-model="renameMode" size="small" style="width: 160px">
          <el-option label="不改名" value="none" />
          <el-option label="前缀" value="prefix" />
          <el-option label="前缀+序号" value="prefix_seq" />
          <el-option label="序号" value="seq" />
          <el-option label="序号+后缀" value="seq_suffix" />
          <el-option label="后缀" value="suffix" />
          <el-option label="日期+序号" value="date_seq" />
        </el-select>
        <el-input
          v-model="renameText"
          size="small"
          :placeholder="renamePlaceholder"
          style="width: 180px"
        />
        <el-input-number
          v-model="renameStart"
          size="small"
          :min="0"
          :max="9999"
          controls-position="right"
          style="width: 100px"
        />
        <el-button size="small" @click="applyRename">应用命名</el-button>
      </div>

      <!-- 拖拽列表 -->
      <draggable
        v-if="pdfItems.length"
        v-model="pdfItems"
        item-key="id"
        handle=".drag-handle"
        animation="200"
        class="pdf-items-list"
      >
        <template #item="{ element, index }">
          <div class="pdf-item-row">
            <div class="drag-handle" title="拖拽排序">
              <el-icon><Rank /></el-icon>
            </div>
            <div class="pdf-item-index">{{ index + 1 }}</div>
            <div class="pdf-item-header">
              <el-input
                v-model="element.header"
                size="small"
                placeholder="页眉名称"
              />
            </div>
            <div class="pdf-item-name" :title="element.name">{{ element.name }}</div>
            <div class="pdf-item-source">
              <el-tag v-if="element.source === 'group'" size="small" type="success">子文件夹</el-tag>
              <el-tag v-else-if="element.source === 'root'" size="small" type="info">母文件夹</el-tag>
              <el-tag v-else size="small">手动添加</el-tag>
            </div>
            <div class="pdf-item-actions">
              <el-button size="small" link type="danger" @click="pdfItems.splice(index, 1)">移除</el-button>
            </div>
          </div>
        </template>
      </draggable>

      <el-empty v-else-if="scan" description="暂无 PDF，请从第一步带入或手动添加" :image-size="60" />
    </section>

    <!-- 第三步：页眉页脚 -->
    <section class="tool-section">
      <div class="section-head">
        <div>
          <h3>3. 页眉页脚</h3>
          <p>为每个 PDF 添加页眉和页脚页码。页脚格式：当前页/总页数（基于合并后全局页数）。</p>
        </div>
      </div>

      <div class="header-config">
        <div class="config-row">
          <label>页眉来源：</label>
          <el-select v-model="headerMode" size="small" style="width: 200px">
            <el-option label="无页眉" value="none" />
            <el-option label="文件名" value="filename" />
            <el-option label="按表格中的名称" value="per_file" />
            <el-option label="自定义文本（全局）" value="custom" />
            <el-option label="序号（证据1、证据2…）" value="seq" />
            <el-option label="序号中文（证据一、证据二…）" value="seq_cn" />
            <el-option label="固定前缀+序号" value="prefix_seq" />
          </el-select>
          <el-input
            v-if="headerMode === 'custom' || headerMode === 'prefix_seq'"
            v-model="headerText"
            size="small"
            :placeholder="headerMode === 'custom' ? '自定义文本' : '前缀文本（如：提交人：原告）'"
            style="width: 240px"
          />
        </div>
        <div class="config-row">
          <label>页脚页码：</label>
          <el-switch v-model="footerEnabled" active-text="启用" inactive-text="关闭" />
          <span v-if="footerEnabled && totalPages > 0" class="muted">
            全局总页数：{{ totalPages }}
          </span>
        </div>
      </div>

      <div class="actions" style="margin-bottom: 10px">
        <el-button size="small" :disabled="!pdfItems.length" :loading="checkingPages" @click="checkAllPages">
          检查页面尺寸
        </el-button>
      </div>

      <!-- 页面检查警告 -->
      <template v-if="pageWarnings.length">
        <el-alert
          v-for="(w, i) in pageWarnings"
          :key="i"
          :title="w"
          type="warning"
          show-icon
          :closable="false"
          style="margin-bottom: 6px"
        />
      </template>

      <!-- 各 PDF 的页数范围预览 -->
      <el-table v-if="pdfItems.length && pageRanges.length" :data="pageRanges" size="small" border class="page-ranges-table">
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="header" label="页眉" min-width="200" />
        <el-table-column prop="name" label="文件" min-width="200" show-overflow-tooltip />
        <el-table-column label="页数" width="80">
          <template #default="{ row }">{{ row.pages }}</template>
        </el-table-column>
        <el-table-column label="全局页码范围" width="140">
          <template #default="{ row }">{{ row.range }}</template>
        </el-table-column>
      </el-table>

      <div class="actions">
        <el-button
          type="primary"
          :disabled="!pdfItems.length || overlaying"
          :loading="overlaying"
          @click="applyOverlay"
        >
          应用页眉页脚（另存副本）
        </el-button>
        <span v-if="overlayResultText" class="muted">{{ overlayResultText }}</span>
      </div>
    </section>

    <!-- 第四步：合并 PDF -->
    <section class="tool-section">
      <div class="section-head">
        <div>
          <h3>4. 合并 PDF</h3>
          <p>将所有 PDF 合并为一个文件。页眉页脚已包含全局页码。</p>
        </div>
        <el-button :disabled="!pdfItems.length" @click="chooseOutput">选择输出文件</el-button>
      </div>
      <el-input v-model="outputPath" placeholder="输出 PDF 路径" />
      <div class="actions">
        <el-button
          type="primary"
          :disabled="!canMerge"
          :loading="merging"
          @click="mergeAll"
        >
          合并全部 PDF
        </el-button>
        <el-button v-if="mergedOutput" @click="openPath(mergedOutput)">打开结果</el-button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";
import { logTrace } from "../services/appLogger.js";
import { Rank } from "@element-plus/icons-vue";
import draggable from "vuedraggable";

const qpdfStatus = ref(null);
const scan = ref(null);
const selected = ref({});
const buildingGroups = ref(false);
const groupOutputs = ref([]);
const groupResultText = ref("");
const pdfItems = ref([]);
const renameMode = ref("none");
const renameText = ref("");
const renameStart = ref(1);
const outputPath = ref("");
const merging = ref(false);
const mergedOutput = ref("");

// ── 第三步：页眉页脚 ──────────────────
const headerMode = ref("filename");
const headerText = ref("");
const footerEnabled = ref(true);
const overlaying = ref(false);
const overlayResultText = ref("");
const pageCounts = ref({}); // { path: number }
const checkingPages = ref(false);
const pageWarnings = ref([]);

const scanSummary = computed(() => {
  if (!scan.value) return "选择后会扫描子文件夹内的 PDF、DOC、DOCX。";
  return `${scan.value.groups.length} 个子文件夹，母文件夹 PDF ${scan.value.rootPdfs.length} 个`;
});
const scanWarnings = computed(() => scan.value?.warnings || []);
const canBuildGroups = computed(() => scan.value && selectedPaths().length > 0 && qpdfStatus.value?.ok);
const canMerge = computed(() => pdfItems.value.length > 0 && outputPath.value && qpdfStatus.value?.ok);

// 页数统计
const totalPages = computed(() =>
  pdfItems.value.reduce((sum, item) => sum + (pageCounts.value[item.path] || 0), 0)
);

// 各 PDF 的全局页码范围
const pageRanges = computed(() => {
  let offset = 0;
  return pdfItems.value.map((item) => {
    const pages = pageCounts.value[item.path] || 0;
    const start = offset + 1;
    const end = offset + pages;
    offset += pages;
    return {
      header: item.header || stripPdf(item.name),
      name: item.name,
      path: item.path,
      start,
      end,
      pages,
      range: pages > 0 ? `${start}-${end}` : "—",
    };
  });
});

// 中文序号
const CN_NUMS = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十",
  "十一", "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十"];

const renamePlaceholder = computed(() => {
  const map = {
    none: "不改名",
    prefix: "前缀文本",
    prefix_seq: "前缀文本",
    seq: "不需输入",
    seq_suffix: "后缀文本",
    suffix: "后缀文本",
    date_seq: "不需输入",
  };
  return map[renameMode.value] || "";
});

onMounted(async () => {
  qpdfStatus.value = await invoke("check_qpdf").catch((err) => ({ ok: false, error: String(err) }));
});

// ── 第一步 ──────────────────────────────────────────

async function pickRoot() {
  const folder = await open({ directory: true, multiple: false });
  if (!folder) return;
  const result = await invoke("scan_evidence_folder", { root: folder });
  scan.value = result;
  selected.value = {};
  for (const group of result.groups || []) {
    for (const file of group.files || []) {
      selected.value[file.path] = true;
    }
  }
  groupOutputs.value = [];
  pdfItems.value = [];
  groupResultText.value = "";
  // 探针：扫描结果
  const totalFiles = (result.groups || []).reduce((s, g) => s + (g.files?.length || 0), 0);
  logTrace("pdf.evidence.scan", "pickRoot", {
    root: folder.split("/").pop(),
    groupCount: result.groups?.length || 0,
    totalFiles,
    rootPdfs: result.rootPdfs?.length || 0,
    warnings: result.warnings || [],
  });
}

function setAll(value) {
  for (const group of scan.value?.groups || []) {
    for (const file of group.files || []) {
      selected.value[file.path] = value;
    }
  }
}

function invertSelection() {
  for (const key of Object.keys(selected.value)) {
    selected.value[key] = !selected.value[key];
  }
}

function selectedCount(files) {
  return files.filter((file) => selected.value[file.path]).length;
}

function selectedPaths() {
  return Object.entries(selected.value)
    .filter(([, checked]) => checked)
    .map(([path]) => path);
}

async function buildGroupPdfs() {
  buildingGroups.value = true;
  try {
    const result = await invoke("build_evidence_group_pdfs", {
      args: {
        root: scan.value.root,
        selectedPaths: selectedPaths(),
      },
    });
    groupOutputs.value = result.outputs || [];
    loadStepPdfs();
    const failed = result.failed?.length || 0;
    groupResultText.value = `生成 ${groupOutputs.value.length} 个 PDF${failed ? `，失败 ${failed} 个` : ""}`;
    // 探针：合成结果
    logTrace("pdf.evidence.build", "buildGroupPdfs", {
      selectedCount: selectedPaths().length,
      outputsCount: groupOutputs.value.length,
      failedCount: failed,
      failedFiles: (result.failed || []).map(f => f.path?.split("/").pop()),
    });
    if (failed) {
      ElMessage.warning(groupResultText.value);
    } else {
      ElMessage.success(groupResultText.value);
    }
  } catch (err) {
    ElMessage.error(`合成失败：${err}`);
  } finally {
    buildingGroups.value = false;
  }
}

// ── 第二步 ──────────────────────────────────────────

function loadRootPdfs() {
  appendPdfItems((scan.value?.rootPdfs || []).map((file) => ({
    id: file.id,
    name: file.name,
    path: file.path,
    source: "root",
    header: stripPdf(file.name),
  })));
}

function loadStepPdfs() {
  appendPdfItems(groupOutputs.value);
}

function appendPdfItems(items) {
  for (const item of items || []) {
    if (pdfItems.value.some((existing) => existing.path === item.path)) continue;
    pdfItems.value.push({ ...item, header: item.header || stripPdf(item.name) });
  }
  fetchPageCounts();
}

async function checkAllPages() {
  checkingPages.value = true;
  pageWarnings.value = [];
  const warnings = [];
  for (const item of pdfItems.value) {
    try {
      const result = await invoke("check_pdf_pages", { path: item.path });
      if (result.hasLandscape) {
        warnings.push(`${item.name}：包含横向页面，请检查是否需要旋转`);
      }
      if (result.hasNonA4) {
        warnings.push(`${item.name}：包含非 A4 尺寸页面`);
      }
      if (!result.pages?.length) {
        warnings.push(`${item.name}：无法读取页面尺寸，不能安全添加页眉页脚`);
      }
      // 更新页数
      pageCounts.value[item.path] = result.pages.length;
    } catch (err) {
      warnings.push(`${item.name}：页面检查失败：${err}`);
    }
  }
  pageWarnings.value = warnings;
  if (warnings.length === 0) {
    ElMessage.success("所有 PDF 页面尺寸正常");
  }
  checkingPages.value = false;
}

async function fetchPageCounts() {
  for (const item of pdfItems.value) {
    if (pageCounts.value[item.path]) continue;
    try {
      const count = await invoke("get_pdf_page_count", { path: item.path });
      pageCounts.value[item.path] = count;
    } catch {
      // 页数获取失败不影响主流程
      pageCounts.value[item.path] = 0;
    }
  }
}

async function addPdfFiles() {
  const files = await open({
    multiple: true,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!files) return;
  const paths = Array.isArray(files) ? files : [files];
  for (const path of paths) {
    const name = path.split("/").pop() || path.split("\\").pop() || "unknown.pdf";
    const id = `manual_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    if (pdfItems.value.some((item) => item.path === path)) continue;
    pdfItems.value.push({
      id,
      name,
      path,
      source: "manual",
      header: stripPdf(name),
    });
  }
  fetchPageCounts();
  ElMessage.success(`已添加 ${paths.length} 个 PDF`);
}

function applyRename() {
  const today = new Date().toISOString().slice(0, 10).replaceAll("-", "");
  let seq = renameStart.value;
  pdfItems.value = pdfItems.value.map((item) => {
    const seqStr = String(seq).padStart(2, "0");
    let header = item.header;
    switch (renameMode.value) {
      case "prefix":
        header = `${renameText.value}${stripPdf(item.name)}`;
        break;
      case "prefix_seq":
        header = `${renameText.value}${seqStr}`;
        seq++;
        break;
      case "seq":
        header = seqStr;
        seq++;
        break;
      case "seq_suffix":
        header = `${seqStr}${renameText.value}`;
        seq++;
        break;
      case "suffix":
        header = `${stripPdf(item.name)}${renameText.value}`;
        break;
      case "date_seq":
        header = `${today}-${seqStr}`;
        seq++;
        break;
      // "none": 不改名
    }
    return { ...item, header };
  });
}

// ── 第三步：页眉页脚 ────────────────────────────────

function buildHeaderText(item, index) {
  const seq = index + 1;
  switch (headerMode.value) {
    case "filename":
      return stripPdf(item.name);
    case "per_file":
      // 使用每文件独立的 header（用户在表格中编辑）
      return item.header || stripPdf(item.name);
    case "custom":
      return headerText.value || stripPdf(item.name);
    case "seq":
      return `证据${seq}`;
    case "seq_cn":
      return `证据${CN_NUMS[seq] || seq}`;
    case "prefix_seq":
      return `${headerText.value || ""}证据${seq}`;
    default:
      return "";
  }
}

async function applyOverlay() {
  overlaying.value = true;
  overlayResultText.value = "";
  try {
    await fetchPageCounts();
    await checkAllPages();
    if (pageWarnings.value.some((warning) => warning.includes("无法读取页面尺寸") || warning.includes("页面检查失败"))) {
      throw new Error("存在无法安全读取页面尺寸的 PDF，请先处理后再添加页眉页脚");
    }
    if (totalPages.value <= 0) {
      throw new Error("未能读取 PDF 页数，不能生成全局页码");
    }

    // 确定输出目录：母文件夹下的 _docsy_evidence_step2
    const root = scan.value?.root || "";
    const step2Dir = root ? `${root}/_docsy_evidence_step2` : "";

    const rangesByPath = new Map(pageRanges.value.map((range) => [range.path, range]));
    const items = pdfItems.value.map((item, index) => {
      const headerText = headerMode.value !== "none" ? buildHeaderText(item, index) : null;
      const range = rangesByPath.get(item.path);
      const pageCount = range?.pages || 0;
      let footerText = null;
      if (footerEnabled.value && pageCount > 0) {
        footerText = "{page}/{total}";
      }
      // 始终用原始路径作输入，输出到 step2 目录，避免重复叠加
      const inputPath = item.originalPath || item.path;
      const outputPath = buildOverlayOutputPath(inputPath, step2Dir);
      return {
        inputPath,
        outputPath,
        pageStart: range?.start || 1,
        totalPages: totalPages.value,
        header: headerText ? { text: headerText, fontSize: 10, marginMm: 10, align: "center", offsetXMm: 0 } : null,
        footer: footerText ? { text: footerText, fontSize: 9, marginMm: 10, align: "center", offsetXMm: 0 } : null,
      };
    });
    const result = await invoke("batch_overlay_pdf_text", { args: { items } });
    const successCount = result.results?.length || 0;
    const failedCount = result.failed?.length || 0;
    overlayResultText.value = `已处理 ${successCount} 个 PDF${failedCount ? `，失败 ${failedCount} 个` : ""}`;
    // 探针：overlay 结果
    logTrace("pdf.evidence.overlay", "applyOverlay", {
      headerMode: headerMode.value,
      footerEnabled: footerEnabled.value,
      totalPages: totalPages.value,
      itemsCount: items.length,
      successCount,
      failedCount,
      failedFiles: (result.failed || []).map(f => f.path?.split("/").pop()),
    });
    // 按 inputPath 精确回填
    if (result.results) {
      const byInput = new Map(result.results.map((r) => [r.inputPath, r]));
      for (const item of pdfItems.value) {
        const inputPath = item.originalPath || item.path;
        const overlay = byInput.get(inputPath);
        if (overlay?.outputPath && overlay.outputPath !== inputPath) {
          // 记住原始路径，防止下次 overlay 叠在已有 overlay 上
          if (!item.originalPath) item.originalPath = item.path;
          item.overlayPath = overlay.outputPath;
        }
      }
    }
    if (failedCount) {
      ElMessage.warning(overlayResultText.value);
    } else {
      ElMessage.success(overlayResultText.value);
    }
  } catch (err) {
    ElMessage.error(`页眉页脚处理失败：${err}`);
  } finally {
    overlaying.value = false;
  }
}

// ── 第四步：合并 ─────────────────────────────────────

async function chooseOutput() {
  const path = await save({
    filters: [{ name: "PDF", extensions: ["pdf"] }],
    defaultPath: "证据材料合并.pdf",
  });
  if (path) outputPath.value = path.endsWith(".pdf") ? path : `${path}.pdf`;
}

async function mergeAll() {
  merging.value = true;
  try {
    // 优先使用 overlay 后的路径
    const items = pdfItems.value.map((item) => ({
      ...item,
      path: item.overlayPath || item.path,
    }));
    const result = await invoke("merge_evidence_pdfs", {
      args: {
        items,
        outputPath: outputPath.value,
      },
    });
    mergedOutput.value = result.outputPath;
    // 探针：合并结果
    logTrace("pdf.evidence.merge", "mergeAll", {
      itemsCount: items.length,
      inputCount: result.inputCount,
      hasOverlay: items.some(i => i.overlayPath),
      output: result.outputPath?.split("/").pop(),
    });
    ElMessage.success(`已合并 ${result.inputCount} 个 PDF`);
  } catch (err) {
    ElMessage.error(`合并失败：${err}`);
  } finally {
    merging.value = false;
  }
}

async function openPath(path) {
  await invoke("open_path", { path }).catch((err) => ElMessage.error(`打开失败：${err}`));
}

function stripPdf(name) {
  return String(name || "").replace(/\.pdf$/i, "");
}

function buildOverlayOutputPath(originalPath, step2Dir) {
  const normalized = String(originalPath || "");
  const name = normalized.split("/").pop().split("\\").pop() || "output.pdf";
  const stem = name.replace(/\.pdf$/i, "");
  if (step2Dir) return `${step2Dir}/${stem}_overlay.pdf`;
  // fallback：和原文件同目录
  const dot = normalized.toLowerCase().lastIndexOf(".pdf");
  if (dot >= 0) return `${normalized.slice(0, dot)}_overlay.pdf`;
  return `${normalized}_overlay.pdf`;
}
</script>

<style scoped>
.pdf-evidence {
  display: grid;
  gap: 16px;
}
.tool-section {
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 14px;
  background: #fff;
}
.section-head {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 12px;
}
.section-head h3 {
  margin: 0 0 4px;
  font-size: 16px;
}
.section-head p,
.muted {
  margin: 0;
  color: #6b7280;
  font-size: 13px;
}
.head-actions,
.actions,
.rename-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.groups-table {
  width: 100%;
}
.rename-bar {
  margin-bottom: 10px;
}

/* ── 拖拽列表样式 ────────────────────── */
.pdf-items-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: #e5e7eb;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  overflow: hidden;
}
.pdf-item-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: #fff;
  transition: background 0.15s;
}
.pdf-item-row:hover {
  background: #f9fafb;
}
.pdf-item-row.sortable-ghost {
  background: #eff6ff;
  opacity: 0.6;
}
.pdf-item-row.sortable-chosen {
  background: #eff6ff;
}
.drag-handle {
  cursor: grab;
  color: #9ca3af;
  font-size: 16px;
  flex-shrink: 0;
}
.drag-handle:active {
  cursor: grabbing;
}
.pdf-item-index {
  width: 28px;
  text-align: center;
  font-size: 13px;
  color: #6b7280;
  flex-shrink: 0;
}
.pdf-item-header {
  flex: 2;
  min-width: 160px;
}
.pdf-item-name {
  flex: 2;
  min-width: 160px;
  font-size: 13px;
  color: #374151;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pdf-item-source {
  flex-shrink: 0;
}
.pdf-item-actions {
  flex-shrink: 0;
}

/* ── 页眉页脚配置 ──────────────────── */
.header-config {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}
.config-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.config-row label {
  font-size: 13px;
  color: #374151;
  min-width: 80px;
}
.page-ranges-table {
  width: 100%;
  margin-bottom: 12px;
}
</style>
