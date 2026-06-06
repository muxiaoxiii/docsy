<template>
  <div class="video-extract">
    <div class="toolbar">
      <el-button type="primary" @click="pickVideo">选择视频文件</el-button>
      <el-button
        type="success"
        :disabled="!videoInfo"
        :loading="extracting"
        @click="startExtract"
      >
        开始抽帧
      </el-button>
    </div>

    <!-- FFmpeg 状态 -->
    <el-alert
      v-if="!ffmpegStatus?.available"
      class="ffmpeg-alert"
      type="warning"
      :closable="false"
      show-icon
    >
      <template #title>
        <div class="ffmpeg-install-hint">
          <span>未检测到 FFmpeg。请先安装完整版 FFmpeg（需支持 drawtext）。</span>
          <div class="ffmpeg-install-actions">
            <el-button size="small" type="primary" :loading="brewInstalling" @click="installFfmpegViaBrew">
              通过 Homebrew 安装
            </el-button>
            <el-button size="small" @click="openFfmpegDownload">
              手动下载
            </el-button>
          </div>
        </div>
      </template>
    </el-alert>
    <el-alert
      v-else-if="!ffmpegStatus?.hasDrawtext"
      class="ffmpeg-alert"
      type="error"
      :closable="false"
      show-icon
    >
      <template #title>
        <div class="ffmpeg-install-hint">
          <span>当前 FFmpeg 不支持 drawtext 滤镜，无法叠加时间轴。请安装完整版。</span>
          <div class="ffmpeg-install-actions">
            <el-button size="small" type="primary" :loading="brewInstalling" @click="reinstallFfmpeg">
              重新安装完整版
            </el-button>
          </div>
        </div>
      </template>
    </el-alert>

    <el-empty v-if="!videoInfo" description="选择一个视频文件开始" :image-size="90" />

    <template v-else>
      <!-- 视频信息 -->
      <section class="panel info-panel">
        <div class="panel-title">视频信息</div>
        <el-descriptions :column="2" size="small" border>
          <el-descriptions-item label="文件">{{ videoFileName }}</el-descriptions-item>
          <el-descriptions-item label="时长">{{ formatDuration(videoInfo.duration) }}</el-descriptions-item>
          <el-descriptions-item label="分辨率">{{ videoInfo.width }} × {{ videoInfo.height }}</el-descriptions-item>
          <el-descriptions-item label="帧率">{{ videoInfo.fps.toFixed(2) }} fps</el-descriptions-item>
          <el-descriptions-item label="编码">{{ videoInfo.codec }}</el-descriptions-item>
          <el-descriptions-item label="大小">{{ formatSize(videoInfo.sizeBytes) }}</el-descriptions-item>
        </el-descriptions>
      </section>

      <div class="settings-grid">
        <!-- 抽帧设置 + 文件名 + 输出目录 -->
        <section class="panel">
          <div class="panel-title">抽帧设置</div>
          <el-form label-width="100px" size="small">
            <el-form-item label="抽帧模式">
              <el-segmented v-model="settings.fpsMode" :options="fpsModeOptions" />
            </el-form-item>
            <el-form-item :label="settings.fpsMode === 'interval' ? '间隔秒数' : '每秒张数'">
              <el-input-number
                v-model="settings.fpsValue"
                :min="0.1"
                :max="settings.fpsMode === 'per_second' ? 60 : 3600"
                :step="settings.fpsMode === 'per_second' ? 1 : 0.5"
                :precision="1"
              />
              <span class="unit">{{ settings.fpsMode === 'interval' ? '秒/张' : '张/秒' }}</span>
            </el-form-item>
            <el-form-item label="输出格式">
              <el-segmented v-model="settings.format" :options="formatOptions" />
            </el-form-item>
            <el-form-item v-if="settings.format === 'jpg'" label="JPEG 质量">
              <el-slider v-model="settings.quality" :min="1" :max="100" :step="1" show-input />
            </el-form-item>

            <el-divider />

            <el-form-item label="文件名前缀">
              <el-input v-model="settings.filenamePrefix" placeholder="默认使用源文件名" />
            </el-form-item>
            <el-form-item label="输出目录">
              <div class="output-dir-row">
                <el-input v-model="outputDir" placeholder="默认：视频同目录/frames" readonly />
                <el-button @click="pickOutputDir">选择</el-button>
              </div>
            </el-form-item>
            <el-form-item>
              <el-button v-if="result?.outputDir" type="primary" link @click="openPath(result.outputDir)">
                打开输出目录
              </el-button>
            </el-form-item>
          </el-form>
        </section>

        <!-- 时间轴设置 -->
        <section class="panel">
          <div class="panel-title">时间轴叠加</div>
          <el-form label-width="100px" size="small">
            <el-form-item label="启用">
              <el-switch v-model="settings.timestamp.enabled" />
            </el-form-item>
            <template v-if="settings.timestamp.enabled">
              <el-form-item label="位置 X">
                <el-slider v-model="settings.timestamp.xPercent" :min="0" :max="50" :step="0.5" show-input />
                <span class="unit">%</span>
              </el-form-item>
              <el-form-item label="位置 Y">
                <el-slider v-model="settings.timestamp.yPercent" :min="0" :max="50" :step="0.5" show-input />
                <span class="unit">%</span>
              </el-form-item>
              <el-form-item label="字号">
                <el-input-number v-model="settings.timestamp.fontSize" :min="12" :max="120" :step="2" />
              </el-form-item>
              <el-form-item label="字体颜色">
                <el-color-picker v-model="settings.timestamp.fontColor" />
              </el-form-item>
              <el-form-item label="字体">
                <el-select v-model="settings.timestamp.fontFile" clearable placeholder="默认字体" style="width: 100%">
                  <el-option
                    v-for="font in systemFonts"
                    :key="font.path"
                    :label="font.name"
                    :value="font.path"
                  />
                </el-select>
              </el-form-item>
              <el-form-item label="背景颜色">
                <el-color-picker v-model="settings.timestamp.bgColor" show-alpha />
              </el-form-item>
              <el-form-item label="背景边距">
                <el-input-number v-model="settings.timestamp.bgPadding" :min="0" :max="50" :step="2" />
              </el-form-item>
            </template>
          </el-form>
        </section>
      </div>

      <!-- 预览命令 -->
      <section class="panel preview-panel">
        <div class="panel-title">
          预估结果
          <el-tag size="small" type="info" style="margin-left: 8px">
            约 {{ estimatedFrames }} 张图片
          </el-tag>
        </div>
        <div class="command-preview">
          <code>{{ previewCommand }}</code>
        </div>
      </section>

      <!-- 抽帧结果 -->
      <section v-if="result" class="panel result-panel">
        <div class="panel-title">抽帧完成</div>
        <el-descriptions :column="2" size="small" border>
          <el-descriptions-item label="输出目录">{{ result.outputDir }}</el-descriptions-item>
          <el-descriptions-item label="提取帧数">{{ result.extractedFrames }}</el-descriptions-item>
        </el-descriptions>
      </section>
    </template>
  </div>
</template>

<script setup>
import { computed, onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";

const ffmpegStatus = ref(null);
const systemFonts = ref([]);
const videoInfo = ref(null);
const videoPath = ref("");
const outputDir = ref("");
const extracting = ref(false);
const result = ref(null);
const brewInstalling = ref(false);

const settings = reactive({
  fpsMode: "interval",
  fpsValue: 3,
  format: "jpg",
  quality: 85,
  filenamePrefix: "",
  timestamp: {
    enabled: true,
    xPercent: 3,
    yPercent: 3,
    fontSize: 32,
    fontColor: "#ff0000",
    fontFile: "",
    bgColor: "#000000",
    bgOpacity: 60,
    bgPadding: 8,
  },
});

const fpsModeOptions = [
  { label: "N秒1张", value: "interval" },
  { label: "每秒N张", value: "per_second" },
];

const formatOptions = [
  { label: "JPG", value: "jpg" },
  { label: "PNG", value: "png" },
];

const videoFileName = computed(() => {
  if (!videoPath.value) return "";
  return videoPath.value.split(/[\\/]/).pop();
});

// 预估帧数
const estimatedFrames = computed(() => {
  if (!videoInfo.value) return 0;
  const duration = videoInfo.value.duration;
  if (settings.fpsMode === "per_second") {
    return Math.floor(duration * settings.fpsValue);
  } else {
    return Math.floor(duration / settings.fpsValue);
  }
});

// 预览命令
const previewCommand = computed(() => {
  if (!videoPath.value) return "";

  let fpsParam;
  if (settings.fpsMode === "per_second") {
    fpsParam = settings.fpsValue.toString();
  } else {
    fpsParam = `1/${settings.fpsValue}`;
  }

  let filter = `fps=${fpsParam}`;

  if (settings.timestamp.enabled) {
    const x = `w*${(settings.timestamp.xPercent / 100).toFixed(2)}`;
    const y = `h*${(settings.timestamp.yPercent / 100).toFixed(2)}`;
    const bgAlpha = (settings.timestamp.bgOpacity / 100).toFixed(1);

    let drawtext = `drawtext=text='%{pts\\:gmtime\\:0\\:%T}'`;
    if (settings.timestamp.fontFile) {
      drawtext += `:fontfile='${settings.timestamp.fontFile}'`;
    }
    drawtext += `:x=${x}:y=${y}`;
    drawtext += `:fontsize=${settings.timestamp.fontSize}`;
    drawtext += `:fontcolor=${settings.timestamp.fontColor}`;
    drawtext += `:box=1:boxcolor=${settings.timestamp.bgColor}@${bgAlpha}`;
    drawtext += `:boxborderw=${settings.timestamp.bgPadding}`;

    filter += `,${drawtext}`;
  }

  const ext = settings.format === "png" ? "png" : "jpg";
  const qParam = settings.format === "jpg" ? `-q:v ${Math.round(31 - (settings.quality * 31 / 100))}` : "";

  // 使用源文件名作为前缀
  const sourceName = settings.filenamePrefix || videoPath.value.split(/[\\/]/).pop().replace(/\.[^.]+$/, "");

  return `ffmpeg -i "${videoPath.value}" -vf "${filter}" ${qParam} output/${sourceName}_%05d.${ext}`.trim();
});

onMounted(async () => {
  try {
    ffmpegStatus.value = await invoke("check_ffmpeg");
  } catch {
    ffmpegStatus.value = { available: false, error: "检测失败" };
  }

  if (ffmpegStatus.value?.available) {
    try {
      systemFonts.value = await invoke("list_system_fonts");
    } catch {
      systemFonts.value = [];
    }
  }
});

async function pickVideo() {
  const path = await openDialog({
    multiple: false,
    filters: [
      { name: "视频文件", extensions: ["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v"] },
    ],
  });
  if (!path) return;

  videoPath.value = path;
  result.value = null;

  try {
    videoInfo.value = await invoke("probe_video", { path });
    ElMessage.success("视频信息读取成功");
  } catch (err) {
    ElMessage.error(`读取视频信息失败：${err}`);
    videoInfo.value = null;
  }
}

async function pickOutputDir() {
  const path = await openDialog({ directory: true, multiple: false });
  if (!path) return;
  outputDir.value = path;
}

async function startExtract() {
  if (!videoPath.value) return;

  extracting.value = true;
  try {
    // 构建背景颜色字符串
    const bgAlpha = (settings.timestamp.bgOpacity / 100).toFixed(1);
    const bgColorStr = `${settings.timestamp.bgColor}@${bgAlpha}`;

    result.value = await invoke("extract_frames", {
      args: {
        input: videoPath.value,
        outputDir: outputDir.value || null,
        fpsMode: settings.fpsMode,
        fpsValue: settings.fpsValue,
        format: settings.format,
        quality: settings.quality,
        filenamePrefix: settings.filenamePrefix || null,
        timestamp: {
          enabled: settings.timestamp.enabled,
          xPercent: settings.timestamp.xPercent,
          yPercent: settings.timestamp.yPercent,
          fontSize: settings.timestamp.fontSize,
          fontColor: settings.timestamp.fontColor,
          fontFile: settings.timestamp.fontFile,
          bgColor: bgColorStr,
          bgPadding: settings.timestamp.bgPadding,
        },
      },
    });
    ElMessage.success(`抽帧完成，共提取 ${result.value.extractedFrames} 帧`);
  } catch (err) {
    ElMessage.error(`抽帧失败：${err}`);
  } finally {
    extracting.value = false;
  }
}

async function openPath(path) {
  try {
    await invoke("open_path", { path });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}

async function installFfmpegViaBrew() {
  brewInstalling.value = true;
  try {
    const msg = await invoke("try_brew_install_ffmpeg");
    ElMessage.success(msg);
    ffmpegStatus.value = await invoke("check_ffmpeg");
    if (ffmpegStatus.value?.available) {
      systemFonts.value = await invoke("list_system_fonts");
    }
  } catch (err) {
    ElMessage.error(`安装失败：${err}`);
  } finally {
    brewInstalling.value = false;
  }
}

async function reinstallFfmpeg() {
  brewInstalling.value = true;
  try {
    await invoke("try_brew_install_ffmpeg");
    ElMessage.success("ffmpeg 已重新安装");
    ffmpegStatus.value = await invoke("check_ffmpeg");
  } catch (err) {
    ElMessage.error(`重装失败：${err}`);
  } finally {
    brewInstalling.value = false;
  }
}

function openFfmpegDownload() {
  invoke("open_path", { path: "https://ffmpeg.org/download.html" });
}

function formatDuration(seconds) {
  if (!seconds) return "00:00";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

function formatSize(bytes) {
  if (!bytes) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}
</script>

<style scoped>
.video-extract {
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
.ffmpeg-alert {
  margin-bottom: 12px;
}
.ffmpeg-install-hint {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.ffmpeg-install-actions {
  display: flex;
  gap: 8px;
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
.info-panel {
  margin-bottom: 16px;
}
.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 16px;
}
.unit {
  margin-left: 6px;
  color: #6b7280;
  font-size: 13px;
}
.output-dir-row {
  display: flex;
  gap: 8px;
  width: 100%;
}
.output-dir-row .el-input {
  flex: 1;
}
.preview-panel {
  margin-bottom: 16px;
}
.command-preview {
  background: #f3f4f6;
  border-radius: 4px;
  padding: 12px;
  overflow-x: auto;
}
.command-preview code {
  font-family: "SF Mono", "Monaco", "Menlo", monospace;
  font-size: 12px;
  color: #374151;
  word-break: break-all;
  white-space: pre-wrap;
}
.result-panel {
  margin-bottom: 0;
}
@media (max-width: 900px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
  .toolbar {
    flex-wrap: wrap;
  }
}
</style>
