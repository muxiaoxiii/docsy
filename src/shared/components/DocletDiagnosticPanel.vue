<template>
  <aside class="doclet-diagnostic" aria-label="Doclet 绘制诊断">
    <button class="diagnostic-toggle" @click="expanded = !expanded">
      独立诊断包 · {{ modeLetter }} {{ expanded ? '收起' : '展开' }}
    </button>
    <div v-if="expanded" class="diagnostic-body">
      <label for="gaze-diagnostic-mode">绘制方式（切换会重建画布）</label>
      <select id="gaze-diagnostic-mode" v-model="gazeDiagnosticMode" @change="modeChanged">
        <option v-for="mode in gazeDiagnosticModes" :key="mode.value" :value="mode.value">{{ mode.label }}</option>
      </select>
      <p>同一页面分别试 A、B、C、D。鼠标停留约半秒触发转头，再移回角色附近；每种方式重复几轮，也试试点击动作。</p>
      <div class="diagnostic-actions">
        <button @click="mark('black')">记录：看到黑底</button>
        <button @click="mark('clear')">记录：背景正常</button>
        <button @click="copy">复制诊断信息</button>
      </div>
      <p role="status">{{ status || '仅本地记录绘制过程与环境，不读取文件内容，不自动上传。' }}</p>
      <textarea v-if="report" v-model="report" readonly aria-label="诊断信息，复制失败时可手动全选复制" rows="5" />
    </div>
  </aside>
</template>
<script setup>
import { computed, ref } from 'vue'
import {
  gazeDiagnosticMode,
  gazeDiagnosticModes,
  recordGazeDiagnostic,
  gazeDiagnosticReport,
} from '../composables/docletGazeDiagnostics.js'
const modeLetter = computed(() => gazeDiagnosticModes.find((entry) => entry.value === gazeDiagnosticMode.value).label.slice(0, 1))
const expanded = ref(true)
const status = ref('')
const report = ref('')
function modeChanged() {
  recordGazeDiagnostic('mode-selected')
  status.value = '已切换；请将鼠标移到页面不同方向，停留后观察。'
  report.value = ''
}
function mark(result) {
  recordGazeDiagnostic('observation', { result, page: window.location.pathname })
  status.value = `已记录 ${modeLetter.value}：${result === 'black' ? '看到黑底' : '背景正常'}`
}
async function copy() {
  report.value = gazeDiagnosticReport()
  try {
    await navigator.clipboard.writeText(report.value)
    status.value = '已复制；可以把诊断信息发给我。'
  } catch {
    status.value = '请在下方文本框中全选复制诊断信息。'
  }
}
</script>
<style scoped>
.doclet-diagnostic {
  position: fixed;
  z-index: 3000;
  right: 18px;
  bottom: 16px;
  width: 340px;
  max-width: calc(100vw - 36px);
  color: #263c30;
  background: #fffdf8;
  border: 1px solid #b7c9b9;
  border-radius: 10px;
  box-shadow: 0 4px 20px #0002;
  font-size: 13px;
}
.diagnostic-toggle {
  width: 100%;
  text-align: left;
  font-weight: 600;
}
.diagnostic-body {
  padding: 0 12px 10px;
}
select,
textarea {
  width: 100%;
  box-sizing: border-box;
  margin-top: 6px;
  border: 1px solid #b7c9b9;
  border-radius: 8px;
  padding: 6px;
  background: white;
  color: inherit;
}
p {
  line-height: 1.6;
  margin: 8px 0;
}
button {
  border: 1px solid #b7c9b9;
  border-radius: 8px;
  background: #f1f5ee;
  color: inherit;
  cursor: pointer;
  padding: 7px 9px;
  font: inherit;
}
.diagnostic-toggle {
  border: 0;
  background: transparent;
  padding: 12px;
}
.diagnostic-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
</style>
