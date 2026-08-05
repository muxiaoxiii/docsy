<template>
  <section class="workspace">
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>完整表单历史</h3>
          <p>按模板分组保存每次生成文书时填写的完整表单。点击记录可直接带入字段并切到填写页。</p>
        </div>
        <el-button :loading="historyRunsLoading" @click="$emit('refresh-history')">刷新</el-button>
      </div>

      <div v-if="groupedHistoryRuns.length" class="history-group-list">
        <section v-for="group in groupedHistoryRuns" :key="group.templateId" class="history-group">
          <div class="history-group-header">
            <div>
              <h4>{{ group.templateName }}</h4>
              <span>{{ group.runs.length }} 条记录</span>
            </div>
            <el-button size="small" text @click="$emit('open-history-template', group.templatePath)"
              >打开模板</el-button
            >
          </div>
          <div class="history-run-list">
            <article
              v-for="run in visibleGroupRuns(group)"
              :key="run.id"
              class="history-run-card"
              @click="$emit('apply-history-run', run)"
            >
              <div class="history-run-main">
                <div class="history-run-title-row">
                  <strong>{{ historyTime(run.generatedAt) }}</strong>
                  <el-tag v-if="run.source === 'batch'" size="small" type="warning" effect="plain">
                    批量填写记录
                  </el-tag>
                </div>
                <span>{{ fileName(run.outputPath) }}</span>
                <div class="history-run-fields">
                  <el-tag
                    v-for="item in historyRunSummary(run)"
                    :key="`${run.id}-${item.label}-${item.display}`"
                    size="small"
                    effect="plain"
                  >
                    {{ item.label }}：{{ item.display }}
                  </el-tag>
                </div>
              </div>
              <div class="history-run-actions">
                <el-button size="small" type="primary" @click.stop="$emit('apply-history-run', run)">填入</el-button>
                <el-button size="small" text @click.stop="$emit('open-path', run.outputPath)">打开文档</el-button>
              </div>
            </article>
            <div
              v-if="group.runs.length > HISTORY_PAGE_SIZE && !expandedHistoryGroups.has(group.templateId)"
              class="history-expand-row"
            >
              <el-button
                size="small"
                text
                type="primary"
                @click="$emit('expand-history-group', group.templateId)"
              >
                展开全部 {{ group.runs.length }} 条
              </el-button>
            </div>
            <div v-else-if="expandedHistoryGroups.has(group.templateId)" class="history-expand-row">
              <el-button size="small" text @click="$emit('collapse-history-group', group.templateId)">收起</el-button>
            </div>
          </div>
        </section>
      </div>
      <el-empty v-else description="还没有生成记录" />
    </div>
  </section>
</template>

<script setup>
import { fileName } from '../../../core/filePath.js'
import { historyTime, historyRunSummary } from '../composables/fieldRowUtils.js'

const HISTORY_PAGE_SIZE = 20

const props = defineProps({
  historyRunsLoading: { type: Boolean, default: false },
  groupedHistoryRuns: { type: Array, default: () => [] },
  expandedHistoryGroups: { type: Object, default: () => new Set() },
})

defineEmits([
  'refresh-history',
  'apply-history-run',
  'open-history-template',
  'open-path',
  'expand-history-group',
  'collapse-history-group',
])

function visibleGroupRuns(group) {
  if (props.expandedHistoryGroups.has(group.templateId)) return group.runs
  return group.runs.slice(0, HISTORY_PAGE_SIZE)
}
</script>
