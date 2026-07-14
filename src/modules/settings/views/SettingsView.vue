<template>
  <div class="settings-view">
    <h2>设置</h2>

    <!-- External Tools Status -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>外部工具状态</span>
          <el-button size="small" @click="openManagedToolsDir">打开 Docsy 工具目录</el-button>
        </div>
      </template>
      <p class="section-desc">
        qpdf、poppler、ffmpeg 可在 macOS 和 Windows 下载到 Docsy 自己的工具目录；LibreOffice 体积较大，保留外部安装和路径配置。
      </p>
      <div v-if="managedToolsDir" class="managed-dir">{{ managedToolsDir }}</div>
      <div class="tool-list">
        <div v-for="tool in tools" :key="tool.name" class="tool-item">
          <div class="tool-info">
            <span class="tool-name">{{ tool.label }}</span>
            <el-tag v-if="tool.status.available" type="success" size="small">可用</el-tag>
            <el-tag v-else type="danger" size="small">未安装</el-tag>
            <el-tag v-if="tool.status.available" size="small" :type="tool.status.managed ? 'primary' : 'info'">
              {{ tool.status.managed ? 'Docsy 托管' : '系统工具' }}
            </el-tag>
          </div>
          <div class="tool-detail" v-if="tool.status.available">
            <span class="tool-path">{{ tool.status.path }}</span>
            <span class="tool-version" v-if="tool.status.version">{{ tool.status.version }}</span>
          </div>
          <div class="tool-desc">{{ tool.description }}</div>
          <div class="tool-actions" v-if="!tool.status.available">
            <span class="install-hint">{{ tool.status.install_hint }}</span>
            <el-button
              v-if="tool.autoInstall"
              size="small"
              type="primary"
              @click="installTool(tool.name)"
              :loading="tool.installing"
            >
              下载安装到 Docsy
            </el-button>
            <el-button
              v-else-if="tool.name === 'libreoffice'"
              size="small"
              @click="openLibreOfficeDownload"
            >
              打开官方下载页
            </el-button>
            <el-tag v-else size="small" type="info">需手动安装</el-tag>
          </div>
        </div>
      </div>
    </el-card>

    <!-- App Settings -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <span>应用设置</span>
      </template>
      <el-form label-width="120px">
        <el-form-item label="LibreOffice 路径">
          <el-input v-model="settings.libreoffice_path" placeholder="留空则自动检测" />
          <span class="form-hint">用于 DOC/DOCX 转 PDF</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveSettings">保存设置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- Diagnostics -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <span>诊断信息</span>
      </template>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item label="版本">{{ diagnostic.version }}</el-descriptions-item>
        <el-descriptions-item label="系统">{{ diagnostic.os }} / {{ diagnostic.arch }}</el-descriptions-item>
        <el-descriptions-item label="qpdf">{{ diagnostic.qpdf?.version || '不可用' }}</el-descriptions-item>
        <el-descriptions-item label="poppler">{{ diagnostic.poppler?.version || '不可用' }}</el-descriptions-item>
        <el-descriptions-item label="ffmpeg">{{ diagnostic.ffmpeg?.version || '不可用' }}</el-descriptions-item>
      </el-descriptions>
      <div class="diag-actions">
        <el-button size="small" @click="openLogDir">打开日志目录</el-button>
        <el-button size="small" @click="openLogFile">打开当前日志</el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { ElMessage } from 'element-plus'

const settings = ref({
  menu_visibility: {},
  libreoffice_path: '',
})
const managedToolsDir = ref('')

const tools = reactive([
  {
    name: 'qpdf',
    label: 'qpdf',
    description: 'PDF 合并、拆分、叠加和结构处理',
    status: defaultToolStatus(),
    installing: false,
    autoInstall: true,
  },
  {
    name: 'poppler',
    label: 'Poppler',
    description: 'PDF 预览渲染和页眉页脚文本检测',
    status: defaultToolStatus(),
    installing: false,
    autoInstall: true,
  },
  {
    name: 'ffmpeg',
    label: 'FFmpeg',
    description: '视频信息读取、抽帧和时间戳水印',
    status: defaultToolStatus(),
    installing: false,
    autoInstall: true,
  },
  {
    name: 'libreoffice',
    label: 'LibreOffice',
    description: 'DOC/DOCX 转 PDF；建议安装到系统后配置路径',
    status: defaultToolStatus(),
    installing: false,
    autoInstall: false,
  },
])

const diagnostic = ref({
  version: '',
  os: '',
  arch: '',
  qpdf: null,
  poppler: null,
  ffmpeg: null,
})

function defaultToolStatus() {
  return {
    available: false,
    path: null,
    version: null,
    install_hint: '',
    managed: false,
    source: 'missing',
  }
}

async function loadSettings() {
  const result = await tauriCallSafe('get_app_settings')
  if (result.ok) {
    settings.value = { ...settings.value, ...result.data }
  }
}

async function saveSettings() {
  const result = await tauriCallSafe('set_app_settings', { settings: settings.value })
  result.ok ? ElMessage.success('设置已保存') : ElMessage.error(result.error || '保存设置失败')
}

async function checkTools() {
  for (const tool of tools) {
    const result = await tauriCallSafe('check_external_tool', { toolName: tool.name })
    if (result.ok) {
      tool.status = result.data
    }
  }
}

async function loadManagedToolsDir() {
  const result = await tauriCallSafe('get_managed_tools_dir')
  if (result.ok) {
    managedToolsDir.value = result.data
  }
}

async function loadDiagnostic() {
  const result = await tauriCallSafe('get_diagnostic_info')
  if (result.ok) {
    diagnostic.value = result.data
  }
}

async function installTool(name) {
  const tool = tools.find(t => t.name === name)
  if (tool) tool.installing = true
  const result = await tauriCallSafe('install_external_tool', { toolName: name })
  if (result.ok) {
    ElMessage.success(result.data || '安装完成')
    await checkTools()
    await loadDiagnostic()
  } else {
    ElMessage.error(result.error || '安装失败')
  }
  if (tool) tool.installing = false
}

async function openLogDir() {
  await tauriCallSafe('open_log_dir')
}

async function openLogFile() {
  await tauriCallSafe('open_log_file')
}

async function openManagedToolsDir() {
  const result = await tauriCallSafe('open_managed_tools_dir')
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开工具目录')
  }
}

async function openLibreOfficeDownload() {
  const result = await tauriCallSafe('open_path', { path: 'https://www.libreoffice.org/download/' })
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开 LibreOffice 下载页')
  }
}

onMounted(() => {
  loadSettings()
  loadManagedToolsDir()
  checkTools()
  loadDiagnostic()
})
</script>

<style scoped>
.settings-view {
  max-width: 700px;
  margin: 0 auto;
  padding: 20px;
}

.settings-view h2 {
  margin: 0 0 20px;
  color: #303133;
}

.settings-section {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.managed-dir {
  margin-bottom: 12px;
  padding: 8px 10px;
  color: #606266;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  font-size: 12px;
  word-break: break-all;
}

.tool-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tool-item {
  padding: 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.tool-info {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.tool-name {
  font-weight: 600;
  font-size: 14px;
}

.tool-detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 12px;
  color: #909399;
  word-break: break-all;
}

.tool-desc {
  margin: 4px 0;
  font-size: 12px;
  color: #606266;
}

.tool-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.install-hint {
  font-size: 12px;
  color: #909399;
}

.form-hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}

.diag-actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}

.section-desc {
  font-size: 13px;
  color: #909399;
  margin: 0 0 12px;
}

.bundle-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.export-options {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
