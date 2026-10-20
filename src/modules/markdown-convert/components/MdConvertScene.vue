<template>
  <div
    class="md-convert-scene"
    :class="{
      'is-converting': converting,
      'is-motion-paused': !motionActive,
      'is-dragging': dragging,
      'is-celebrating': celebrating,
    }"
  >
    <!-- Animation Stage -->
    <div
      ref="sceneStageRef"
      class="scene-stage"
      :class="{ 'is-nodding': nodActive }"
      @pointerdown="handlePointerDown"
    >
      <!-- Ambient Glow Behind Stage -->
      <div class="stage-ambient-glow" />

      <!-- Click Ripple Effects -->
      <div
        v-for="r in ripples"
        :key="r.id"
        class="click-ripple"
        :style="{ left: `${r.x}px`, top: `${r.y}px` }"
      >
        <span class="ripple-wave" />
        <span class="ripple-glyph">{{ r.glyph }}</span>
      </div>

      <!-- Left Floating Markdown Card -->
      <div class="doc-card doc-card-md" :class="{ 'card-flying': converting }">
        <div class="card-header">
          <span class="card-badge badge-md">.md</span>
          <div class="card-dots">
            <span class="dot red" /><span class="dot yellow" /><span class="dot green" />
          </div>
        </div>
        <div class="card-body">
          <div class="code-line line-h1" />
          <div class="code-line line-text" />
          <div class="code-line line-formula">
            <span class="math-symbol">∑</span>
            <div class="line-sub" />
          </div>
          <div class="code-line line-code" />
          <div class="code-line line-table">
            <span class="tbl-cell" /><span class="tbl-cell" />
          </div>
        </div>
        <div class="card-corner-fold" />
      </div>

      <!-- Center Transformation Stream / Energy Rings -->
      <div class="transform-portal">
        <svg class="portal-svg" viewBox="0 0 120 40" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path
            d="M 10 20 C 40 5, 80 35, 110 20"
            stroke="currentColor"
            stroke-width="2"
            stroke-dasharray="4 4"
            class="flow-track"
          />
          <circle v-if="converting" r="3" class="flow-particle particle-1 flow-css-dot" />
          <circle v-if="converting" r="2.5" class="flow-particle particle-2 flow-css-dot" />
          <circle v-if="converting" r="2" class="flow-particle particle-3 flow-css-dot" />
        </svg>

        <!-- Dynamic Floating Glyphs during converting -->
        <div v-if="converting" class="floating-glyphs">
          <span class="glyph g1">✨</span>
          <span class="glyph g2">§</span>
          <span class="glyph g3">f(x)</span>
          <span class="glyph g4">✦</span>
        </div>
      </div>

      <!-- Mascot Doclet with Dynamic Motion -->
      <div class="scene-mascot">
        <DocletSprite :interactive="false" :size="112" :motion="mascotMotion" :gaze-tracking="!nodActive" />
      </div>

      <!-- Right Floating Converted Document Card -->
      <div
        class="doc-card doc-card-office"
        :class="[`format-${targetFormat}`, { 'card-received': converting }]"
      >
        <div class="card-header">
          <span class="card-badge" :class="`badge-${targetFormat}`">.{{ targetFormat }}</span>
          <span class="format-icon">{{ formatGlyph }}</span>
        </div>
        <div class="card-body">
          <div class="doc-title-bar" />
          <div class="doc-text-row" />
          <div class="doc-text-row short" />
          <div class="doc-rich-preview">
            <div class="doc-styled-table">
              <div class="tbl-row th"><span /><span /></div>
              <div class="tbl-row td"><span /><span /></div>
            </div>
          </div>
        </div>
        <div class="card-corner-fold" />
      </div>
    </div>

    <!-- Context Text & Action CTA -->
    <div class="scene-meta">
      <template v-if="converting">
        <div class="converting-indicator">
          <span class="pulse-ring" />
          <strong class="converting-title">{{ progressText || 'Doclet 正在排版生成文档…' }}</strong>
        </div>
        <p class="converting-desc">根据输入类型提取文字，或按所选格式生成文件</p>
      </template>
      <template v-else>
        <strong class="scene-title">选择或拖入文档</strong>
        <p class="scene-desc">{{ MARKDOWN_SCENE_DESCRIPTION }}</p>
        <div class="scene-actions">
          <el-button type="primary" plain size="default" class="select-btn" @click="$emit('select-files')">
            添加文件
          </el-button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, onBeforeUnmount } from 'vue'
import { MARKDOWN_SCENE_DESCRIPTION } from '../composables/conversionCapabilities.js'
import { useMotionActivity } from '../../../shared/composables/useMotionActivity.js'
import DocletSprite from '../../../shared/components/DocletSprite.vue'
import { useDocletClickAction } from '../../../shared/composables/useDocletClickAction.js'

const props = defineProps({
  converting: { type: Boolean, default: false },
  dragging: { type: Boolean, default: false },
  celebrating: { type: Boolean, default: false },
  progressText: { type: String, default: '' },
  targetFormat: { type: String, default: 'docx' },
})

defineEmits(['select-files'])

const sceneStageRef = ref(null)
const { active: motionActive, reduced } = useMotionActivity(sceneStageRef)
const interactionTimers = new Set()
function later(callback, delay) {
  const timer = setTimeout(() => { interactionTimers.delete(timer); callback() }, delay)
  interactionTimers.add(timer)
}
onBeforeUnmount(() => interactionTimers.forEach(clearTimeout))
const { motion: clickMotion, play: playClickMotion } = useDocletClickAction()
const nodActive = computed(() => Boolean(clickMotion.value))
const ripples = ref([])

function handlePointerDown(e) {
  if (props.converting || props.celebrating || props.dragging) return
  if (reduced.value || !motionActive.value || e.button !== 0 || ripples.value.length >= 6) return
  if (!sceneStageRef.value) return
  const rect = sceneStageRef.value.getBoundingClientRect()
  const x = Math.round(e.clientX - rect.left)
  const y = Math.round(e.clientY - rect.top)
  const id = Date.now() + Math.random()
  const glyphs = ['✨', '✦', '📝', '📊', '🌐']
  const randomGlyph = glyphs[Math.floor(Math.random() * glyphs.length)]
  ripples.value.push({ id, x, y, glyph: randomGlyph })
  later(() => {
    ripples.value = ripples.value.filter((r) => r.id !== id)
  }, 850)

  playClickMotion()
}

const mascotMotion = computed(() => {
  if (props.celebrating) return 'celebrate'
  if (props.converting) return 'working'
  if (props.dragging) return 'greet'
  if (nodActive.value) return clickMotion.value
  return 'look'
})

const formatGlyph = computed(() => {
  switch (props.targetFormat) {
    case 'xlsx':
      return '📊'
    case 'pptx':
      return '📽️'
    case 'html':
      return '🌐'
    default:
      return '📝'
  }
})
</script>

<style scoped>
.md-convert-scene {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: clamp(16px, 3vh, 32px) 20px;
  user-select: none;
  min-height: 260px;
}

/* ── Stage Layout ── */
.scene-stage {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: clamp(12px, 3vw, 28px);
  width: 100%;
  max-width: 460px;
  margin-bottom: 14px;
  perspective: 750px;
  cursor: pointer;
}

/* ── Click Ripple & Floating Glyphs ── */
.click-ripple {
  position: absolute;
  pointer-events: none;
  z-index: 20;
  transform: translate(-50%, -50%);
}

.ripple-wave {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 10px;
  height: 10px;
  margin-top: -5px;
  margin-left: -5px;
  border-radius: 50%;
  border: 2px solid var(--docsy-primary);
  opacity: 0.85;
  animation: ripple-grow 0.75s cubic-bezier(0.1, 0.8, 0.3, 1) forwards;
}

@keyframes ripple-grow {
  to {
    width: 68px;
    height: 68px;
    margin-top: -34px;
    margin-left: -34px;
    opacity: 0;
    transform: scale(1.4);
  }
}

.ripple-glyph {
  position: absolute;
  top: -8px;
  left: 0;
  font-size: 15px;
  user-select: none;
  animation: ripple-float-up 0.8s cubic-bezier(0.2, 0.8, 0.3, 1) forwards;
}

@keyframes ripple-float-up {
  0% { transform: translate(-50%, 0) scale(0.6); opacity: 0; }
  25% { transform: translate(-50%, -10px) scale(1.2); opacity: 1; }
  100% { transform: translate(-50%, -32px) scale(0.8); opacity: 0; }
}

.stage-ambient-glow {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 280px;
  height: 120px;
  transform: translate(-50%, -50%);
  border-radius: 50%;
  background: radial-gradient(
    ellipse at center,
    color-mix(in srgb, var(--docsy-primary) 12%, transparent) 0%,
    transparent 70%
  );
  pointer-events: none;
  filter: blur(16px);
  transition: all 0.4s ease;
}

.md-convert-scene.is-dragging .stage-ambient-glow {
  width: 360px;
  height: 160px;
  background: radial-gradient(
    ellipse at center,
    color-mix(in srgb, var(--docsy-primary) 22%, transparent) 0%,
    transparent 70%
  );
}

/* ── Mascot Container ── */
.scene-mascot {
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  align-items: center;
  transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px), 0) rotate(calc(var(--look-x, 0) * 6deg));
  transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.4, 1);
}

.is-nodding .scene-mascot {
  animation: doclet-nod 0.38s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes doclet-nod {
  0%, 100% { transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px), 0) scale(1); }
  45% { transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px - 9px), 0) scale(1.08) rotate(calc(var(--look-x, 0) * 8deg)); }
}

.mascot-shadow {
  width: 72px;
  height: 10px;
  margin-top: -6px;
  border-radius: 50%;
  background: radial-gradient(ellipse at center, rgba(0, 0, 0, 0.14) 0%, transparent 75%);
  animation: shadow-breathe 3s ease-in-out infinite;
}

/* ── Floating Document Cards ── */
.doc-card {
  position: relative;
  z-index: 1;
  width: 82px;
  height: 104px;
  padding: 8px 9px;
  border-radius: 8px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.06), 0 1px 3px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1), box-shadow 0.4s ease;
}

.doc-card-md {
  transform: translate3d(calc(var(--look-x, 0) * 4px), calc(var(--look-y, 0) * 3px), 0) rotateY(calc(var(--look-x, 0) * 10deg)) rotateX(calc(var(--look-y, 0) * -6deg)) rotate(-5deg);
  animation: card-float-left 4.2s ease-in-out infinite;
  background: linear-gradient(175deg, var(--docsy-surface-elevated) 60%, var(--docsy-surface-muted));
  transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.4, 1), box-shadow 0.3s ease;
}

.doc-card-office {
  transform: translate3d(calc(var(--look-x, 0) * 4px), calc(var(--look-y, 0) * 3px), 0) rotateY(calc(var(--look-x, 0) * 10deg)) rotateX(calc(var(--look-y, 0) * -6deg)) rotate(5deg);
  animation: card-float-right 4.5s ease-in-out infinite;
  background: linear-gradient(175deg, var(--docsy-surface-elevated) 60%, var(--docsy-surface-muted));
  transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.4, 1), box-shadow 0.3s ease;
}

.md-convert-scene.is-dragging .doc-card-md {
  transform: translateY(-8px) rotate(-10deg) scale(1.05);
}

.md-convert-scene.is-dragging .doc-card-office {
  transform: translateY(-8px) rotate(10deg) scale(1.05);
}

/* Corner fold effect */
.card-corner-fold {
  position: absolute;
  top: 0;
  right: 0;
  width: 0;
  height: 0;
  border-style: solid;
  border-width: 0 12px 12px 0;
  border-color: transparent var(--docsy-surface-muted) transparent transparent;
  box-shadow: -1px 1px 2px rgba(0, 0, 0, 0.05);
}

/* Card Header & Badges */
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.card-badge {
  font-size: 9.5px;
  font-weight: 700;
  font-family: var(--docsy-font-mono, monospace);
  padding: 1px 4px;
  border-radius: 4px;
  letter-spacing: -0.2px;
}

.badge-md {
  background: #3b82f618;
  color: #2563eb;
  border: 1px solid #3b82f630;
}

.badge-docx {
  background: #2563eb18;
  color: #1d4ed8;
  border: 1px solid #2563eb30;
}

.badge-xlsx {
  background: #05966918;
  color: #047857;
  border: 1px solid #05966930;
}

.badge-pptx {
  background: #ea580c18;
  color: #c2410c;
  border: 1px solid #ea580c30;
}

.badge-html {
  background: #7c3aed18;
  color: #6d28d9;
  border: 1px solid #7c3aed30;
}

.card-dots {
  display: flex;
  gap: 2px;
}

.card-dots .dot {
  width: 3.5px;
  height: 3.5px;
  border-radius: 50%;
}

.dot.red { background: #ef4444; }
.dot.yellow { background: #f59e0b; }
.dot.green { background: #10b981; }

.format-icon {
  font-size: 11px;
}

/* Card Body Content Lines */
.card-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.code-line {
  height: 3px;
  border-radius: 1.5px;
  background: var(--docsy-border-strong);
  opacity: 0.7;
}

.line-h1 {
  width: 55%;
  height: 4.5px;
  background: var(--docsy-primary);
  opacity: 0.9;
}

.line-text { width: 85%; }
.line-code { width: 70%; background: #64748b; }

.line-formula {
  display: flex;
  align-items: center;
  gap: 3px;
  background: transparent;
  height: auto;
}

.math-symbol {
  font-size: 8px;
  font-weight: 700;
  color: #8b5cf6;
  line-height: 1;
}

.line-sub {
  height: 3px;
  width: 60%;
  border-radius: 1.5px;
  background: #8b5cf680;
}

.line-table {
  display: flex;
  gap: 2px;
  height: auto;
  background: transparent;
}

.tbl-cell {
  flex: 1;
  height: 4px;
  border-radius: 1px;
  background: var(--docsy-border-subtle);
  border: 0.5px solid var(--docsy-border-strong);
}

/* Office Card Styled Content */
.doc-title-bar {
  width: 65%;
  height: 5px;
  border-radius: 2px;
  background: #1e40af;
  margin-bottom: 2px;
}

.format-xlsx .doc-title-bar { background: #065f46; }
.format-pptx .doc-title-bar { background: #9a3412; }
.format-html .doc-title-bar { background: #5b21b6; }

.doc-text-row {
  width: 90%;
  height: 3px;
  border-radius: 1.5px;
  background: var(--docsy-border-strong);
  opacity: 0.75;
}

.doc-text-row.short { width: 60%; }

.doc-rich-preview {
  margin-top: 3px;
  border-radius: 3px;
  padding: 2px;
  background: var(--docsy-surface-muted);
}

.doc-styled-table {
  display: flex;
  flex-direction: column;
  gap: 1.5px;
}

.tbl-row {
  display: flex;
  gap: 2px;
}

.tbl-row.th span {
  flex: 1;
  height: 3px;
  border-radius: 1px;
  background: #93c5fd;
}

.format-xlsx .tbl-row.th span { background: #6ee7b7; }
.format-pptx .tbl-row.th span { background: #fdba74; }
.format-html .tbl-row.th span { background: #c4b5fd; }

.tbl-row.td span {
  flex: 1;
  height: 3px;
  border-radius: 1px;
  background: var(--docsy-border-subtle);
}

/* ── Central Particle Stream ── */
.transform-portal {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 140px;
  height: 40px;
  transform: translate(-50%, -50%);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 1;
}

.portal-svg {
  width: 100%;
  height: 100%;
  color: var(--docsy-primary);
  opacity: 0.35;
}

.flow-particle {
  fill: var(--docsy-primary);
}

.is-converting .portal-svg {
  opacity: 0.8;
}

.floating-glyphs {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.glyph {
  position: absolute;
  font-size: 11px;
  font-weight: 700;
  color: var(--docsy-primary);
  animation: glyph-pulse 1.6s ease-in-out infinite alternate;
}

.g1 { top: -8px; left: 20%; animation-delay: 0s; }
.g2 { bottom: -6px; left: 45%; animation-delay: 0.4s; color: #8b5cf6; }
.g3 { top: -10px; right: 25%; animation-delay: 0.8s; font-size: 9px; color: #10b981; }
.g4 { bottom: -8px; right: 15%; animation-delay: 1.2s; color: #f59e0b; }

/* ── Scene Meta Text & Actions ── */
.scene-meta {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 380px;
}

.scene-title {
  color: var(--docsy-text-strong);
  font-size: 15px;
  font-weight: 700;
  margin-bottom: 4px;
}

.scene-desc {
  color: var(--docsy-text-muted);
  font-size: 12.5px;
  line-height: 1.5;
  margin: 0 0 12px;
}

.select-btn {
  font-weight: 600;
  padding: 8px 20px;
  border-radius: var(--docsy-radius);
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.select-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--docsy-primary) 20%, transparent);
}

/* Converting Indicator */
.converting-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.pulse-ring {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--docsy-primary);
  box-shadow: 0 0 0 0 var(--docsy-primary);
  animation: pulse-ring-anim 1.4s infinite;
}

.converting-title {
  color: var(--docsy-primary);
  font-size: 14.5px;
  font-weight: 700;
}

.converting-desc {
  color: var(--docsy-text-muted);
  font-size: 12px;
  margin: 0;
}

/* ── Keyframes ── */
@keyframes card-float-left {
  0%, 100% {
    transform: translateY(0) rotate(-5deg);
  }
  50% {
    transform: translateY(-7px) rotate(-3deg);
  }
}

@keyframes card-float-right {
  0%, 100% {
    transform: translateY(0) rotate(5deg);
  }
  50% {
    transform: translateY(-9px) rotate(7deg);
  }
}

@keyframes shadow-breathe {
  0%, 100% {
    transform: scale(1);
    opacity: 0.8;
  }
  50% {
    transform: scale(0.9);
    opacity: 0.5;
  }
}

@keyframes glyph-pulse {
  0% {
    transform: translateY(0) scale(0.85);
    opacity: 0.3;
  }
  100% {
    transform: translateY(-6px) scale(1.15);
    opacity: 1;
  }
}

@keyframes pulse-ring-anim {
  0% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--docsy-primary) 70%, transparent);
  }
  70% {
    transform: scale(1);
    box-shadow: 0 0 0 8px transparent;
  }
  100% {
    transform: scale(0.95);
    box-shadow: 0 0 0 0 transparent;
  }
}
</style>

<style scoped>
.doc-card-md, .doc-card-office { animation-iteration-count: 1; }
.flow-css-dot { animation: docsy-stream-dot 2.2s linear infinite; }
.flow-css-dot:nth-of-type(2) { animation-delay: -.73s; }
.flow-css-dot:nth-of-type(3) { animation-delay: -1.46s; }
@keyframes docsy-stream-dot {
  0% { transform: translate(10px, 20px); opacity: 0; }
  15% { opacity: 1; }
  35% { transform: translate(42px, 17px); }
  65% { transform: translate(78px, 23px); }
  85% { opacity: 1; }
  100% { transform: translate(110px, 20px); opacity: 0; }
}
.md-convert-scene:not(.is-converting) :is(.stream-flow-line, .flow-track) { animation: none; }
.md-convert-scene.is-dragging .doc-card-md { animation: none; transform: translateY(-8px) rotate(-6deg) scale(1.04); }
.md-convert-scene.is-dragging .doc-card-office { animation: none; transform: translateY(-4px) rotate(5deg); }
.md-convert-scene.is-celebrating .doc-card-office { animation: docsy-result-arrive 460ms cubic-bezier(.2,.9,.3,1.2) 1; }
@keyframes docsy-result-arrive { from { opacity: .4; translate: -8px 4px; scale: .94; } to { opacity: 1; translate: 0 0; scale: 1; } }
@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation: none !important; transition: none !important; } }
</style>
