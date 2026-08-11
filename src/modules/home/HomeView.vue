<template>
  <div class="home-view">
    <div class="hero">
      <div class="hero-copy">
        <span class="eyebrow">Docsy Workspace</span>
        <h1>让文档处理，<br />安静地完成。</h1>
        <p class="subtitle">在本地整理证据、转换文档、排版图片和生成文书。文件不离开设备，常用工具始终触手可及。</p>
      </div>
      <div class="hero-art">
        <div class="doclet-stage">
          <img src="../../assets/doclet-mascot-transparent.png" alt="Doclet" class="hero-mascot" />
        </div>
      </div>
    </div>

    <div class="section">
      <div class="section-heading">
        <h2>开始处理</h2>
        <span>选择一个工作空间</span>
      </div>
      <div class="cards">
        <button
          v-for="card in homeCards"
          :key="card.route"
          class="home-card"
          type="button"
          @click="router.push({ name: card.route })"
        >
          <span class="card-topline">
            <span
              class="home-card-icon"
              :style="{ '--home-icon-url': `url(&quot;${homeIconByRoute[card.route]}&quot;)` }"
              aria-hidden="true"
            ></span>
          </span>
          <span class="card-copy">
            <strong>{{ card.title }}</strong>
            <span class="card-arrow">→</span>
          </span>
          <span class="card-description">{{ card.description }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getHomeCards } from '../../core/moduleRegistry.js'
import { tauriCallSafe } from '../../core/tauriBridge.js'
import evidenceIconUrl from '../../assets/icons/evidence.svg?url'
import documentsIconUrl from '../../assets/icons/documents.svg?url'
import imageLayoutIconUrl from '../../assets/icons/image-layout.svg?url'
import videoFramesIconUrl from '../../assets/icons/video-frames.svg?url'
import templateIconUrl from '../../assets/icons/template.svg?url'

const router = useRouter()
const settings = ref({
  menu_visibility: {},
  menu_order: [],
})
const homeCards = computed(() => getHomeCards(settings.value))
const homeIconByRoute = {
  'evidence-pdf': evidenceIconUrl,
  'pdf-tools': documentsIconUrl,
  'image-paddler': imageLayoutIconUrl,
  'video-extract': videoFramesIconUrl,
  template: templateIconUrl,
}
async function loadData() {
  const appSettings = await tauriCallSafe('get_app_settings')
  if (appSettings.ok) {
    settings.value = { ...settings.value, ...appSettings.data }
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
  width: min(1120px, calc(100% - 64px));
  margin: 0 auto;
  padding: 46px 0 54px;
}

.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.35fr) minmax(280px, 0.65fr);
  min-height: 300px;
  overflow: hidden;
  background: var(--docsy-surface);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  box-shadow: var(--docsy-shadow);
}

.hero-copy {
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-width: 0;
  padding: 48px 54px;
}

.eyebrow {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 22px;
  color: var(--docsy-accent);
  font-size: 10px;
  font-weight: 760;
  letter-spacing: 0.17em;
  text-transform: uppercase;
}

.eyebrow::before {
  width: 28px;
  height: 1px;
  content: '';
  background: currentColor;
}

.hero h1 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: clamp(32px, 4vw, 54px);
  font-weight: 760;
  letter-spacing: -0.045em;
  line-height: 1.08;
  text-wrap: balance;
}

.subtitle {
  max-width: 520px;
  color: var(--docsy-text-muted);
  font-size: 14px;
  line-height: 1.75;
  margin: 18px 0 0;
}

.hero-art {
  position: relative;
  display: grid;
  place-items: center;
  min-height: 300px;
  background: var(--docsy-primary-soft);
  border-left: 1px solid #cadbd4;
}

.hero-art::before,
.hero-art::after {
  position: absolute;
  content: '';
  border: 1px solid rgba(36, 76, 68, 0.15);
  border-radius: 50%;
}

.hero-art::before {
  width: 230px;
  height: 230px;
}

.hero-art::after {
  width: 280px;
  height: 280px;
  animation: doclet-ring 5s var(--ease-out) infinite;
}

.doclet-stage {
  z-index: 1;
  width: min(240px, 78%);
  animation: doclet-idle 4.8s ease-in-out 700ms infinite;
  transform-origin: 50% 88%;
  transition: filter 220ms var(--ease-out);
}

.hero-art:hover .doclet-stage {
  filter: drop-shadow(0 6px 8px rgba(32, 65, 59, 0.08));
}

.hero-mascot {
  display: block;
  width: 100%;
  max-height: 270px;
  object-fit: contain;
  filter: drop-shadow(0 20px 28px rgba(32, 65, 59, 0.18));
  animation: doclet-settle 700ms var(--ease-out) both;
}

.section {
  margin-top: 34px;
}

.section-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  margin-bottom: 14px;
}

.section-heading h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 720;
  color: var(--docsy-text-strong);
}

.section-heading span {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.cards {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  background: var(--docsy-surface-elevated);
  border-top: 1px solid var(--docsy-border-subtle);
  border-left: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  box-shadow: 0 14px 36px rgba(48, 41, 32, 0.06);
  overflow: hidden;
}

.home-card {
  position: relative;
  display: flex;
  grid-column: span 4;
  flex-direction: column;
  min-height: 184px;
  padding: 24px 24px 22px;
  cursor: pointer;
  text-align: left;
  color: var(--docsy-text-strong);
  border: 0;
  border-right: 1px solid var(--docsy-border-subtle);
  border-bottom: 1px solid var(--docsy-border-subtle);
  border-radius: 0;
  background: var(--docsy-surface-elevated);
  transition:
    color 180ms var(--ease-out),
    background 180ms var(--ease-out),
    transform 140ms var(--ease-out),
    box-shadow 180ms var(--ease-out);
}

.home-card:nth-child(1),
.home-card:nth-child(2) {
  grid-column: span 6;
}

.home-card:hover {
  z-index: 1;
  color: var(--docsy-surface-elevated);
  background: var(--docsy-primary-hover);
  transform: translateY(-2px);
  box-shadow: 0 16px 30px rgba(30, 61, 55, 0.18);
}

.home-card:active {
  transform: translateY(0) scale(0.99);
}

.card-topline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: auto;
}

.home-card-icon {
  display: block;
  width: 64px;
  height: 64px;
  color: var(--docsy-primary);
  background: currentColor;
  mask-image: var(--home-icon-url);
  mask-position: center;
  mask-repeat: no-repeat;
  mask-size: contain;
  -webkit-mask-image: var(--home-icon-url);
  -webkit-mask-position: center;
  -webkit-mask-repeat: no-repeat;
  -webkit-mask-size: contain;
  transition:
    color 180ms var(--ease-out),
    transform 180ms var(--ease-out);
}

.home-card:hover .home-card-icon {
  color: var(--docsy-accent-light);
  transform: translateY(-2px) scale(1.06);
}

.card-copy {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 22px;
}

.card-copy strong {
  font-size: 15px;
  font-weight: 720;
}

.card-arrow {
  font-size: 18px;
  font-weight: 400;
  transition: transform 180ms var(--ease-out);
}

.home-card:hover .card-arrow {
  transform: translateX(4px);
}

.card-description {
  color: var(--docsy-text-muted);
  font-size: 11px;
  line-height: 1.6;
  margin-top: 6px;
}

.home-card:hover .card-description {
  color: rgba(255, 255, 255, 0.62);
}

@keyframes doclet-settle {
  from {
    transform: translateY(16px) scale(0.96);
    opacity: 0;
  }

  to {
    transform: translateY(0) scale(1);
    opacity: 1;
  }
}

@keyframes doclet-idle {
  0%,
  100% {
    transform: translateY(0) rotate(0deg);
  }

  45% {
    transform: translateY(-7px) rotate(-1deg);
  }

  55% {
    transform: translateY(-7px) rotate(1deg);
  }
}

@keyframes doclet-ring {
  0%,
  100% {
    opacity: 0.45;
    transform: scale(0.96);
  }

  50% {
    opacity: 0.9;
    transform: scale(1.02);
  }
}

@media (max-width: 900px) {
  .home-view {
    width: min(100% - 36px, 760px);
    padding-top: 28px;
  }

  .hero {
    grid-template-columns: 1fr 230px;
  }

  .hero-copy {
    padding: 34px;
  }

  .home-card:nth-child(n) {
    grid-column: span 6;
  }
}

@media (max-width: 680px) {
  .home-view {
    width: min(100% - 28px, 560px);
  }

  .hero {
    grid-template-columns: 1fr;
  }

  .hero-art {
    min-height: 210px;
    border-top: 1px solid #cadbd4;
    border-left: 0;
  }

  .home-card:nth-child(n) {
    grid-column: span 12;
  }
}
</style>
