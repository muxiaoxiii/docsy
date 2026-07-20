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
        qpdf、poppler、ffmpeg 可在 macOS 和 Windows 下载到 Docsy 自己的工具目录；Word 文件转 PDF 会优先使用 Microsoft
        Word，失败后使用 LibreOffice。
      </p>
      <div v-if="managedToolsDir" class="managed-dir">{{ managedToolsDir }}</div>
      <div class="tool-list">
        <div v-for="tool in tools" :key="tool.name" class="tool-item">
          <div class="tool-info">
            <span class="tool-name">{{ tool.label }}</span>
            <el-tag v-if="tool.checking" type="info" size="small">检测中</el-tag>
            <el-tag v-else-if="tool.status.available" type="success" size="small">可用</el-tag>
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
              v-if="tool.autoInstall"
              size="small"
              @click="installToolFromPackage(tool.name)"
              :loading="tool.installingLocal"
            >
              本地 zip 安装
            </el-button>
            <el-button size="small" @click="openToolDownload(tool)"> 下载页 </el-button>
          </div>
        </div>
      </div>
    </el-card>

    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>主菜单</span>
          <el-button size="small" @click="resetMenuOrder">恢复默认顺序</el-button>
        </div>
      </template>
      <div class="menu-order-list">
        <div v-for="(item, index) in menuSettingsItems" :key="item.id" class="menu-order-item">
          <el-checkbox :model-value="isMenuVisible(item.id)" @change="(value) => setMenuVisible(item.id, value)">
            {{ item.name }}
          </el-checkbox>
          <div class="menu-order-actions">
            <el-button size="small" :disabled="index === 0" @click="moveMenuItem(index, -1)">上移</el-button>
            <el-button size="small" :disabled="index === menuSettingsItems.length - 1" @click="moveMenuItem(index, 1)"
              >下移</el-button
            >
          </div>
        </div>
      </div>
    </el-card>

    <el-card class="settings-section" shadow="never">
      <template #header>
        <div class="card-header">
          <span>模板回收站</span>
          <el-button size="small" :loading="templateTrashLoading" @click="loadTemplateTrash">刷新</el-button>
        </div>
      </template>
      <p class="section-desc">删除的模板先进入回收站；彻底删除会同时删除该模板的内部填写数据。</p>
      <el-table v-if="templateTrash.length" :data="templateTrash" size="small" border>
        <el-table-column prop="name" label="模板" min-width="180" />
        <el-table-column label="字段" width="80">
          <template #default="{ row }">{{ row.fieldCount }}</template>
        </el-table-column>
        <el-table-column prop="updated" label="更新时间" min-width="140">
          <template #default="{ row }">{{ shortDate(row.updated) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="restoreTemplate(row)">恢复</el-button>
            <el-button size="small" link type="danger" @click="permanentlyDeleteTemplate(row)">彻底删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-else description="回收站为空" />
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
        <el-form-item label="工具清单地址">
          <el-input
            v-model="settings.tool_manifest_url"
            placeholder="留空使用默认清单；国内环境可填写 Gitee、对象存储或内网清单地址"
          />
          <span class="form-hint">用于 qpdf、Poppler、FFmpeg 在线安装</span>
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
import { computed, ref, reactive, onMounted } from 'vue'
import { openPath, tauriCallSafe } from '../../../core/tauriBridge.js'
import { defaultMenuOrder, getMenuModules } from '../../../core/moduleRegistry.js'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'

const settings = ref({
  menu_visibility: {},
  menu_order: [],
  libreoffice_path: '',
  tool_manifest_url: '',
})
const managedToolsDir = ref('')
const menuModules = getMenuModules()
const menuSettingsItems = computed(() =>
  normalizedMenuOrder()
    .map((id) => menuModules.find((item) => item.id === id))
    .filter(Boolean),
)

const tools = reactive([
  {
    name: 'qpdf',
    label: 'qpdf',
    description: 'PDF 合并、拆分、叠加和结构处理',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: true,
    downloadUrl: 'https://github.com/qpdf/qpdf/releases',
  },
  {
    name: 'poppler',
    label: 'Poppler',
    description: 'PDF 预览渲染和页眉页脚文本检测',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: true,
    downloadUrl: 'https://github.com/oschwartz10612/poppler-windows/releases',
  },
  {
    name: 'ffmpeg',
    label: 'FFmpeg',
    description: '视频信息读取、抽帧和时间戳水印',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: true,
    downloadUrl: 'https://www.gyan.dev/ffmpeg/builds/',
  },
  {
    name: 'word',
    label: 'Microsoft Word',
    description: 'Word 文件转 PDF 的首选引擎；Windows 使用 COM，macOS 使用 AppleScript',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: false,
    downloadUrl: 'https://www.microsoft.com/microsoft-365/word',
  },
  {
    name: 'wps',
    label: 'WPS Writer',
    description: 'Windows 下 Word 不可用时的第二转换引擎，使用 WPS COM 导出 PDF',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: false,
    downloadUrl: 'https://www.wps.cn/',
  },
  {
    name: 'libreoffice',
    label: 'LibreOffice',
    description: 'Word 文件转 PDF 的备用引擎；没有 Word 或 Word 转换失败时使用',
    status: defaultToolStatus(),
    checking: false,
    installing: false,
    installingLocal: false,
    autoInstall: false,
    downloadUrl: 'https://www.libreoffice.org/download/',
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
const templateTrash = ref([])
const templateTrashLoading = ref(false)

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
    settings.value.menu_visibility = settings.value.menu_visibility || {}
    settings.value.menu_order = Array.isArray(settings.value.menu_order) ? settings.value.menu_order : []
    settings.value.libreoffice_path = settings.value.libreoffice_path || ''
    settings.value.tool_manifest_url = settings.value.tool_manifest_url || ''
  }
}

async function saveSettings() {
  const payload = {
    ...settings.value,
    menu_order: normalizedMenuOrder(),
    libreoffice_path: settings.value.libreoffice_path || null,
    tool_manifest_url: settings.value.tool_manifest_url || null,
  }
  const result = await tauriCallSafe('set_app_settings', { settings: payload })
  if (result.ok) {
    settings.value = { ...settings.value, ...payload }
    window.dispatchEvent(new CustomEvent('docsy-settings-updated', { detail: payload }))
    ElMessage.success('设置已保存')
  } else {
    ElMessage.error(result.error || '保存设置失败')
  }
}

async function loadTemplateTrash() {
  templateTrashLoading.value = true
  const result = await tauriCallSafe('list_template_trash')
  templateTrashLoading.value = false
  if (result.ok) {
    templateTrash.value = result.data || []
  } else {
    ElMessage.error(result.error || '读取模板回收站失败')
  }
}

async function restoreTemplate(row) {
  const result = await tauriCallSafe('restore_template_from_trash', { args: { path: row.path } })
  if (!result.ok) {
    ElMessage.error(result.error || '恢复失败')
    return
  }
  ElMessage.success('模板已恢复')
  await loadTemplateTrash()
}

async function permanentlyDeleteTemplate(row) {
  let migrateToCommon = false
  try {
    await ElMessageBox.confirm(
      `彻底删除“${row.name}”？可以先把该模板的内部填写数据迁移为模板通用数据，供其他模板按通用字段名继续检索。`,
      '彻底删除模板',
      {
        confirmButtonText: '迁移数据并删除',
        cancelButtonText: '直接删除数据',
        distinguishCancelAndClose: true,
        type: 'warning',
      },
    )
    migrateToCommon = true
  } catch (action) {
    if (action !== 'cancel') return
  }
  const result = await tauriCallSafe('permanently_delete_template', {
    args: { path: row.path, migrateToCommon },
  })
  if (!result.ok) {
    ElMessage.error(result.error || '彻底删除失败')
    return
  }
  ElMessage.success(migrateToCommon ? '模板已删除，数据已迁移为模板通用数据' : '模板和内部数据已删除')
  await loadTemplateTrash()
}

function shortDate(value) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

function normalizedMenuOrder() {
  const knownIds = new Set(menuModules.map((item) => item.id))
  const current = Array.isArray(settings.value.menu_order) ? settings.value.menu_order : []
  const ordered = current.filter((id) => knownIds.has(id))
  for (const id of defaultMenuOrder()) {
    if (!ordered.includes(id)) ordered.push(id)
  }
  return ordered
}

function moveMenuItem(index, delta) {
  const order = normalizedMenuOrder()
  const target = index + delta
  if (target < 0 || target >= order.length) return
  const next = [...order]
  ;[next[index], next[target]] = [next[target], next[index]]
  settings.value.menu_order = next
}

function resetMenuOrder() {
  settings.value.menu_order = defaultMenuOrder()
}

function isMenuVisible(id) {
  return (settings.value.menu_visibility || {})[id] !== false
}

function setMenuVisible(id, value) {
  settings.value.menu_visibility = {
    ...(settings.value.menu_visibility || {}),
    [id]: Boolean(value),
  }
}

async function checkTools() {
  await Promise.all(
    tools.map(async (tool) => {
      tool.checking = true
      const result = await tauriCallSafe('check_external_tool', { toolName: tool.name })
      if (result.ok) {
        tool.status = result.data
      }
      tool.checking = false
    }),
  )
  syncDiagnosticToolStatus()
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
    diagnostic.value = { ...diagnostic.value, ...result.data }
  }
}

function syncDiagnosticToolStatus() {
  for (const name of ['qpdf', 'poppler', 'ffmpeg']) {
    const tool = tools.find((t) => t.name === name)
    if (tool) {
      diagnostic.value[name] = {
        available: tool.status.available,
        version: tool.status.version,
      }
    }
  }
}

async function installTool(name) {
  const tool = tools.find((t) => t.name === name)
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

async function installToolFromPackage(name) {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'ZIP 工具包', extensions: ['zip'] }],
  })
  if (!selected) return

  const tool = tools.find((t) => t.name === name)
  if (tool) tool.installingLocal = true
  const result = await tauriCallSafe('install_external_tool_from_package', {
    toolName: name,
    packagePath: selected,
  })
  if (result.ok) {
    ElMessage.success(result.data || '安装完成')
    await checkTools()
    await loadDiagnostic()
  } else {
    ElMessage.error(result.error || '安装失败')
  }
  if (tool) tool.installingLocal = false
}

async function openLogDir() {
  const result = await tauriCallSafe('open_log_dir')
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开日志目录')
  }
}

async function openLogFile() {
  const result = await tauriCallSafe('open_log_file')
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开日志文件')
  }
}

async function openManagedToolsDir() {
  const result = await tauriCallSafe('open_managed_tools_dir')
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开工具目录')
  }
}

async function openToolDownload(tool) {
  const result = await openPath(tool.downloadUrl)
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开下载页')
  }
}

onMounted(() => {
  loadSettings()
  loadManagedToolsDir()
  loadDiagnostic()
  loadTemplateTrash()
  checkTools()
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

.menu-order-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.menu-order-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  background: #f5f7fa;
  border-radius: 4px;
}

.menu-order-actions {
  display: flex;
  gap: 8px;
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
