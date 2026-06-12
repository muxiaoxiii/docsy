<template>
  <div class="image-paddler">
    <div class="toolbar">
      <el-button type="primary" @click="pickFolder">选择图片文件夹</el-button>
      <el-button :disabled="!folder" :loading="analyzing" @click="analyze">分析图片</el-button>
      <el-button
        type="success"
        :disabled="!analysis"
        :loading="running"
        @click="generate"
      >
        生成文件
      </el-button>
      <el-button v-if="result?.outputDir" @click="openPath(result.outputDir)">打开输出目录</el-button>
    </div>

    <el-alert
      v-if="folder"
      class="folder-alert"
      type="info"
      :closable="false"
      :title="folder"
    />

    <el-empty v-if="!folder" description="选择一个包含截图的文件夹" :image-size="90" />

    <!-- 分析中动画 -->
    <div v-else-if="analyzing" class="loading-container">
      <el-icon class="loading-icon" :size="48"><Loading /></el-icon>
      <div class="loading-text">正在分析图片...</div>
    </div>

    <!-- 生成中动画 -->
    <div v-else-if="running" class="loading-container">
      <el-icon class="loading-icon" :size="48"><Loading /></el-icon>
      <div class="loading-text">正在生成文件...</div>
    </div>

    <template v-else>
      <div class="main-layout">
        <div class="left-column">
          <section class="panel">
            <div class="panel-title">参数</div>
            <el-form label-width="80px" size="small">
              <el-form-item label="输出">
                <el-segmented v-model="settings.outputFormat" :options="outputOptions" />
              </el-form-item>
              <el-form-item label="页面">
                <el-segmented v-model="settings.orientation" :options="orientationOptions" />
              </el-form-item>

              <el-divider />

              <el-form-item label="布局模式">
                <el-segmented v-model="settings.layoutMode" :options="layoutModeOptions" />
              </el-form-item>
              <el-form-item v-if="settings.layoutMode === 'count'" label="每页张数">
                <el-segmented v-model="settings.layout" :options="layoutOptions" />
              </el-form-item>
              <el-form-item v-if="settings.layoutMode === 'grid'" label="行列">
                <div class="grid-input">
                  <el-input-number v-model="settings.cols" :min="1" :max="6" size="small" />
                  <span class="grid-separator">×</span>
                  <el-input-number v-model="settings.rows" :min="1" :max="6" size="small" />
                  <span class="grid-unit">= {{ settings.cols * settings.rows }} 张/页</span>
                </div>
              </el-form-item>

              <el-divider />

              <el-form-item label="缩放">
                <el-segmented v-model="settings.scale" :options="scaleOptions" />
              </el-form-item>
              <el-form-item label="顺序">
                <el-select v-model="settings.order" style="width: 100%">
                  <el-option label="Z 形：左上开始" value="z" />
                  <el-option label="N 形：左下开始" value="n" />
                  <el-option label="反 Z：右上开始" value="z_rev" />
                  <el-option label="反 N：右下开始" value="n_rev" />
                </el-select>
              </el-form-item>
              <el-form-item label="DPI">
                <el-select v-model="settings.dpi" style="width: 100%">
                  <el-option label="原图" value="orig" />
                  <el-option label="600" value="600" />
                  <el-option label="400" value="400" />
                  <el-option label="300" value="300" />
                  <el-option label="150" value="150" />
                </el-select>
              </el-form-item>
              <el-form-item label="嵌入格式">
                <el-select v-model="settings.raster" style="width: 100%">
                  <el-option label="自动" value="auto" />
                  <el-option label="PNG" value="png" />
                  <el-option label="JPEG" value="jpeg" />
                </el-select>
              </el-form-item>
              <el-form-item label="文件名">
                <el-switch v-model="settings.showFilename" />
              </el-form-item>
              <el-form-item label="页边距">
                <el-input-number v-model="settings.marginMm" :min="0" :max="40" />
                <span class="unit">mm</span>
              </el-form-item>
              <el-form-item label="图片间距">
                <el-input-number v-model="settings.gapMm" :min="0" :max="30" />
                <span class="unit">mm</span>
              </el-form-item>
            </el-form>
          </section>

          <div class="lower-panels">
            <section class="panel analysis-panel">
              <div class="panel-title">分析</div>
              <el-descriptions v-if="analysis" :column="2" size="small" border>
                <el-descriptions-item label="图片数">{{ analysis.imageCount }}</el-descriptions-item>
                <el-descriptions-item label="可处理文件夹">{{ analysis.folderCount }}</el-descriptions-item>
                <el-descriptions-item label="中位尺寸">
                  {{ Math.round(analysis.stats.medW) }} × {{ Math.round(analysis.stats.medH) }}
                </el-descriptions-item>
                <el-descriptions-item label="推荐">
                  {{ analysis.recommended.orientation }} /
                  {{ analysis.recommended.layout }} 张 /
                  {{ analysis.recommended.dpi }} DPI
                </el-descriptions-item>
              </el-descriptions>
              <el-empty v-else description="先分析图片后显示推荐参数" :image-size="70" />
            </section>

            <section v-if="analysis?.sampleImages?.length" class="panel image-list-panel">
              <div class="panel-title">图片列表（{{ analysis.sampleImages.length }}）</div>
              <div class="image-list">
                <div v-for="img in analysis.sampleImages" :key="img.path" class="image-item">
                  <span class="image-name">{{ img.name }}</span>
                  <span class="image-size">{{ img.width }}×{{ img.height }}</span>
                </div>
              </div>
            </section>
          </div>

          <!-- 文件名分组选择 -->
          <section v-if="analysis?.groups?.length" class="panel">
            <div class="panel-title">检测到多组截图</div>
            <el-alert type="warning" :closable="false" show-icon style="margin-bottom: 12px">
              同一目录中有 {{ analysis.groups.length }} 种不同来源的截图，建议按来源分别生成文档。
            </el-alert>
            <el-radio-group v-model="groupMode" class="group-mode">
              <el-radio value="auto">按来源分组（推荐）</el-radio>
              <el-radio value="merge">全部合并</el-radio>
              <el-radio value="selected">手动选择</el-radio>
            </el-radio-group>
            <div v-if="groupMode === 'selected'" class="group-list">
              <el-checkbox-group v-model="selectedPrefixes">
                <el-checkbox
                  v-for="g in analysis.groups"
                  :key="g.prefix"
                  :value="g.prefix"
                  class="group-item"
                >
                  <span class="group-prefix">{{ g.prefix }}</span>
                  <el-tag size="small" type="info">{{ g.count }} 张</el-tag>
                </el-checkbox>
              </el-checkbox-group>
            </div>
          </section>
        </div>

        <!-- 右侧：预览框 -->
        <div v-if="analysis" class="right-column">
          <section class="panel preview-panel">
            <div class="panel-title">布局预览</div>
            <div class="preview-container">
              <div class="preview-page" :class="previewOrientation">
                <div class="preview-content" :style="previewContentStyle">
                  <div
                    v-for="(cell, idx) in previewCells"
                    :key="idx"
                    class="preview-cell"
                    :style="cell.style"
                  >
                    <div class="preview-image-area" :style="cell.areaStyle">
                      <div class="preview-image" :style="cell.imageStyle">
                        <span class="preview-label">{{ cell.label }}</span>
                      </div>
                    </div>
                    <div v-if="settings.showFilename" class="preview-filename">文件名</div>
                  </div>
                </div>
              </div>
            </div>
            <div class="preview-info">
              <div class="preview-info-item">
                <span class="preview-info-label">页面：</span>
                <span>{{ previewInfo.pageWidth }}×{{ previewInfo.pageHeight }}mm</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">页边距：</span>
                <span>{{ settings.marginMm }}mm</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">可用区域：</span>
                <span>{{ previewInfo.contentWidth }}×{{ previewInfo.contentHeight }}mm</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">布局：</span>
                <span>{{ previewInfo.cols }}×{{ previewInfo.rows }} = {{ previewInfo.cellsPerPage }} 张/页</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">每个 cell：</span>
                <span>{{ previewInfo.cellWidth }}×{{ previewInfo.cellHeight }}mm</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">图片大小：</span>
                <span>{{ previewInfo.imageWidth }}×{{ previewInfo.imageHeight }}mm</span>
              </div>
              <div class="preview-info-item">
                <span class="preview-info-label">总页数：</span>
                <span>{{ previewInfo.totalPages }} 页（{{ analysis?.imageCount || 0 }} 张图片）</span>
              </div>
              <div v-if="previewInfo.warning" class="preview-warning">
                <el-icon><WarningFilled /></el-icon>
                <span>{{ previewInfo.warning }}</span>
              </div>
            </div>
          </section>
        </div>
      </div>

      <!-- 底部：生成结果 -->
      <section v-if="result" class="panel result-panel">
        <div class="panel-title">输出</div>
        <el-table :data="result.outputs" size="small">
          <el-table-column prop="format" label="格式" width="80" />
          <el-table-column prop="imageCount" label="图片" width="80" />
          <el-table-column prop="pageCount" label="页数" width="80" />
          <el-table-column label="校验" min-width="180">
            <template #default="{ row }">
              <el-tag :type="row.valid ? 'success' : 'danger'" effect="plain">
                {{ row.validation }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="path" label="文件" min-width="260" show-overflow-tooltip />
          <el-table-column label="操作" width="90">
            <template #default="{ row }">
              <el-button link type="primary" @click="openPath(row.path)">打开</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-alert
          v-if="result.skipped?.length"
          class="skipped"
          type="warning"
          :closable="false"
          :title="result.skipped.join('；')"
        />
      </section>
    </template>
  </div>
</template>

<script setup>
import { computed, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";
import { Loading, WarningFilled } from "@element-plus/icons-vue";

const folder = ref("");
const analysis = ref(null);
const result = ref(null);
const analyzing = ref(false);
const running = ref(false);
const groupMode = ref("auto");
const selectedPrefixes = ref([]);

const settings = reactive({
  layout: 0,
  layoutMode: "auto",
  rows: 1,
  cols: 2,
  dpi: "300",
  scale: "fit",
  raster: "auto",
  orientation: "auto",
  order: "z",
  showFilename: true,
  outputFormat: "docx",
  jpegQuality: 85,
  marginMm: 10,
  gapMm: 6,
  depth: 5,
});

const outputOptions = [
  { label: "DOCX", value: "docx" },
  { label: "PDF", value: "pdf" },
  { label: "两者", value: "both" },
];
const orientationOptions = [
  { label: "自动", value: "auto" },
  { label: "横向", value: "landscape" },
  { label: "竖向", value: "portrait" },
];
const layoutModeOptions = [
  { label: "自动", value: "auto" },
  { label: "按张数", value: "count" },
  { label: "按行列", value: "grid" },
];
const layoutOptions = [
  { label: "1", value: 1 },
  { label: "2", value: 2 },
  { label: "3", value: 3 },
  { label: "4", value: 4 },
];
const scaleOptions = [
  { label: "适应页面", value: "fit" },
  { label: "填满裁切", value: "fill" },
  { label: "原始大小", value: "original" },
];

// A4 尺寸（mm）
const A4_WIDTH = 210;
const A4_HEIGHT = 297;
const CAPTION_HEIGHT_MM = 14 * 25.4 / 72;

// 预览方向
const previewOrientation = computed(() => {
  if (settings.orientation === "auto") {
    return analysis.value?.recommended?.orientation === "portrait" ? "portrait" : "landscape";
  }
  return settings.orientation;
});

// 布局计算
const layoutInfo = computed(() => {
  const isLandscape = previewOrientation.value === "landscape";
  const pageW = isLandscape ? A4_HEIGHT : A4_WIDTH;
  const pageH = isLandscape ? A4_WIDTH : A4_HEIGHT;

  let cols, rows;
  if (settings.layoutMode === "grid") {
    cols = settings.cols;
    rows = settings.rows;
  } else {
    const layout = settings.layout === 0 ? (analysis.value?.recommended?.layout || 2) : settings.layout;
    switch (layout) {
      case 1: cols = 1; rows = 1; break;
      case 2: cols = 2; rows = 1; break;
      case 3: cols = 3; rows = 1; break;
      case 4: cols = 2; rows = 2; break;
      default: cols = 2; rows = 1;
    }
  }

  const contentW = pageW - 2 * settings.marginMm;
  const contentH = pageH - 2 * settings.marginMm;
  const cellW = (contentW - settings.gapMm * (cols - 1)) / cols;
  const cellH = (contentH - settings.gapMm * (rows - 1)) / rows;

  return { pageW, pageH, cols, rows, contentW, contentH, cellW, cellH };
});

const previewContentStyle = computed(() => {
  const { pageW, pageH, contentW, contentH } = layoutInfo.value;
  return {
    left: `${settings.marginMm / pageW * 100}%`,
    top: `${settings.marginMm / pageH * 100}%`,
    width: `${contentW / pageW * 100}%`,
    height: `${contentH / pageH * 100}%`,
  };
});

// 预览单元格
const previewCells = computed(() => {
  const { cols, rows, cellW, cellH } = layoutInfo.value;
  const cells = [];

  const imgW = analysis.value ? analysis.value.stats.medW * 25.4 / 72 : 100;
  const imgH = analysis.value ? analysis.value.stats.medH * 25.4 / 72 : 75;

  // 计算图片在 cell 内的显示大小
  const filenameH = settings.showFilename ? CAPTION_HEIGHT_MM : 0;
  const availH = cellH - filenameH;

  let displayW, displayH;
  if (settings.scale === "fill") {
    displayW = cellW;
    displayH = availH;
  } else if (settings.scale === "original") {
    displayW = Math.min(imgW, cellW);
    displayH = Math.min(imgH, availH);
  } else {
    // fit
    const scale = Math.min(cellW / imgW, availH / imgH);
    displayW = imgW * scale;
    displayH = imgH * scale;
  }

  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const idx = row * cols + col;
      cells.push({
        label: `${idx + 1}`,
        style: {
          left: `${(col * (cellW + settings.gapMm)) / layoutInfo.value.contentW * 100}%`,
          top: `${(row * (cellH + settings.gapMm)) / layoutInfo.value.contentH * 100}%`,
          width: `${cellW / layoutInfo.value.contentW * 100}%`,
          height: `${cellH / layoutInfo.value.contentH * 100}%`,
        },
        areaStyle: {
          height: `${Math.max(availH, 1) / cellH * 100}%`,
        },
        imageStyle: {
          width: `${displayW / cellW * 100}%`,
          height: `${displayH / Math.max(availH, 1) * 100}%`,
        },
      });
    }
  }

  return cells;
});

// 预览信息
const previewInfo = computed(() => {
  const { pageW, pageH, cols, rows, contentW, contentH, cellW, cellH } = layoutInfo.value;
  const imgW = analysis.value ? Math.round(analysis.value.stats.medW * 25.4 / 72) : 100;
  const imgH = analysis.value ? Math.round(analysis.value.stats.medH * 25.4 / 72) : 75;

  const cellsPerPage = cols * rows;
  const imageCount = analysis.value?.imageCount || 0;
  const totalPages = Math.ceil(imageCount / cellsPerPage);

  // 检查警告
  let warning = null;
  if (settings.scale === "original" && (imgW > cellW || imgH > cellH)) {
    warning = `原始图片（${imgW}×${imgH}mm）大于 cell（${Math.round(cellW)}×${Math.round(cellH)}mm），将自动缩小`;
  }

  return {
    pageWidth: Math.round(pageW),
    pageHeight: Math.round(pageH),
    contentWidth: Math.round(contentW),
    contentHeight: Math.round(contentH),
    cols,
    rows,
    cellsPerPage,
    cellWidth: Math.round(cellW),
    cellHeight: Math.round(cellH),
    imageWidth: imgW,
    imageHeight: imgH,
    totalPages,
    warning,
  };
});

async function pickFolder() {
  const path = await openDialog({ directory: true, multiple: false });
  if (!path) return;
  folder.value = path;
  result.value = null;
  await analyze();
}

async function analyze() {
  if (!folder.value) return;
  analyzing.value = true;
  try {
    const data = await invoke("analyze_image_paddler_folder", {
      folder: folder.value,
      depth: settings.depth,
    });
    analysis.value = data;
    Object.assign(settings, data.recommended || {});
    ElMessage.success("分析完成");
  } catch (err) {
    ElMessage.error(`分析失败：${err}`);
  } finally {
    analyzing.value = false;
  }
}

async function generate() {
  if (!folder.value) return;
  running.value = true;
  try {
    const rec = analysis.value?.recommended;
    const effectiveSettings = {
      ...settings,
      layout: settings.layout === 0 && rec ? rec.layout : settings.layout,
      orientation: settings.orientation === "auto" && rec ? rec.orientation : settings.orientation,
    };
    const hasGroups = analysis.value?.groups?.length > 0;
    const effectiveGroupMode = hasGroups ? groupMode.value : "merge";
    result.value = await invoke("run_image_paddler", {
      args: {
        folder: folder.value,
        outputDir: null,
        settings: effectiveSettings,
        groupMode: effectiveGroupMode,
        selectedPrefixes: effectiveGroupMode === "selected" ? selectedPrefixes.value : [],
      },
    });
    ElMessage.success("生成完成");
  } catch (err) {
    ElMessage.error(`生成失败：${err}`);
  } finally {
    running.value = false;
  }
}

async function openPath(path) {
  try {
    await invoke("open_path", { path });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}
</script>

<style scoped>
.image-paddler {
  height: 100%;
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 16px;
  overflow: auto;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.folder-alert {
  margin-bottom: 12px;
}
.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
}
.loading-icon {
  color: #409eff;
  animation: spin 1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.loading-text {
  margin-top: 16px;
  color: #6b7280;
  font-size: 14px;
}
.main-layout {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.left-column {
  flex: 1;
  min-width: 0;
}
.right-column {
  width: 360px;
  flex-shrink: 0;
}
.panel {
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 14px;
  background: #ffffff;
  margin-bottom: 16px;
}
.panel-title {
  font-weight: 600;
  color: #1f2937;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
}
.unit {
  margin-left: 6px;
  color: #6b7280;
}
.grid-input {
  display: flex;
  align-items: center;
  gap: 8px;
}
.grid-separator {
  color: #6b7280;
}
.grid-unit {
  color: #6b7280;
  font-size: 12px;
  margin-left: 8px;
}
.image-list {
  max-height: 320px;
  overflow-y: auto;
  border: 1px solid #e5e7eb;
  border-radius: 4px;
}
.image-item {
  display: flex;
  justify-content: space-between;
  padding: 6px 10px;
  font-size: 12px;
  border-bottom: 1px solid #f3f4f6;
}
.image-item:last-child {
  border-bottom: none;
}
.image-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  margin-right: 8px;
}
.image-size {
  color: #6b7280;
  white-space: nowrap;
}

/* 分组 */
.group-mode {
  margin-bottom: 12px;
}
.group-list {
  margin-top: 8px;
  padding: 12px;
  background: #f9fafb;
  border-radius: 6px;
}
.group-item {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
  width: 100%;
}
.group-item:last-child {
  margin-bottom: 0;
}
.group-prefix {
  margin-right: 8px;
  font-size: 13px;
  color: #374151;
  word-break: break-all;
}

/* 预览框 */
.preview-panel {
  position: sticky;
  top: 16px;
}
.preview-container {
  display: flex;
  justify-content: center;
  margin-bottom: 12px;
}
.preview-page {
  position: relative;
  background: #f9fafb;
  border: 2px solid #d1d5db;
  border-radius: 4px;
  width: 100%;
}
.preview-page.portrait {
  aspect-ratio: 210 / 297;
  max-height: 520px;
}
.preview-page.landscape {
  aspect-ratio: 297 / 210;
  max-height: 360px;
}
.preview-content {
  position: absolute;
  border: 1px dotted #cbd5e1;
}
.preview-cell {
  position: absolute;
  border: 1px dashed #9ca3af;
  border-radius: 2px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  padding: 1px;
  box-sizing: border-box;
  overflow: hidden;
}
.preview-image-area {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 0;
}
.preview-image {
  background: #dbeafe;
  border: 1px solid #93c5fd;
  border-radius: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.preview-label {
  font-size: 10px;
  color: #3b82f6;
}
.preview-filename {
  font-size: 8px;
  color: #6b7280;
  text-align: center;
  margin-top: 2px;
}
.preview-info {
  font-size: 12px;
  line-height: 1.8;
}
.preview-info-item {
  display: flex;
  gap: 4px;
}
.preview-info-label {
  color: #6b7280;
  white-space: nowrap;
}
.preview-warning {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
  padding: 8px;
  background: #fef3c7;
  border-radius: 4px;
  color: #92400e;
  font-size: 12px;
}

/* 结果 */
.result-panel {
  margin-top: 16px;
}
.skipped {
  margin-top: 10px;
}

@media (max-width: 1200px) {
  .main-layout {
    flex-direction: column;
  }
  .left-column,
  .right-column {
    width: 100%;
  }
}
</style>
