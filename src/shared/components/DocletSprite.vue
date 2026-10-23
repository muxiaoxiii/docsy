<template>
  <span
    ref="root"
    class="doclet-sprite"
    :class="{ 'is-paused': !motionActive }"
    :style="{ width: size + 'px', height: Math.round((size * 13) / 12) + 'px' }"
    aria-hidden="true"
  >
    <!-- Experimental vector engine stays opt-in; the original Doclet is the default. -->
    <DocletSvgMascot
      v-if="engine === 'svg'"
      :size="size"
      :motion="motion"
      :look-x="lookX"
      :look-y="lookY"
      :interactive="interactive"
    />

    <canvas
      v-else-if="gazeEnabled && !videoFailed"
      ref="gazeCanvas"
      :key="gazeDiagnosticMode"
      class="doclet-sprite__gaze"
      :style="gazeSize"
    />

    <!-- Pre-keyed Gemini footage, decoded only while this mascot is visible. -->
    <img
      v-else-if="engine === 'video' && !videoFailed"
      :key="motion"
      class="doclet-sprite__video"
      :src="motionActive ? videoAsset.animation : videoAsset.poster"
      alt=""
      decoding="async"
      draggable="false"
      @error="videoFailed = true"
    />

    <!-- Original sprite remains available if the derived asset cannot load. -->
    <span v-else :key="motion" class="doclet-sprite__pose" :class="`doclet-sprite__pose--${motion}`">
      <span
        class="doclet-sprite__frames"
        :class="`doclet-sprite__frames--${motion}`"
        :style="frameStyle"
      />
    </span>
  </span>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { DOCLET_MOTIONS, docletVideoAsset } from '../docletVideoAssets.js'
import { gazeDiagnosticMode } from '../composables/docletGazeDiagnostics.js'
import { useDocletGaze } from '../composables/useDocletGaze.js'
import { useMotionActivity } from '../composables/useMotionActivity.js'
import spritesheet from '../../assets/doclet-v2-spritesheet.webp'
import DocletSvgMascot from './DocletSvgMascot.vue'

const motions = {
  idle: { row: 0, frames: 6, duration: 2.4 },
  greet: { row: 3, frames: 4, duration: 1.4 },
  celebrate: { row: 4, frames: 5, duration: 1.2 },
  error: { row: 5, frames: 8, duration: 2.8 },
  waiting: { row: 6, frames: 6, duration: 3.2 },
  working: { row: 7, frames: 6, duration: 1.2 },
  review: { row: 8, frames: 6, duration: 2 },
  look: { row: 9, frames: 8, duration: 5.6 },
}

const props = defineProps({
  size: { type: Number, default: 144 },
  motion: {
    type: String,
    default: 'idle',
    validator: (value) =>
      DOCLET_MOTIONS.includes(value),
  },
  engine: { type: String, default: 'video', validator: (v) => ['svg', 'raster', 'video'].includes(v) },
  gazeTracking: { type: Boolean, default: true },
  lookX: { type: Number, default: null },
  lookY: { type: Number, default: null },
  interactive: { type: Boolean, default: false },
})

const videoFailed = ref(false)
const videoAsset = computed(() => docletVideoAsset(props.motion))
watch(() => props.motion, () => { videoFailed.value = false })
const root = ref(null)
const { active: motionActive } = useMotionActivity(root)
const gazeEnabled = computed(() => motionActive.value && props.engine === 'video' && ['idle', 'look', 'waiting'].includes(props.motion) && props.gazeTracking)
const gazeCanvas = ref(null)
useDocletGaze(root, gazeCanvas, gazeEnabled, () => { videoFailed.value = true })
const gazeSize = computed(() => {
  const height = Math.round(props.size * 13 / 12)
  return { width: `${height * .8}px`, height: `${height}px` }
})

const frameStyle = computed(() => {
  const action = motions[props.motion] || motions.idle
  return {
    backgroundImage: `url(${spritesheet})`,
    transform: `scale(${props.size / 144})`,
    '--doclet-row': `${-action.row * 156}px`,
    '--doclet-last-frame': `${-(action.frames - 1) * 144}px`,
    '--doclet-steps': action.frames - 1,
    '--doclet-duration': `${action.duration}s`,
  }
})

</script>

<style scoped>
.doclet-sprite {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  position: relative;
}
.doclet-sprite__gaze { display: block; flex: none; pointer-events: none; }
.doclet-sprite__video {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
  pointer-events: none;
}
.doclet-sprite__frames {
  display: block;
  width: 144px;
  height: 156px;
  transform-origin: top left;
  background-repeat: no-repeat;
  background-size: 1152px 1716px;
  background-position: 0 var(--doclet-row);
  animation: doclet-frames var(--doclet-duration) steps(var(--doclet-steps), end) infinite;
}
.doclet-sprite__pose { overflow: hidden; display: block; width: 100%; height: 100%; transform-origin: 50% 90%; }
.doclet-sprite__pose--greet { animation: doclet-greet 1.4s ease-in-out 2; }
.doclet-sprite__pose--celebrate { animation: doclet-hop 1.2s ease-in-out 2; }
.doclet-sprite__pose--error { animation: doclet-tilt 1.6s ease-in-out; }
.is-paused .doclet-sprite__frames,
.is-paused .doclet-sprite__pose { animation-play-state: paused; }
@keyframes doclet-frames {
  0% { background-position: 0 var(--doclet-row); }
  85%, 100% { background-position: var(--doclet-last-frame) var(--doclet-row); }
}
@keyframes doclet-greet {
  0%, 100% { transform: rotate(0); }
  25% { transform: rotate(-5deg); }
  60% { transform: rotate(4deg); }
}
@keyframes doclet-hop {
  0%, 70%, 100% { transform: translateY(0); }
  15% { transform: translateY(2px) scaleY(0.96); }
  38% { transform: translateY(-12px) rotate(-3deg); }
  58% { transform: translateY(0) scaleY(0.96); }
}
@keyframes doclet-tilt {
  0%, 100% { transform: rotate(0); }
  40%, 65% { transform: rotate(-6deg); }
}
</style>

<style scoped>
@media (prefers-reduced-motion: reduce) {
  .doclet-sprite__frames, .doclet-sprite__pose { animation: none !important; }
}
</style>
