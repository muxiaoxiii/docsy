<template>
  <div
    class="module-scene-stage"
    :class="[
      `scene--${resolvedScene}`,
      {
        'is-processing': busy || processing,
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
      :class="{ 'is-nodding': nodActive, 'is-interacting': isInteracting }"
      :style="{ '--look-x': lookX, '--look-y': lookY }"
      aria-hidden="true"
      @pointermove="handlePointerMove"
      @pointerleave="handlePointerLeave"
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
          <!-- Unlock: Locked doc -->
          <template v-if="resolvedScene === 'pdf-unlock'">
            <div class="preview-line wide" />
            <div class="lock-shield-badge">🔒</div>
            <div class="preview-line medium" />
          </template>
          <!-- Merge: Multiple documents stacking -->
          <template v-else-if="resolvedScene === 'pdf-merge'">
            <div class="stacked-layer layer-1" />
            <div class="stacked-layer layer-2" />
            <div class="preview-line wide" />
            <div class="preview-line short" />
          </template>
          <!-- Compress: Dense heavy file -->
          <template v-else-if="resolvedScene === 'pdf-compress'">
            <div class="preview-line wide" />
            <div class="file-size-tag tag-heavy">100% · 24 MB</div>
            <div class="preview-line medium" />
          </template>
          <!-- Anti-copy: Unprotected file -->
          <template v-else-if="resolvedScene === 'pdf-anticopy'">
            <div class="preview-line wide" />
            <div class="preview-line medium" />
            <div class="copy-warning-badge">⚠️ 未保护</div>
          </template>
          <!-- Extract: Multi-page document with selection -->
          <template v-else-if="resolvedScene === 'pdf-extract'">
            <div class="page-corner-tags">
              <span>P1</span><span>P2</span><span class="highlight">P3</span>
            </div>
            <div class="preview-line wide" />
          </template>
          <!-- Evidence: Case files with record folder -->
          <template v-else-if="resolvedScene === 'evidence'">
            <div class="evidence-folder-tab">案卷材料</div>
            <div class="preview-line wide" />
            <div class="preview-line short" />
          </template>
          <!-- Template: Word doc with yellow highlight placeholders -->
          <template v-else-if="resolvedScene === 'template'">
            <div class="template-highlight-box">{客户名称}</div>
            <div class="preview-line wide" />
            <div class="template-highlight-box short">{签约日期}</div>
          </template>
          <!-- Image paddler: Photos -->
          <template v-else-if="resolvedScene === 'image-paddler'">
            <div class="polaroid-inner">
              <span class="cam-lens">📷</span>
            </div>
          </template>
          <!-- Video extract: Film strip -->
          <template v-else-if="resolvedScene === 'video-extract'">
            <div class="film-strip-preview">
              <span class="film-hole" /><span class="film-hole" /><span class="film-hole" />
            </div>
            <div class="preview-line medium" />
          </template>
          <!-- Generic fallback -->
          <template v-else>
            <div class="preview-line wide" />
            <div class="preview-line medium" />
            <div class="preview-line short" />
          </template>
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
          <circle r="3" class="stream-dot dot-1">
            <animateMotion path="M 10 20 C 40 6, 80 34, 110 20" dur="2.2s" repeatCount="indefinite" />
          </circle>
          <circle r="2.4" class="stream-dot dot-2">
            <animateMotion path="M 10 20 C 40 6, 80 34, 110 20" dur="2.2s" begin="0.75s" repeatCount="indefinite" />
          </circle>
          <circle r="2" class="stream-dot dot-3">
            <animateMotion path="M 10 20 C 40 6, 80 34, 110 20" dur="2.2s" begin="1.5s" repeatCount="indefinite" />
          </circle>
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
        <DocletSprite :size="106" :motion="mascotMotion" />
        <div class="mascot-ground-shadow" />
      </div>

      <!-- Right Target Result Card -->
      <div class="scene-card target-card" :class="{ 'card-received': busy || processing }">
        <div class="card-badge-row">
          <span class="card-badge badge-target">{{ sceneMeta.targetBadge }}</span>
          <span class="card-sub-icon">{{ sceneMeta.targetIcon }}</span>
        </div>
        <div class="card-content-preview">
          <!-- Unlock: Unlocked PDF with green shield -->
          <template v-if="resolvedScene === 'pdf-unlock'">
            <div class="preview-line wide" />
            <div class="unlocked-badge">🔓 已解除限制</div>
            <div class="preview-line medium" />
          </template>
          <!-- Merge: Bound Book / Unified Doc -->
          <template v-else-if="resolvedScene === 'pdf-merge'">
            <div class="bound-spine" />
            <div class="preview-line wide" />
            <div class="preview-line medium" />
            <div class="bound-tag">完整合订本</div>
          </template>
          <!-- Compress: Slimmed light file -->
          <template v-else-if="resolvedScene === 'pdf-compress'">
            <div class="preview-line wide" />
            <div class="file-size-tag tag-slim">-68% · 7.6 MB</div>
            <div class="preview-line medium" />
          </template>
          <!-- Anti-copy: Guard shield & watermark -->
          <template v-else-if="resolvedScene === 'pdf-anticopy'">
            <div class="shield-watermark">🛡️</div>
            <div class="preview-line wide" />
            <div class="secured-tag">防复制 / 防篡改</div>
          </template>
          <!-- Extract: Targeted single page -->
          <template v-else-if="resolvedScene === 'pdf-extract'">
            <div class="extracted-page-tag">导出页 3</div>
            <div class="preview-line wide" />
            <div class="preview-line medium" />
          </template>
          <!-- Evidence: Red sealed evidence page with page numbers -->
          <template v-else-if="resolvedScene === 'evidence'">
            <div class="evidence-header-tag">证据目录 · 第 1 卷</div>
            <div class="preview-line wide" />
            <div class="red-seal">证据印</div>
            <div class="evidence-footer-tag">第 001 页 / 共 036 页</div>
          </template>
          <!-- Template: Generated Word -->
          <template v-else-if="resolvedScene === 'template'">
            <div class="template-filled-line">Docsy 科技有限公司</div>
            <div class="preview-line wide" />
            <div class="template-filled-line">2026 年 10 月 20 日</div>
          </template>
          <!-- Image paddler: Padded Photo with Border -->
          <template v-else-if="resolvedScene === 'image-paddler'">
            <div class="framed-photo-preview">
              <div class="photo-inner-canvas" />
            </div>
          </template>
          <!-- Video extract: Snapshot card -->
          <template v-else-if="resolvedScene === 'video-extract'">
            <div class="snapshot-frame">
              <span class="timestamp-pill">00:01:24.500</span>
            </div>
            <div class="preview-line short" />
          </template>
          <!-- Generic fallback -->
          <template v-else>
            <div class="preview-line wide" />
            <div class="preview-line medium" />
            <div class="preview-line short" />
          </template>
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
import { computed, ref } from 'vue'
import DocletSprite from './DocletSprite.vue'

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
const lookX = ref(0)
const lookY = ref(0)
const isInteracting = ref(false)
const nodActive = ref(false)
const ripples = ref([])

function handlePointerMove(e) {
  if (!stageCanvasRef.value) return
  const rect = stageCanvasRef.value.getBoundingClientRect()
  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2
  const maxDist = Math.max(rect.width, rect.height) / 2
  const dx = (e.clientX - centerX) / (maxDist || 1)
  const dy = (e.clientY - centerY) / (maxDist || 1)
  lookX.value = Number(Math.max(-1, Math.min(1, dx)).toFixed(3))
  lookY.value = Number(Math.max(-1, Math.min(1, dy)).toFixed(3))
  isInteracting.value = true
}

function handlePointerLeave() {
  lookX.value = 0
  lookY.value = 0
  isInteracting.value = false
}

function handlePointerDown(e) {
  if (!stageCanvasRef.value) return
  const rect = stageCanvasRef.value.getBoundingClientRect()
  const x = Math.round(e.clientX - rect.left)
  const y = Math.round(e.clientY - rect.top)
  const id = Date.now() + Math.random()
  const glyphs = sceneMeta.value?.glyphs?.length ? sceneMeta.value.glyphs : ['✨', '✦', '📑', '🎉', '💫']
  const randomGlyph = glyphs[Math.floor(Math.random() * glyphs.length)]
  ripples.value.push({ id, x, y, glyph: randomGlyph })
  setTimeout(() => {
    ripples.value = ripples.value.filter((r) => r.id !== id)
  }, 850)

  // 触发 Doclet 快乐点头回应
  nodActive.value = true
  setTimeout(() => {
    nodActive.value = false
  }, 420)
}

const mascotMotion = computed(() => {
  if (props.celebrating) return 'celebrate'
  if (props.busy || props.processing) return 'review'
  if (props.dragging) return 'greet'
  if (nodActive.value) return 'greet'
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
    busyDesc: '规范压平注释，整齐编排书脊与双面打印分隔页',
    defaultTitle: '添加需要合并的文档',
    defaultDesc: '支持按顺序合并多个 PDF 或 Word 文档，自动转换排版并生成单一合订本',
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
    defaultDesc: '高效压缩文档体积，支持大幅降低传输与存储占用，不丢失排版清晰度',
    actionLabel: '添加文件',
  },
  'pdf-anticopy': {
    sourceBadge: '普通 PDF',
    sourceIcon: '📄',
    targetBadge: '防护 PDF',
    targetIcon: '🛡️',
    glyphs: ['🛡️', '🔒', '✨', '✦'],
    busyTitle: 'Doclet 正在注入安全防复制层…',
    busyDesc: '重构字形矢量轮廓，彻底杜绝文本抓取与随意盗用',
    defaultTitle: '添加需保护的 PDF 文档',
    defaultDesc: '应用独家矢量防复制技术，肉眼阅读完全无损，复制粘贴呈现乱码',
    actionLabel: '添加文件',
  },
  'pdf-extract': {
    sourceBadge: '源 PDF',
    sourceIcon: '📑',
    targetBadge: '目标页面',
    targetIcon: '🎯',
    glyphs: ['✂️', '🎯', '📄', '✦'],
    busyTitle: 'Doclet 正在定向提取页面…',
    busyDesc: '分离选定页码并完整保留页面资源与书签结构',
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
    busyTitle: 'Doclet 正在编排印章与流水页码…',
    busyDesc: '生成证据目录、编制流水页码与规范红蓝印章',
    defaultTitle: '导入证据 PDF 或材料',
    defaultDesc: '智能拆分连续合并证据，或批量导入证据材料生成规范案卷与流水页码',
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
    targetBadge: '高光快照',
    targetIcon: '📸',
    glyphs: ['🎬', '⏱️', '⚡', '✦'],
    busyTitle: 'Doclet 正在扫描视频时间轴…',
    busyDesc: '按指定间隔或关键帧截取无损清晰画面',
    defaultTitle: '选择或拖入视频文件',
    defaultDesc: '精准提取视频序列帧或高光画面，支持时间轴自选与批量导出',
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
  border-radius: var(--docsy-radius-sm, 6px);
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
