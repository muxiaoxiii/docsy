<template>
  <div class="evidence-pdf-view">
    <el-tabs v-model="activeTab" :tab-position="tabPosition" class="evidence-tabs">
      <el-tab-pane label="分项证据合并" name="merge" lazy>
        <EvidencePdfWorkbench workflow="merge" />
      </el-tab-pane>
      <el-tab-pane label="合并证据拆分" name="split" lazy>
        <EvidencePdfWorkbench workflow="split" />
      </el-tab-pane>
      <el-tab-pane label="证据扫描" name="scan" lazy>
        <ToolWorkspaceShell
          title="证据扫描"
          description="扫描 PDF、Word 和静态图片，母文件夹的文件单独成组；可按文件夹分别输出或全部合并，签章外观压平用于打印。"
        >
          <template #toolbar>
            <el-button type="primary" plain :disabled="scanning || building" @click="selectEvidenceFolder">选择证据文件夹</el-button>
            <el-button v-if="evidenceFolder" :loading="scanning" :disabled="building" @click="scanEvidence">重新扫描</el-button>
          </template>
          <div class="scan-options compact-form">
            <el-radio-group v-model="outputMode" size="small" :disabled="scanning || building"><el-radio-button value="per_folder">各文件夹分别合并</el-radio-button><el-radio-button value="all">全部合成一份</el-radio-button></el-radio-group>
            <span class="scan-separator-option"><el-checkbox v-model="duplexFiles" :disabled="scanning || building">文件夹内按文件补齐双面</el-checkbox><HelpTooltip content="每份材料为奇数页时在末尾补一页空白，让下一份材料从正面起印。预览中合成的一批图片算作一份材料。" /></span>
            <span v-if="outputMode === 'all'" class="scan-separator-option"><el-checkbox v-model="duplexGroups" :disabled="scanning || building">文件夹之间补齐双面</el-checkbox><HelpTooltip content="先合并各文件夹，文件夹总页数为奇数时补一页空白，让下一个文件夹从正面起印。可与文件夹内分隔同时开启，偶数页不重复补白。母文件夹的本层文件也作为一组。" /></span>
          </div>
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
                :sortable="!scanning && !building"
                max-height="240px"
                @reorder="(payload) => reorderGroupFiles(group, payload)"
              >
                <template #meta="{ item }">
                  <el-tag :type="documentKind(item.path) === 'word' ? 'warning' : 'info'" size="small">
                    {{ item.fromImages ? '图片已排版' : documentKind(item.path) === 'word' ? 'Word' : documentKind(item.path) === 'image' ? '图片' : 'PDF' }}
                  </el-tag>
                </template>
              </FileQueuePanel>
            </div>
          </div>
          <WorkspaceEmptyState
            v-else
            scene="evidence"
            :state="scanning ? 'loading' : 'empty'"
            :icon-url="evidenceIconUrl"
            :title="scanning ? '正在扫描证据文件夹' : evidenceFolder ? '尚未发现证据分组' : '等待选择证据文件夹'"
            :description="
              evidenceFolder
                ? '当前文件夹中还没有可整理的证据文件。'
                : '选择文件夹后，Docsy 会按目录自动识别并整理证据。'
            "
          ><template #actions><el-button type="primary" plain :disabled="scanning || building" @click="selectEvidenceFolder">选择证据文件夹</el-button></template></WorkspaceEmptyState>
          <el-alert
            v-if="conversionFailures.length"
            class="conversion-alert"
            type="warning"
            :closable="false"
            show-icon
            title="部分文件未能转换"
          >
            <div class="conversion-failures">
              <div v-for="item in conversionFailures" :key="item.path" class="conversion-failure">
                <strong>{{ item.name }}</strong>
                <span>{{ item.groupName }}</span>
                <p>{{ item.reason }}</p>
                <el-button size="small" :disabled="scanning || building" @click="replaceFailedConversion(item)">选择手动转换的 PDF</el-button>
              </div>
            </div>
          </el-alert>
          <el-alert v-if="groupFailures.length" type="warning" :closable="false" show-icon title="部分分组未生成，未输出缺少文件的合并件">
            <p v-for="item in groupFailures" :key="item.groupId">{{ item.name }}：{{ item.reason }}</p>
          </el-alert>
          <template #actions>
            <span v-if="evidenceOutputDir" class="output-path">输出文件夹：{{ evidenceOutputDir }}</span>
            <el-button v-if="evidenceOutputDir" @click="openPath(evidenceOutputDir)">打开文件夹</el-button>
            <el-button type="success" :disabled="!evidenceGroups.length || scanning" :loading="building" @click="buildEvidence">
              生成合并 PDF
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>
    </el-tabs>
    <ImagePdfImportDialog ref="imageImportDialog" />
  </div>
</template>

<script setup>
import ImagePdfImportDialog from '../../../shared/pdf-tools/components/ImagePdfImportDialog.vue'
import HelpTooltip from '../../../shared/components/HelpTooltip.vue'
import { documentKind } from '../../../shared/pdf-tools/documentImport.js'
const imageImportDialog = ref(null)
import { useResponsiveTabs } from '../../../shared/composables/useResponsiveTabs.js'
import { openPath } from '../../../core/tauriBridge.js'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import EvidencePdfWorkbench from '../components/EvidencePdfWorkbench.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import evidenceIconUrl from '../../../assets/icons/evidence.svg?url'
import { moveItem } from '../../../shared/components/reorderableItems.js'
import { useEvidenceFolder } from '../../../shared/pdf-tools/composables/useEvidenceFolder.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'

const tabPosition = useResponsiveTabs()

const activeTab = ref('merge')
const { outputMode, duplexFiles, duplexGroups, evidenceFolder, evidenceGroups, scanning, building, conversionFailures, groupFailures, evidenceOutputDir, selectEvidenceFolder, scanEvidence, buildEvidence, replaceFailedConversion } = useEvidenceFolder({ reviewImages: (paths, options) => imageImportDialog.value.open(paths, options) })
const preference = useWorkspacePreferences('evidence-pdf.workspace', { activeTab, outputMode, duplexFiles, duplexGroups })

function reorderGroupFiles(group, { from, to }) {
  if (!scanning.value && !building.value) group.files = moveItem(group.files, from, to)
}

onMounted(() => void preference.start())
onBeforeUnmount(() => void preference.stop())
</script>

<style scoped>
.scan-options { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 18px; margin-bottom: 12px; }
.scan-separator-option { display: inline-flex; align-items: center; gap: 4px; }
.scan-separator-option :deep(.el-checkbox) { margin-right: 0; }
.evidence-pdf-view,
.evidence-tabs {
  height: 100%;
  min-height: 0;
}

.evidence-pdf-view {
  overflow: hidden;
  background: var(--docsy-canvas);
}

:deep(.evidence-tabs > .el-tabs__content),
:deep(.evidence-tabs > .el-tabs__content > .el-tab-pane) {
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.evidence-info {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  margin-bottom: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  font-size: 13px;
  color: var(--docsy-text);
}

.evidence-info p {
  margin: 0;
  overflow-wrap: anywhere;
}

.output-path {
  min-width: 0;
  margin-right: auto;
  color: var(--docsy-text-muted);
  font-size: 12px;
  overflow-wrap: anywhere;
}

.path-label {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 11px;
  font-weight: 600;
}

.evidence-groups {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
}

.group-item {
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-soft);
}

.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--docsy-border-subtle);
}

.group-item h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: var(--docsy-text-strong);
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
  overflow-wrap: anywhere;
}

@media (max-width: 1280px) {
  :deep(.evidence-tabs > .el-tabs__content > .el-tab-pane) {
    overflow: auto;
  }
}
</style>
