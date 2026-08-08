<template>
  <div class="home-view">
    <div class="hero">
      <div class="hero-copy">
        <h1>Docsy</h1>
        <p class="subtitle">轻量、高效的本地文档处理工具箱 v{{ version }}</p>
      </div>
      <img src="../../assets/doclet-mascot-transparent.png" alt="Docsy" class="hero-mascot" />
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
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getHomeCards } from '../../core/moduleRegistry.js'
import { tauriCallSafe } from '../../core/tauriBridge.js'

const router = useRouter()
const settings = ref({
  menu_visibility: {},
  menu_order: [],
})
const homeCards = computed(() => getHomeCards(settings.value))
const version = ref(import.meta.env.PACKAGE_VERSION || '')

async function loadData() {
  const appSettings = await tauriCallSafe('get_app_settings')
  if (appSettings.ok) {
    settings.value = { ...settings.value, ...appSettings.data }
  }
  const diag = await tauriCallSafe('get_diagnostic_info')
  if (diag.ok && diag.data.version) {
    version.value = diag.data.version
  }
}

function applySettingsEvent(event) {
  settings.value = { ...settings.value, ...(event.detail || {}) }
}

onMounted(() => {
  loadData()
  window.addEventListener('docsy-settings-updated', applySettingsEvent)
})

onBeforeUnmount(() => {
  window.removeEventListener('docsy-settings-updated', applySettingsEvent)
})
</script>

<style scoped>
.home-view {
  max-width: 1040px;
  margin: 0 auto;
  padding: 34px 28px 40px;
}

.hero {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 36px;
  margin-bottom: 28px;
  min-height: 220px;
}

.hero-copy {
  min-width: 260px;
}

.hero h1 {
  margin: 0 0 8px;
  color: var(--docsy-text-strong);
  font-size: 34px;
  line-height: 1.1;
}

.subtitle {
  color: var(--docsy-text-muted);
  font-size: 13px;
  margin: 0;
}

.hero-mascot {
  width: min(220px, 38vw);
  max-height: 260px;
  object-fit: contain;
}

.section {
  margin-bottom: 28px;
}

.section h3 {
  margin: 0 0 12px;
  font-size: 15px;
  color: var(--docsy-text-strong);
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.home-card {
  cursor: pointer;
  min-height: 142px;
  text-align: left;
  border-color: var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  transition:
    transform 200ms var(--ease-out),
    box-shadow 200ms var(--ease-out);
}

.home-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 12px 36px rgba(54, 45, 36, 0.12);
  border-color: var(--docsy-primary);
}

.home-card:active {
  transform: scale(0.98);
}

.card-icon {
  margin-bottom: 8px;
  color: var(--docsy-primary);
}

.home-card h4 {
  margin: 0 0 4px;
  font-size: 14px;
  color: var(--docsy-text-strong);
}

.home-card p {
  color: var(--docsy-text-muted);
  font-size: 12px;
  margin: 0;
}

@media (max-width: 720px) {
  .hero {
    flex-direction: column-reverse;
    gap: 16px;
    text-align: center;
  }

  .hero-copy {
    min-width: 0;
  }

  .hero-mascot {
    width: 180px;
  }
}
</style>
