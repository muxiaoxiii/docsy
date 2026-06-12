<template>
  <div class="home-view">
    <div class="hero">
      <img src="../../assets/docsy-logo.png" alt="Docsy" class="hero-logo" />
      <h1>欢迎使用 Docsy</h1>
      <p class="subtitle">轻量、高效的本地文档处理工具箱 v{{ version }}</p>
    </div>

    <!-- Quick Access Cards -->
    <div class="section">
      <h3>快捷入口</h3>
      <div class="cards">
        <el-card
          v-for="card in homeCards"
          :key="card.route"
          class="home-card"
          shadow="hover"
          @click="router.push({ name: card.route })"
        >
          <div class="card-icon">
            <el-icon :size="28"><component :is="card.icon" /></el-icon>
          </div>
          <h4>{{ card.title }}</h4>
          <p>{{ card.description }}</p>
        </el-card>
      </div>
    </div>

    <!-- Recent Templates -->
    <div class="section" v-if="recentTemplates.length">
      <h3>最近模板</h3>
      <div class="template-list">
        <div
          v-for="tpl in recentTemplates"
          :key="tpl.id"
          class="template-item"
          @click="router.push({ name: 'doc-gen-form', params: { templateId: tpl.id } })"
        >
          <el-icon><Document /></el-icon>
          <span class="tpl-name">{{ tpl.name }}</span>
          <el-tag v-if="tpl.builtin" size="small" type="info">内置</el-tag>
          <el-tag v-if="tpl.pinned_to_tab" size="small" type="success">已固定</el-tag>
        </div>
      </div>
    </div>

    <!-- Recent Records -->
    <div class="section" v-if="recentRecords.length">
      <h3>最近记录</h3>
      <div class="record-list">
        <div v-for="rec in recentRecords" :key="rec.id" class="record-item">
          <span class="rec-label">{{ rec.label }}</span>
          <span class="rec-time">{{ rec.timestamp }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getHomeCards } from '../../core/moduleRegistry.js'
import { tauriCallSafe } from '../../core/tauriBridge.js'

const router = useRouter()
const homeCards = getHomeCards()
const version = ref('0.5.2')
const recentTemplates = ref([])
const recentRecords = ref([])

async function loadData() {
  // Load templates
  const tplResult = await tauriCallSafe('list_templates')
  if (tplResult.ok) {
    recentTemplates.value = tplResult.data.slice(0, 5)
  }

  // Load recent records across all templates
  if (tplResult.ok && tplResult.data.length > 0) {
    const allRecords = []
    for (const tpl of tplResult.data.slice(0, 3)) {
      const recResult = await tauriCallSafe('list_generation_records', { templateId: tpl.id })
      if (recResult.ok) {
        allRecords.push(...recResult.data.slice(0, 3))
      }
    }
    allRecords.sort((a, b) => b.timestamp.localeCompare(a.timestamp))
    recentRecords.value = allRecords.slice(0, 5)
  }

  // Load version from diagnostic
  const diag = await tauriCallSafe('get_diagnostic_info')
  if (diag.ok && diag.data.version) {
    version.value = diag.data.version
  }
}

onMounted(loadData)
</script>

<style scoped>
.home-view {
  max-width: 800px;
  margin: 0 auto;
  padding: 30px 20px;
}

.hero {
  text-align: center;
  margin-bottom: 32px;
}

.hero-logo {
  width: 56px;
  height: 56px;
}

.hero h1 {
  margin: 12px 0 6px;
  color: #303133;
  font-size: 22px;
}

.subtitle {
  color: #909399;
  font-size: 13px;
}

.section {
  margin-bottom: 28px;
}

.section h3 {
  margin: 0 0 12px;
  font-size: 15px;
  color: #303133;
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.home-card {
  cursor: pointer;
  text-align: center;
  transition: transform 0.15s, box-shadow 0.15s;
}

.home-card:hover {
  transform: translateY(-2px);
}

.card-icon {
  margin-bottom: 8px;
  color: #409eff;
}

.home-card h4 {
  margin: 0 0 4px;
  font-size: 14px;
  color: #303133;
}

.home-card p {
  color: #909399;
  font-size: 12px;
  margin: 0;
}

.template-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.template-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}

.template-item:hover {
  background: #f5f7fa;
}

.tpl-name {
  flex: 1;
  font-size: 13px;
}

.record-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.record-item {
  display: flex;
  justify-content: space-between;
  padding: 6px 12px;
  background: #f9f9f9;
  border-radius: 4px;
  font-size: 13px;
}

.rec-label {
  color: #303133;
}

.rec-time {
  color: #909399;
  font-size: 12px;
}
</style>
