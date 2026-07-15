<template>
  <div class="evidence-pdf-view">
    <el-tabs v-model="activeTab" tab-position="left" class="evidence-tabs">
      <el-tab-pane label="分项证据处理" name="merge" lazy>
        <EvidencePdfWorkbench workflow="merge" />
      </el-tab-pane>
      <el-tab-pane label="合并证据处理" name="split" lazy>
        <EvidencePdfWorkbench workflow="split" />
      </el-tab-pane>
      <el-tab-pane label="证据扫描" name="scan" lazy>
        <div class="tab-content">
          <h3>证据扫描</h3>
          <p class="hint">扫描文件夹，按子文件夹自动分组合并 PDF</p>
          <el-button type="primary" @click="selectEvidenceFolder">选择证据文件夹</el-button>
          <div v-if="evidenceFolder" class="evidence-info">
            <p>文件夹: {{ evidenceFolder }}</p>
            <el-button @click="scanEvidence" :loading="scanning">扫描</el-button>
          </div>
          <div v-if="evidenceGroups.length" class="evidence-groups">
            <div v-for="group in evidenceGroups" :key="group.name" class="group-item">
              <h4>{{ group.name }} ({{ group.files.length }} 个文件)</h4>
              <div class="group-files">
                <span v-for="f in group.files" :key="f.path" class="group-file">{{ f.name }}</span>
              </div>
            </div>
            <el-button type="success" @click="buildEvidence" :loading="building">生成合并 PDF</el-button>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import EvidencePdfWorkbench from '../../pdf-tools/views/EvidencePdfWorkbench.vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

const activeTab = ref('merge')
const evidenceFolder = ref('')
const evidenceGroups = ref([])
const scanning = ref(false)
const building = ref(false)

async function selectEvidenceFolder() {
  const selected = await open({ directory: true })
  if (selected) evidenceFolder.value = selected
}

async function scanEvidence() {
  if (!evidenceFolder.value) return
  scanning.value = true
  const result = await tauriCallSafe('scan_evidence_folder', { root: evidenceFolder.value })
  if (result.ok) {
    evidenceGroups.value = result.data.groups || []
  } else {
    ElMessage.error(result.error || '扫描失败')
  }
  scanning.value = false
}

async function buildEvidence() {
  building.value = true
  const result = await tauriCallSafe('build_evidence_group_pdfs', {
    root: evidenceFolder.value,
    groups: evidenceGroups.value,
  })
  result.ok ? ElMessage.success('证据 PDF 生成完成') : ElMessage.error(result.error || '证据 PDF 生成失败')
  building.value = false
}
</script>

<style scoped>
.evidence-pdf-view,
.evidence-tabs {
  height: 100%;
}

.tab-content {
  padding: 16px;
  max-width: 680px;
}

h3 {
  margin: 0 0 6px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin: 0 0 16px;
}

.evidence-info {
  margin-top: 12px;
  font-size: 13px;
  color: #606266;
}

.evidence-groups {
  margin-top: 16px;
}

.group-item {
  margin-bottom: 12px;
  padding: 10px;
  background: #f9f9f9;
  border-radius: 4px;
}

.group-item h4 {
  margin: 0 0 6px;
  font-size: 13px;
}

.group-files {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.group-file {
  font-size: 12px;
  color: #909399;
}
</style>
