<template>
  <div
    class="module-scene-stage"
    :class="[
      `scene--${resolvedScene}`,
      {
        'is-processing': busy || processing,
        'is-motion-paused': !motionActive,
        'is-dragging': dragging,
        'is-celebrating': celebrating,
      },
    ]"
    role="region"
    :aria-label="title || '工作区空态场景'"
  >
    <!-- Animation Canvas Stage -->
    <div
      ref="stageCanvasRef"
      class="stage-canvas"
      :class="{ 'is-nodding': nodActive }"
      aria-hidden="true"
      @pointerdown="handlePointerDown"
    >
      <!-- Ambient Glow Behind Stage -->
      <div class="stage-glow" />

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

      <!-- Left Source Card -->
      <div class="scene-card source-card" :class="{ 'card-flying': busy || processing }">
        <div class="card-badge-row">
          <span class="card-badge">{{ sceneMeta.sourceBadge }}</span>
          <span class="card-sub-icon">{{ sceneMeta.sourceIcon }}</span>
        </div>
        <div class="card-content-preview">
          <SceneIllustrations :scene="resolvedScene" type="source" :active="busy || processing" />
        </div>
        <div class="card-dogear" />
      </div>

      <!-- Center Transformation Stream & Portal -->
      <div class="stage-portal">
        <svg class="stream-svg" viewBox="0 0 120 40" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path
            d="M 10 20 C 40 6, 80 34, 110 20"
            stroke="currentColor"
            stroke-width="2"
            stroke-dasharray="4 4"
            class="stream-flow-line"
          />
          <circle v-if="busy || processing" r="3" class="stream-dot dot-1 flow-css-dot" />
          <circle v-if="busy || processing" r="2.4" class="stream-dot dot-2 flow-css-dot" />
          <circle v-if="busy || processing" r="2" class="stream-dot dot-3 flow-css-dot" />
        </svg>

        <!-- Floating glyphs when processing -->
        <div v-if="busy || processing" class="portal-glyphs">
          <span
            v-for="(glyph, idx) in sceneMeta.glyphs"
            :key="idx"
            class="glyph-item"
            :class="`g-${idx + 1}`"
          >
            {{ glyph }}
          </span>
        </div>
      </div>

      <!-- Doclet Mascot in Center -->
      <div class="stage-mascot">
        <DocletSprite :interactive="false" :size="112" :motion="mascotMotion" :gaze-tracking="!nodActive" />
      </div>

      <!-- Right Target Result Card -->
      <div class="scene-card target-card" :class="{ 'card-received': busy || processing }">
        <div class="card-badge-row">
          <span class="card-badge badge-target">{{ sceneMeta.targetBadge }}</span>
          <span class="card-sub-icon">{{ sceneMeta.targetIcon }}</span>
        </div>
        <div class="card-content-preview">
          <SceneIllustrations :scene="resolvedScene" type="target" :active="busy || processing" />
        </div>
        <div class="card-dogear" />
      </div>
    </div>

    <!-- Descriptive Text & Main Action CTA -->
    <div class="stage-meta">
      <template v-if="busy || processing">
        <div class="meta-busy-row">
          <span class="pulse-spark" />
          <strong class="meta-busy-title">{{ progressText || sceneMeta.busyTitle }}</strong>
        </div>
        <p class="meta-desc">{{ sceneMeta.busyDesc }}</p>
      </template>
      <template v-else>
        <strong class="meta-title">{{ title || sceneMeta.defaultTitle }}</strong>
        <p class="meta-desc">{{ description || sceneMeta.defaultDesc }}</p>
        <div v-if="showAction || $slots.actions" class="meta-actions">
          <slot name="actions">
            <el-button
              v-if="actionLabel || sceneMeta.actionLabel"
              type="primary"
              plain
              size="default"
              class="stage-cta-btn"
              @click="$emit('action')"
            >
              {{ actionLabel || sceneMeta.actionLabel }}
            </el-button>
          </slot>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, onBeforeUnmount } from 'vue'
import { useMotionActivity } from '../composables/useMotionActivity.js'
import DocletSprite from './DocletSprite.vue'
import { useDocletClickAction } from '../composables/useDocletClickAction.js'
import SceneIllustrations from './SceneIllustrations.vue'

const props = defineProps({
  scene: {
    type: String,
    default: 'generic',
    validator: (v) =>
      [
        'generic',
        'pdf-unlock',
        'pdf-merge',
        'pdf-compress',
        'pdf-anticopy',
        'pdf-extract',
        'evidence',
        'template',
        'image-paddler',
        'video-extract',
        'markdown',
      ].includes(v),
  },
  busy: { type: Boolean, default: false },
  processing: { type: Boolean, default: false },
  dragging: { type: Boolean, default: false },
  celebrating: { type: Boolean, default: false },
  progressText: { type: String, default: '' },
  title: { type: String, default: '' },
  description: { type: String, default: '' },
  actionLabel: { type: String, default: '' },
  showAction: { type: Boolean, default: true },
})

defineEmits(['action'])

const resolvedScene = computed(() => props.scene || 'generic')

const stageCanvasRef = ref(null)
const { active: motionActive, reduced } = useMotionActivity(stageCanvasRef)
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
  if (props.busy || props.processing || props.celebrating || props.dragging) return
  if (reduced.value || !motionActive.value || e.button !== 0 || ripples.value.length >= 6) return
  if (!stageCanvasRef.value) return
  const rect = stageCanvasRef.value.getBoundingClientRect()
  const x = Math.round(e.clientX - rect.left)
  const y = Math.round(e.clientY - rect.top)
  const id = Date.now() + Math.random()
  const glyphs = sceneMeta.value?.glyphs?.length ? sceneMeta.value.glyphs : ['✨', '✦', '📑', '🎉', '💫']
  const randomGlyph = glyphs[Math.floor(Math.random() * glyphs.length)]
  ripples.value.push({ id, x, y, glyph: randomGlyph })
  later(() => {
    ripples.value = ripples.value.filter((r) => r.id !== id)
  }, 850)

  // 触发 Doclet 快乐点头回应
  playClickMotion()
}

const mascotMotion = computed(() => {
  if (props.celebrating) return 'celebrate'
  if (props.busy || props.processing) return ['pdf-merge', 'pdf-compress', 'template', 'image-paddler'].includes(resolvedScene.value) ? 'working' : 'review'
  if (props.dragging) return 'greet'
  if (nodActive.value) return clickMotion.value
  return 'look'
})

const SCENE_CONFIGS = {
  'pdf-unlock': {
    sourceBadge: '.pdf 🔒',
    sourceIcon: '🔑',
    targetBadge: '.pdf 🔓',
    targetIcon: '✨',
    glyphs: ['🔑', '✨', '🔓', '✦'],
    busyTitle: '正在分析解除安全限制…',
    busyDesc: '移除打印、复制或编辑限制，生成免密可用 PDF',
    defaultTitle: '选择或拖入加密 PDF',
    defaultDesc: '批量解除密码保护与打印限制，保持原文档版式与矢量清晰度',
    actionLabel: '选择加密文件',
  },
  'pdf-merge': {
    sourceBadge: '多份文档',
    sourceIcon: '📄',
    targetBadge: '合订 PDF',
    targetIcon: '📑',
    glyphs: ['📎', '📑', '🔗', '✦'],
    busyTitle: 'Doclet 正在装订合并文档…',
    busyDesc: '按所选顺序合并文档，并应用已设置的合并选项',
    defaultTitle: '添加需要合并的文档',
    defaultDesc: '按顺序合并 PDF；Word 文件需先转换为 PDF',
    actionLabel: '添加文件',
  },
  'pdf-compress': {
    sourceBadge: '原始 PDF',
    sourceIcon: '📦',
    targetBadge: '瘦身 PDF',
    targetIcon: '🍃',
    glyphs: ['⚡', '📉', '🍃', '✦'],
    busyTitle: 'Doclet 正在智能压缩文档…',
    busyDesc: '重采样高清图片、精简冗余对象并保留文字矢量精度',
    defaultTitle: '拖入大体积 PDF 文件',
    defaultDesc: '按所选参数压缩 PDF；体积变化和图片清晰度取决于原文件与压缩设置',
    actionLabel: '添加文件',
  },
  'pdf-anticopy': {
    sourceBadge: '普通 PDF',
    sourceIcon: '📄',
    targetBadge: '防护 PDF',
    targetIcon: '🛡️',
    glyphs: ['🛡️', '🔒', '✨', '✦'],
    busyTitle: 'Doclet 正在注入安全防复制层…',
    busyDesc: '按所选模式处理文本，降低直接复制或提取文字的便利性',
    defaultTitle: '添加需保护的 PDF 文档',
    defaultDesc: '按所选模式限制直接复制；无法保证阻止截图或 OCR，处理后请核对阅读效果',
    actionLabel: '添加文件',
  },
  'pdf-extract': {
    sourceBadge: '源 PDF',
    sourceIcon: '📑',
    targetBadge: '目标页面',
    targetIcon: '🎯',
    glyphs: ['✂️', '🎯', '📄', '✦'],
    busyTitle: 'Doclet 正在定向提取页面…',
    busyDesc: '按指定页码范围生成新的 PDF 文件',
    defaultTitle: '选择需要提取的 PDF',
    defaultDesc: '支持灵活输入单页或连续页码范围（如 3, 7, 12-15），独立快速导出',
    actionLabel: '选择 PDF 文件',
  },
  evidence: {
    sourceBadge: '证据材料',
    sourceIcon: '📂',
    targetBadge: '证据册',
    targetIcon: '⚖️',
    glyphs: ['⚖️', '📜', '🔖', '✦'],
    busyTitle: 'Doclet 正在处理证据页段…',
    busyDesc: '按所选页段、纸张及页眉页脚规则处理证据材料',
    defaultTitle: '导入证据 PDF 或材料',
    defaultDesc: '导入材料后划分页段，设置纸张、页码和页眉页脚，再导出 PDF',
    actionLabel: '选择证据文件',
  },
  template: {
    sourceBadge: 'Word 蓝图',
    sourceIcon: '📝',
    targetBadge: '生成文书',
    targetIcon: '⚡',
    glyphs: ['📝', '⚡', '{ }', '✦'],
    busyTitle: 'Doclet 正在装配文书模板…',
    busyDesc: '智能解析标黄占位符，匹配数据自动批量生成',
    defaultTitle: '导入 Word 模板文件',
    defaultDesc: '把需要替换的文字或勾选符号标黄后导入，Docsy 自动识别为模板字段',
    actionLabel: '选择 Word 模板',
  },
  'image-paddler': {
    sourceBadge: '原始照片',
    sourceIcon: '📷',
    targetBadge: '画框相片',
    targetIcon: '🖼️',
    glyphs: ['🖼️', '✂️', '✨', '✦'],
    busyTitle: 'Doclet 正在调整白边与画框…',
    busyDesc: '计算对齐尺寸、应用精工留白与专业相框边距',
    defaultTitle: '添加待排版图片',
    defaultDesc: '批量添加图片，为相片增加考究白边与统一尺寸，提升视觉格调',
    actionLabel: '选择图片',
  },
  'video-extract': {
    sourceBadge: '源视频',
    sourceIcon: '🎬',
    targetBadge: '帧图片',
    targetIcon: '📸',
    glyphs: ['🎬', '⏱️', '⚡', '✦'],
    busyTitle: 'Doclet 正在扫描视频时间轴…',
    busyDesc: '按设置的频率或时间间隔提取视频帧',
    defaultTitle: '选择或拖入视频文件',
    defaultDesc: '设置时间范围、抽帧频率或间隔，批量导出图片',
    actionLabel: '选择视频',
  },
  generic: {
    sourceBadge: '待处理文件',
    sourceIcon: '📄',
    targetBadge: '处理结果',
    targetIcon: '✨',
    glyphs: ['✨', '⚡', '📄', '✦'],
    busyTitle: 'Doclet 正在处理中…',
    busyDesc: '正在执行文档操作，请稍候',
    defaultTitle: '选择或拖入文件',
    defaultDesc: '将文件拖拽至此区域或点击下方按钮添加',
    actionLabel: '添加文件',
  },
}

const sceneMeta = computed(() => SCENE_CONFIGS[resolvedScene.value] || SCENE_CONFIGS.generic)
</script>

<style scoped>
.module-scene-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  min-height: 280px;
  padding: 28px 20px;
  box-sizing: border-box;
  text-align: center;
  user-select: none;
  position: relative;
  overflow: hidden;
}

/* ── Animation Stage ── */
.stage-canvas {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  width: 100%;
  max-width: 480px;
  margin-bottom: 20px;
  perspective: 750px;
  cursor: pointer;
}

/* ── Click Ripple & Particle Effects ── */
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

.stage-glow {
  position: absolute;
  inset: -16px 20px;
  border-radius: 50%;
  background: radial-gradient(
    ellipse at center,
    color-mix(in srgb, var(--docsy-primary) 14%, transparent) 0%,
    transparent 72%
  );
  filter: blur(24px);
  pointer-events: none;
  z-index: 0;
  transition: background 0.4s ease;
}

.is-processing .stage-glow {
  background: radial-gradient(
    ellipse at center,
    color-mix(in srgb, var(--docsy-primary) 28%, transparent) 0%,
    transparent 72%
  );
}

/* ── Floating Document Cards ── */
.scene-card {
  position: relative;
  z-index: 2;
  width: 90px;
  height: 114px;
  padding: 8px 9px;
  box-sizing: border-box;
  background: var(--docsy-surface-elevated, #fff);
  border: 1px solid var(--docsy-border-subtle, rgba(0, 0, 0, 0.08));
  border-radius: 8px;
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.06), 0 1px 3px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.3s ease;
  flex-shrink: 0;
}

.source-card {
  transform: translate3d(calc(var(--look-x, 0) * 4px), calc(var(--look-y, 0) * 3px), 0) rotateY(calc(var(--look-x, 0) * 10deg)) rotateX(calc(var(--look-y, 0) * -6deg)) rotate(-3.5deg);
  animation: float-source 4.2s ease-in-out infinite;
  transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.4, 1), box-shadow 0.3s ease;
}

.target-card {
  transform: translate3d(calc(var(--look-x, 0) * 4px), calc(var(--look-y, 0) * 3px), 0) rotateY(calc(var(--look-x, 0) * 10deg)) rotateX(calc(var(--look-y, 0) * -6deg)) rotate(3.5deg);
  animation: float-target 4.6s ease-in-out infinite 0.5s;
  transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.4, 1), box-shadow 0.3s ease;
}

.is-dragging .source-card {
  transform: translateY(-8px) scale(1.05) rotate(-6deg);
  box-shadow: 0 12px 28px color-mix(in srgb, var(--docsy-primary) 20%, transparent);
}

.is-dragging .target-card {
  transform: translateY(-6px) scale(1.04) rotate(6deg);
}

.card-flying {
  animation: fly-source 1.8s ease-in-out infinite alternate !important;
}

.card-received {
  animation: pulse-target 1.8s ease-in-out infinite alternate 0.9s !important;
}

@keyframes float-source {
  0%, 100% { transform: translateY(0) rotate(-3.5deg); }
  50% { transform: translateY(-7px) rotate(-1.5deg); }
}

@keyframes float-target {
  0%, 100% { transform: translateY(0) rotate(3.5deg); }
  50% { transform: translateY(-6px) rotate(5deg); }
}

@keyframes fly-source {
  0% { transform: translateY(0) rotate(-3.5deg) scale(1); }
  100% { transform: translateY(-9px) rotate(1deg) scale(1.04); }
}

@keyframes pulse-target {
  0% { transform: translateY(0) rotate(3.5deg) scale(1); }
  100% { transform: translateY(-8px) rotate(-1deg) scale(1.05); }
}

/* ── Card Header ── */
.card-badge-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.card-badge {
  font-size: 10px;
  font-weight: 700;
  color: var(--docsy-text-strong, #1f2d26);
  letter-spacing: -0.2px;
  white-space: nowrap;
}

.badge-target {
  color: var(--docsy-primary);
}

.card-sub-icon {
  font-size: 11px;
}

.card-dogear {
  position: absolute;
  top: 0;
  right: 0;
  width: 14px;
  height: 14px;
  background: linear-gradient(135deg, transparent 50%, color-mix(in srgb, var(--docsy-text) 12%, transparent) 50%);
  border-bottom-left-radius: 4px;
  pointer-events: none;
}

/* ── Card Interior Lines ── */
.card-content-preview {
  display: flex;
  flex-direction: column;
  gap: 5px;
  flex: 1;
}

.preview-line {
  height: 4px;
  border-radius: 2px;
  background: var(--docsy-border-subtle, rgba(0, 0, 0, 0.08));
}

.preview-line.wide { width: 100%; }
.preview-line.medium { width: 75%; }
.preview-line.short { width: 50%; }

/* Scene-specific Card Interiors */
.lock-shield-badge {
  font-size: 14px;
  margin: 3px 0;
}

.unlocked-badge {
  font-size: 8.5px;
  font-weight: 600;
  color: #10b981;
  background: color-mix(in srgb, #10b981 12%, transparent);
  padding: 2px 4px;
  border-radius: 3px;
  white-space: nowrap;
}

.file-size-tag {
  font-size: 8.5px;
  font-weight: 600;
  padding: 2px 4px;
  border-radius: 3px;
  white-space: nowrap;
}

.tag-heavy {
  color: #d97706;
  background: color-mix(in srgb, #d97706 12%, transparent);
}

.tag-slim {
  color: #059669;
  background: color-mix(in srgb, #059669 12%, transparent);
}

.copy-warning-badge {
  font-size: 8.5px;
  color: #ef4444;
  white-space: nowrap;
}

.shield-watermark {
  font-size: 15px;
  margin: 2px auto;
}

.secured-tag {
  font-size: 8.5px;
  font-weight: 600;
  color: var(--docsy-primary);
  background: color-mix(in srgb, var(--docsy-primary) 12%, transparent);
  padding: 2px 4px;
  border-radius: 3px;
  white-space: nowrap;
}

.bound-tag {
  font-size: 8.5px;
  font-weight: 600;
  color: var(--docsy-primary);
  margin-top: 2px;
}

.page-corner-tags {
  display: flex;
  gap: 3px;
  font-size: 8px;
  color: var(--docsy-text-muted);
}

.page-corner-tags .highlight {
  color: var(--docsy-primary);
  font-weight: 700;
}

.extracted-page-tag {
  font-size: 8.5px;
  font-weight: 700;
  color: var(--docsy-primary);
  background: color-mix(in srgb, var(--docsy-primary) 12%, transparent);
  padding: 2px 4px;
  border-radius: 3px;
}

.evidence-folder-tab {
  font-size: 8.5px;
  font-weight: 700;
  color: #b45309;
  background: color-mix(in srgb, #b45309 12%, transparent);
  padding: 1px 4px;
  border-radius: 2px;
}

.red-seal {
  width: 22px;
  height: 22px;
  margin: 1px auto;
  border: 1.5px solid #dc2626;
  border-radius: 50%;
  color: #dc2626;
  font-size: 7px;
  display: grid;
  place-items: center;
  font-weight: 800;
  letter-spacing: -0.5px;
}

.evidence-header-tag, .evidence-footer-tag {
  font-size: 7px;
  color: var(--docsy-text-muted);
}

.template-highlight-box {
  background: #fef08a;
  color: #854d0e;
  font-size: 8.5px;
  padding: 1px 3px;
  border-radius: 2px;
  font-family: var(--docsy-font-mono, monospace);
  width: fit-content;
}

.template-filled-line {
  font-size: 7.5px;
  font-weight: 600;
  color: var(--docsy-text-strong);
}

.polaroid-inner {
  height: 52px;
  background: #f1f5f9;
  border-radius: 4px;
  display: grid;
  place-items: center;
  font-size: 16px;
}

.framed-photo-preview {
  height: 56px;
  border: 3px solid #e2e8f0;
  box-shadow: inset 0 0 4px rgba(0,0,0,0.06);
  border-radius: 3px;
  display: grid;
  place-items: center;
}

.photo-inner-canvas {
  width: 80%;
  height: 80%;
  background: linear-gradient(135deg, #a7f3d0, #38bdf8);
  border-radius: 2px;
}

.film-strip-preview {
  display: flex;
  gap: 4px;
  padding: 4px 0;
  background: #1e293b;
  border-radius: 3px;
  justify-content: center;
}

.film-hole {
  width: 6px;
  height: 6px;
  background: #fff;
  border-radius: 1px;
}

.snapshot-frame {
  height: 48px;
  background: #0f172a;
  border-radius: 4px;
  display: grid;
  place-items: center;
}

.timestamp-pill {
  font-size: 7px;
  color: #38bdf8;
  font-family: var(--docsy-font-mono, monospace);
}

/* ── Center Transformation Portal ── */
.stage-portal {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 140px;
  height: 50px;
  pointer-events: none;
  z-index: 1;
}

.stream-svg {
  width: 100%;
  height: 100%;
  color: color-mix(in srgb, var(--docsy-primary) 32%, transparent);
}

.stream-flow-line {
  stroke-dashoffset: 0;
  animation: stream-dash 1.6s linear infinite;
}

@keyframes stream-dash {
  to { stroke-dashoffset: -16; }
}

.stream-dot {
  fill: var(--docsy-primary);
  filter: drop-shadow(0 0 3px var(--docsy-primary));
}

.portal-glyphs {
  position: absolute;
  inset: 0;
}

.glyph-item {
  position: absolute;
  font-size: 13px;
  animation: glyph-fly 1.8s cubic-bezier(0.2, 0.8, 0.4, 1) infinite;
  opacity: 0;
}

.glyph-item.g-1 { top: -4px; left: 24%; animation-delay: 0s; }
.glyph-item.g-2 { top: 28px; left: 40%; animation-delay: 0.45s; }
.glyph-item.g-3 { top: -8px; left: 60%; animation-delay: 0.9s; }
.glyph-item.g-4 { top: 24px; left: 74%; animation-delay: 1.35s; }

@keyframes glyph-fly {
  0% { transform: translate(0, 8px) scale(0.6); opacity: 0; }
  35% { opacity: 0.95; transform: translate(6px, -6px) scale(1.1); }
  75% { opacity: 0.8; transform: translate(14px, -14px) scale(1); }
  100% { opacity: 0; transform: translate(22px, -22px) scale(0.7); }
}

/* ── Mascot Container ── */
.stage-mascot {
  position: relative;
  z-index: 3;
  display: flex;
  flex-direction: column;
  align-items: center;
  transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px), 0) rotate(calc(var(--look-x, 0) * 6deg));
  transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.4, 1);
}

.is-nodding .stage-mascot {
  animation: doclet-nod 0.38s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes doclet-nod {
  0%, 100% { transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px), 0) scale(1); }
  45% { transform: translate3d(calc(var(--look-x, 0) * 8px), calc(var(--look-y, 0) * 5px - 9px), 0) scale(1.08) rotate(calc(var(--look-x, 0) * 8deg)); }
}

.mascot-ground-shadow {
  width: 60px;
  height: 10px;
  margin-top: -6px;
  background: radial-gradient(ellipse at center, rgba(0, 0, 0, 0.14) 0%, transparent 72%);
  border-radius: 50%;
  animation: shadow-breathe 2.4s ease-in-out infinite;
}

@keyframes shadow-breathe {
  0%, 100% { transform: scale(1); opacity: 0.8; }
  50% { transform: scale(0.85); opacity: 0.5; }
}

/* ── Descriptive Text & CTA ── */
.stage-meta {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 440px;
  z-index: 2;
}

.meta-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--docsy-text-strong, #1f2d26);
  margin-bottom: 6px;
  letter-spacing: -0.2px;
}

.meta-desc {
  margin: 0;
  font-size: 13px;
  line-height: 1.55;
  color: var(--docsy-text-muted, #5b6862);
  max-width: 400px;
}

.meta-actions {
  margin-top: 14px;
}

.stage-cta-btn {
  padding: 8px 22px;
  font-weight: 600;
  border-radius: var(--docsy-radius);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--docsy-primary) 18%, transparent);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.stage-cta-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--docsy-primary) 28%, transparent);
}

/* ── Busy Progress Row ── */
.meta-busy-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.pulse-spark {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--docsy-primary);
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--docsy-primary) 60%, transparent);
  animation: spark-pulse 1.4s infinite;
}

@keyframes spark-pulse {
  0% { transform: scale(0.95); box-shadow: 0 0 0 0 color-mix(in srgb, var(--docsy-primary) 70%, transparent); }
  70% { transform: scale(1.1); box-shadow: 0 0 0 8px color-mix(in srgb, var(--docsy-primary) 0%, transparent); }
  100% { transform: scale(0.95); box-shadow: 0 0 0 0 color-mix(in srgb, var(--docsy-primary) 0%, transparent); }
}

.meta-busy-title {
  font-size: 14.5px;
  font-weight: 700;
  color: var(--docsy-primary);
}
</style>

<style scoped>
.source-card, .target-card { animation-iteration-count: 1; }
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
.module-scene-stage:not(.is-processing) :is(.stream-flow-line, .flow-track) { animation: none; }
.module-scene-stage.is-dragging .source-card { animation: none; transform: translateY(-8px) rotate(-6deg) scale(1.04); }
.module-scene-stage.is-dragging .target-card { animation: none; transform: translateY(-4px) rotate(5deg); }
.module-scene-stage.is-celebrating .target-card { animation: docsy-result-arrive 460ms cubic-bezier(.2,.9,.3,1.2) 1; }
@keyframes docsy-result-arrive { from { opacity: .4; translate: -8px 4px; scale: .94; } to { opacity: 1; translate: 0 0; scale: 1; } }
@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation: none !important; transition: none !important; } }
</style>
