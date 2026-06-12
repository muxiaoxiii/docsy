<template>
  <div class="settings-view">
    <h2>设置</h2>

    <!-- External Tools Status -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <span>外部工具状态</span>
      </template>
      <div class="tool-list">
        <div v-for="tool in tools" :key="tool.name" class="tool-item">
          <div class="tool-info">
            <span class="tool-name">{{ tool.name }}</span>
            <el-tag v-if="tool.status.available" type="success" size="small">可用</el-tag>
            <el-tag v-else type="danger" size="small">未安装</el-tag>
          </div>
          <div class="tool-detail" v-if="tool.status.available">
            <span class="tool-path">{{ tool.status.path }}</span>
            <span class="tool-version" v-if="tool.status.version">{{ tool.status.version }}</span>
          </div>
          <div class="tool-actions" v-else>
            <span class="install-hint">{{ tool.status.install_hint }}</span>
            <el-button size="small" type="primary" @click="installTool(tool.name)" :loading="tool.installing">
              自动安装
            </el-button>
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
        <el-form-item label="历史上限">
          <el-input-number v-model="settings.history_max" :min="10" :max="500" />
          <span class="form-hint">每个模板最多保留的生成记录数</span>
        </el-form-item>
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
        <el-descriptions-item label="ffmpeg">{{ diagnostic.ffmpeg?.version || '不可用' }}</el-descriptions-item>
      </el-descriptions>
      <div class="diag-actions">
        <el-button size="small" @click="openLogDir">打开日志目录</el-button>
        <el-button size="small" @click="openLogFile">打开当前日志</el-button>
      </div>
    </el-card>

    <!-- Config Import/Export -->
    <el-card class="settings-section" shadow="never">
      <template #header>
        <span>配置导入导出</span>
      </template>
      <p class="section-desc">导出配置为 .docsybundle 文件，可在其他电脑导入</p>
      <div class="bundle-actions">
        <el-button type="primary" @click="handleExport">导出配置</el-button>
        <el-button @click="handleImport">导入配置</el-button>
      </div>
      <el-checkbox-group v-model="exportOptions" class="export-options">
        <el-checkbox label="templates">模板</el-checkbox>
        <el-checkbox label="dictionaries">字典</el-checkbox>
        <el-checkbox label="parties">当事人主档</el-checkbox>
        <el-checkbox label="field_history">字段历史</el-checkbox>
        <el-checkbox label="settings">应用设置</el-checkbox>
      </el-checkbox-group>
    </el-card>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { tauriCall, tauriCallSafe } from '../../../core/tauriBridge.js'

const settings = ref({
  history_max: 50,
  menu_visibility: {},
  libreoffice_path: '',
})

const tools = reactive([
  { name: 'qpdf', status: { available: false, path: null, version: null, install_hint: '' }, installing: false },
  { name: 'ffmpeg', status: { available: false, path: null, version: null, install_hint: '' }, installing: false },
  { name: 'libreoffice', status: { available: false, path: null, version: null, install_hint: '' }, installing: false },
])

const diagnostic = ref({
  version: '',
  os: '',
  arch: '',
  qpdf: null,
  ffmpeg: null,
})

const exportOptions = ref(['templates', 'dictionaries', 'parties'])

async function handleExport() {
  const { save } = await import('@tauri-apps/plugin-dialog')
  const path = await save({
    filters: [{ name: 'Docsy Bundle', extensions: ['docsybundle'] }],
    defaultPath: `Docsy-Bundle-${new Date().toISOString().slice(0, 10)}.docsybundle`,
  })
  if (!path) return
  const options = {}
  for (const opt of exportOptions.value) options[opt] = true
  const res = await tauriCallSafe('export_bundle', { path, options })
  if (res.ok) {
    console.log('Exported:', res.data)
  }
}

async function handleImport() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selected = await open({
    filters: [{ name: 'Docsy Bundle', extensions: ['docsybundle'] }],
  })
  if (!selected) return
  const res = await tauriCallSafe('import_bundle', {
    path: selected,
    options: { templates: true, dictionaries: true, parties: true },
  })
  if (res.ok) {
    console.log('Imported:', res.data)
  }
}

async function loadSettings() {
  const result = await tauriCallSafe('get_app_settings')
  if (result.ok) {
    settings.value = { ...settings.value, ...result.data }
  }
}

async function saveSettings() {
  await tauriCallSafe('set_app_settings', { settings: settings.value })
}

async function checkTools() {
  for (const tool of tools) {
    const result = await tauriCallSafe('check_external_tool', { toolName: tool.name })
    if (result.ok) {
      tool.status = result.data
    }
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
  await tauriCallSafe('install_external_tool', { toolName: name })
  await checkTools()
  if (tool) tool.installing = false
}

async function openLogDir() {
  await tauriCallSafe('open_log_dir')
}

async function openLogFile() {
  await tauriCallSafe('open_log_file')
}

onMounted(() => {
  loadSettings()
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
  gap: 12px;
  font-size: 12px;
  color: #909399;
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
