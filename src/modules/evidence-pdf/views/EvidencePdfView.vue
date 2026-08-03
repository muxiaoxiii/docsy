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
        <ToolWorkspaceShell title="证据扫描" description="扫描文件夹并按子文件夹自动整理、合并证据 PDF。">
          <template #toolbar>
            <el-button type="primary" @click="selectEvidenceFolder">选择证据文件夹</el-button>
            <el-button v-if="evidenceFolder" :loading="scanning" @click="scanEvidence">重新扫描</el-button>
          </template>
          <div v-if="evidenceFolder" class="evidence-info">
            <span class="path-label">当前文件夹</span>
            <p>{{ evidenceFolder }}</p>
          </div>
          <div v-if="evidenceGroups.length" class="evidence-groups">
            <div v-for="group in evidenceGroups" :key="group.name" class="group-item">
              <div class="group-head">
                <h4>{{ group.name }}</h4>
                <el-tag size="small" type="info">{{ group.files.length }} 个文件</el-tag>
              </div>
              <FileQueuePanel
                :items="group.files"
                :clearable="false"
                :removable="false"
                sortable
                max-height="240px"
                @reorder="(payload) => reorderGroupFiles(group, payload)"
              />
            </div>
          </div>
          <el-empty v-else :description="evidenceFolder ? '尚未扫描到证据分组' : '先选择需要扫描的证据文件夹'" />
          <el-alert
            v-if="conversionFailures.length"
            class="conversion-alert"
            type="warning"
            :closable="false"
            show-icon
            title="部分 Word 文件未能转换"
          >
            <div class="conversion-failures">
              <div v-for="item in conversionFailures" :key="item.path" class="conversion-failure">
                <strong>{{ item.name }}</strong>
                <span>{{ item.groupName }}</span>
                <p>{{ item.reason }}</p>
              </div>
            </div>
          </el-alert>
          <template #actions>
            <el-button type="success" :disabled="!evidenceGroups.length" :loading="building" @click="buildEvidence">
              生成合并 PDF
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import EvidencePdfWorkbench from '../../pdf-tools/views/EvidencePdfWorkbench.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

const activeTab = ref('merge')
const evidenceFolder = ref('')
const evidenceGroups = ref([])
const scanning = ref(false)
const building = ref(false)
const conversionFailures = ref([])

function reorderGroupFiles(group, { from, to }) {
  group.files = moveItem(group.files, from, to)
}

async function selectEvidenceFolder() {
  const selected = await open({ directory: true })
  if (selected) evidenceFolder.value = selected
}

async function scanEvidence() {
  if (!evidenceFolder.value) return
  scanning.value = true
  conversionFailures.value = []
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
    args: {
      root: evidenceFolder.value,
      groups: evidenceGroups.value,
    },
  })
  if (result.ok) {
    conversionFailures.value = result.data.failedConversions || []
    if (conversionFailures.value.length) {
      ElMessage.warning(`证据 PDF 已生成，${conversionFailures.value.length} 个 Word 文件未能转换`)
    } else {
      ElMessage.success('证据 PDF 生成完成')
    }
  } else {
    ElMessage.error(result.error || '证据 PDF 生成失败')
  }
  building.value = false
}
</script>

<style scoped>
.evidence-pdf-view,
.evidence-tabs {
  height: 100%;
  min-height: 0;
}

.evidence-pdf-view {
  overflow: hidden;
  background: var(--docsy-surface);
}

:deep(.evidence-tabs > .el-tabs__content),
:deep(.evidence-tabs > .el-tabs__content > .el-tab-pane) {
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.evidence-info {
  padding: 10px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 5px;
  background: var(--docsy-surface-muted);
  font-size: 13px;
  color: var(--docsy-text);
}

.evidence-info p {
  margin: 4px 0 0;
  word-break: break-all;
}

.path-label {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.evidence-groups {
  display: grid;
  gap: 10px;
}

.group-item {
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 5px;
  background: var(--docsy-surface-elevated);
}

.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.group-item h4 {
  margin: 0;
  font-size: 13px;
}

.group-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 10px;
  margin-top: 8px;
}

.group-file {
  font-size: 12px;
  color: var(--docsy-text-muted);
  word-break: break-all;
}

.conversion-alert {
  margin-top: 16px;
}

.conversion-failures {
  display: grid;
  gap: 8px;
}

.conversion-failure {
  font-size: 12px;
  line-height: 1.5;
}

.conversion-failure strong {
  display: block;
  color: var(--docsy-text-strong);
}

.conversion-failure span {
  color: var(--docsy-text-muted);
}

.conversion-failure p {
  margin: 2px 0 0;
  word-break: break-all;
}

@media (max-width: 1280px) {
  :deep(.evidence-tabs > .el-tabs__content > .el-tab-pane) {
    overflow: auto;
  }
}
</style>
