<template>
  <span
    ref="root"
    class="doclet-sprite"
    :class="{ 'is-paused': !inView || !pageVisible }"
    :style="{ width: size + 'px', height: Math.round((size * 13) / 12) + 'px' }"
    aria-hidden="true"
  >
    <!-- Modern High-FPS Vector SVG Mascot Engine -->
    <DocletSvgMascot
      v-if="engine !== 'raster'"
      :size="size"
      :motion="motion"
      :look-x="lookX"
      :look-y="lookY"
      :interactive="interactive"
    />

    <!-- Fallback Legacy Raster Spritesheet (if explicitly requested) -->
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
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
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
      ['idle', 'greet', 'celebrate', 'error', 'waiting', 'working', 'review', 'look'].includes(value),
  },
  engine: { type: String, default: 'svg', validator: (v) => ['svg', 'raster'].includes(v) },
  lookX: { type: Number, default: null },
  lookY: { type: Number, default: null },
  interactive: { type: Boolean, default: true },
})

const root = ref(null)
const inView = ref(false)
const pageVisible = ref(true)
let observer

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

function updateVisibility() {
  pageVisible.value = !document.hidden
}

onMounted(() => {
  updateVisibility()
  document.addEventListener('visibilitychange', updateVisibility)
  if (typeof window.IntersectionObserver !== 'undefined') {
    observer = new window.IntersectionObserver((entries) => {
      inView.value = entries[0]?.isIntersecting ?? false
    })
    observer.observe(root.value)
  } else {
    inView.value = true
  }
})

onBeforeUnmount(() => {
  observer?.disconnect()
  document.removeEventListener('visibilitychange', updateVisibility)
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
.doclet-sprite__pose { display: block; width: 100%; height: 100%; transform-origin: 50% 90%; }
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
