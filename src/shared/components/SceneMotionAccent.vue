<template>
  <svg class="scene-motion-accent" viewBox="0 0 100 120" fill="none" aria-hidden="true">
    <g v-if="scene === 'pdf-unlock'" class="unlock-turn" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
      <path d="M 66 87 V 79 A 7 7 0 0 1 80 79" />
      <rect x="62" y="87" width="22" height="17" rx="4" fill="var(--docsy-surface-elevated, white)" />
      <path d="M 73 93 V 98" />
    </g>
    <g v-else-if="scene === 'pdf-compress'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path class="press-left" d="M 2 50 H 18 M 13 45 L 18 50 L 13 55" />
      <path class="press-right" d="M 98 70 H 82 M 87 65 L 82 70 L 87 75" />
    </g>
    <g v-else-if="scene === 'pdf-merge' || scene === 'evidence'" class="binding" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
      <path d="M 24 28 H 32 M 24 45 H 32 M 24 62 H 32 M 24 79 H 32" />
    </g>
    <g v-else-if="scene === 'image-paddler'" class="align-guides" stroke="currentColor" stroke-width="1.2" stroke-dasharray="3 3">
      <path d="M 8 18 H 92 M 8 100 H 92 M 20 8 V 112 M 80 8 V 112" />
    </g>
    <g v-else-if="scene === 'video-extract'" class="capture-frame" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
      <path d="M 12 42 V 28 H 26 M 74 28 H 88 V 42 M 12 78 V 92 H 26 M 74 92 H 88 V 78" />
    </g>
    <g v-else-if="scene === 'pdf-anticopy'" class="shield-trace" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
      <path pathLength="1" d="M 50 40 L 69 47 V 64 Q 69 79 50 87 Q 31 79 31 64 V 47 Z M 40 62 L 48 70 L 61 55" />
    </g>
    <g v-else-if="scene === 'pdf-extract'" class="extract-sheet" stroke="currentColor" stroke-width="2">
      <rect x="48" y="40" width="34" height="48" rx="3" fill="var(--docsy-surface-elevated, white)" />
      <path d="M 55 52 H 75 M 55 61 H 72 M 55 70 H 68" stroke-linecap="round" />
    </g>
    <g v-else-if="scene === 'template' || scene === 'markdown'" class="writing-lines" stroke="currentColor" stroke-width="3" stroke-linecap="round">
      <path pathLength="1" d="M 25 42 H 70 M 25 56 H 62 M 25 70 H 74" />
    </g>
    <g v-else class="scan-line" stroke="currentColor" stroke-width="2" stroke-linecap="round">
      <path d="M 18 40 H 82" />
      <path d="M 18 36 V 44 M 82 36 V 44" />
    </g>
  </svg>
</template>

<script setup>
defineProps({ scene: { type: String, default: 'generic' } })
</script>

<style scoped>
.scene-motion-accent { position: absolute; inset: 0; width: 100%; height: 100%; max-height: 108px; margin: auto; color: var(--docsy-primary, #22a9bd); pointer-events: none; }
.unlock-turn { transform-origin: 73px 88px; animation: unlock-turn 1.8s ease-in-out infinite; }
.press-left { animation: press-left 1.5s ease-in-out infinite; }
.press-right { animation: press-right 1.5s ease-in-out infinite; }
.binding { animation: bind-pages 1.8s ease-in-out infinite; }
.align-guides { animation: align-guides 2s ease-in-out infinite; }
.capture-frame { transform-origin: 50px 60px; animation: capture-frame 2s ease-in-out infinite; }
.shield-trace path, .writing-lines path { stroke-dasharray: 1; animation: draw-lines 2.2s ease-in-out infinite; }
.extract-sheet { animation: extract-sheet 2s ease-in-out infinite; }
.scan-line { animation: scan-line 2s ease-in-out infinite; }
@keyframes unlock-turn { 0%, 100% { rotate: 0deg; } 45%, 75% { rotate: -18deg; } }
@keyframes press-left { 0%,100% { translate: -4px 0; opacity: .3; } 50% { translate: 3px 0; opacity: 1; } }
@keyframes press-right { 0%,100% { translate: 4px 0; opacity: .3; } 50% { translate: -3px 0; opacity: 1; } }
@keyframes bind-pages { 0%,100% { translate: -5px 0; opacity: .2; } 40%,80% { translate: 0 0; opacity: 1; } }
@keyframes align-guides { 0%,100% { translate: 0 4px; opacity: .2; } 50% { translate: 0 0; opacity: .85; } }
@keyframes capture-frame { 0%,100% { scale: 1.12; opacity: .25; } 40%,60% { scale: 1; opacity: 1; } }
@keyframes draw-lines { 0% { stroke-dashoffset: 1; opacity: .2; } 65%,90% { stroke-dashoffset: 0; opacity: .85; } 100% { stroke-dashoffset: 0; opacity: 0; } }
@keyframes extract-sheet { 0% { translate: -10px 0; opacity: 0; } 30%,70% { opacity: 1; } 100% { translate: 10px -6px; opacity: 0; } }
@keyframes scan-line { 0%,100% { translate: 0 0; opacity: .25; } 50% { translate: 0 40px; opacity: .8; } }
@media (prefers-reduced-motion: reduce) { .scene-motion-accent { display: none; } }
</style>
