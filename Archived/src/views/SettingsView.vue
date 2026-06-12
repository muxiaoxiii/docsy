<template>
  <div class="settings-view">
    <h3>设置</h3>

    <el-card shadow="never" class="settings-card">
      <template #header>一级菜单显示</template>
      <p class="muted">选择左侧菜单里要显示的项。取消勾选不会删除数据。</p>
      <div class="menu-grid">
        <el-checkbox
          v-for="item in MENU_ITEMS"
          :key="item.key"
          :model-value="isVisible(item.key)"
          @change="(v) => setVisibility(item.key, v)"
        >
          {{ item.label }}
        </el-checkbox>
      </div>
    </el-card>

    <el-card shadow="never" class="settings-card">
      <template #header>数据目录</template>
      <p class="muted">所有用户数据存放位置：</p>
      <code class="path">{{ diagnostic.appDataDir || "正在读取..." }}</code>
    </el-card>

    <el-card shadow="never" class="settings-card">
      <template #header>问题定位</template>
      <p class="muted">当前日志文件：</p>
      <code class="path">{{ diagnostic.currentLogFile || logPath || "正在读取..." }}</code>
      <p class="muted mt">日志目录：</p>
      <code class="path">{{ diagnostic.logDir || "正在读取..." }}</code>
      <div class="diagnostic-grid">
        <span>系统：{{ diagnostic.os || "-" }}</span>
        <span>架构：{{ diagnostic.arch || "-" }}</span>
        <span>调试构建：{{ diagnostic.debug ? "是" : "否" }}</span>
      </div>
      <div class="actions">
        <el-button type="primary" plain @click="openLog">打开日志文件</el-button>
        <el-button plain @click="openLogDir">打开日志目录</el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup>
import { onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";
import { logWarn } from "../services/appLogger.js";

const MENU_ITEMS = [
  { key: "imagePaddler", label: "图片排版" },
  { key: "videoExtract", label: "视频抽帧" },
  { key: "pdf", label: "PDF 工具" },
  { key: "settings", label: "设置（本页）" },
];

const settings = reactive({
  history_max: 50,
  menu_visibility: {},
});
const logPath = ref("");
const diagnostic = reactive({
  appDataDir: "",
  logDir: "",
  currentLogFile: "",
  recentLogFiles: [],
  os: "",
  arch: "",
  debug: false,
});

const emit = defineEmits(["settings-changed"]);

onMounted(async () => {
  try {
    const s = await invoke("get_app_settings");
    Object.assign(settings, s);
    settings.menu_visibility = s.menu_visibility || {};
  } catch (err) {
    logWarn("settings", "load_settings.failed", { error: err });
  }
  try {
    logPath.value = await invoke("get_log_file_path");
  } catch (err) {
    logWarn("settings", "load_log_path.failed", { error: err });
  }
  try {
    Object.assign(diagnostic, await invoke("get_diagnostic_info"));
  } catch (err) {
    logWarn("settings", "load_diagnostic_info.failed", { error: err });
  }
});

function isVisible(key) {
  return settings.menu_visibility[key] !== false;
}

async function setVisibility(key, value) {
  settings.menu_visibility = { ...settings.menu_visibility, [key]: value };
  await save();
  emit("settings-changed");
}

async function save() {
  try {
    await invoke("set_app_settings", { settings });
    ElMessage.success("已保存");
  } catch (err) {
    ElMessage.error(`保存失败：${err}`);
  }
}

async function openLog() {
  try {
    await invoke("open_log_file");
  } catch (err) {
    ElMessage.error(`打开日志失败：${err}`);
  }
}

async function openLogDir() {
  try {
    await invoke("open_log_dir");
  } catch (err) {
    ElMessage.error(`打开日志目录失败：${err}`);
  }
}
</script>

<style scoped>
.settings-view {
  max-width: 760px;
  margin: 0 auto;
}
.settings-card {
  margin-bottom: 16px;
}
.muted {
  color: #6b7280;
  font-size: 13px;
}
.ml {
  margin-left: 8px;
}
.mt {
  margin-top: 10px;
}
.menu-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px 16px;
}
.path {
  display: inline-block;
  max-width: 100%;
  overflow-wrap: anywhere;
  background: #f3f4f6;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  color: #1f2937;
}
.actions {
  margin-top: 12px;
}
.diagnostic-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 12px;
  color: #4b5563;
  font-size: 13px;
}
</style>
