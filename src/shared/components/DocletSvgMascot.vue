<template>
  <div
    ref="rootRef"
    class="doclet-svg-mascot"
    :class="[
      `motion--${motion}`,
      {
        'is-blinking': isBlinking,
        'is-winking': isWinking,
        'is-celebrating': motion === 'celebrate',
        'is-reviewing': motion === 'review' || motion === 'working',
        'is-hovered': isHovered,
      },
    ]"
    :style="{
      width: `${size}px`,
      height: `${Math.round(size * 1.125)}px`,
      '--mascot-look-x': currentLookX,
      '--mascot-look-y': currentLookY,
    }"
    role="img"
    :aria-label="`Doclet 吉祥物 (${motion})`"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
    @click="handleClick"
  >
    <svg
      class="mascot-svg"
      viewBox="0 0 160 180"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <!-- Soft Ground Shadow Gradient -->
        <radialGradient id="doclet-shadow-grad" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="#051c3d" stop-opacity="0.18" />
          <stop offset="100%" stop-color="#051c3d" stop-opacity="0" />
        </radialGradient>

        <!-- Body Main Gradient -->
        <linearGradient id="doclet-body-grad" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stop-color="#ffffff" />
          <stop offset="70%" stop-color="#f5f7fb" />
          <stop offset="100%" stop-color="#e3e9f3" />
        </linearGradient>

        <!-- Ear Inner Gradient -->
        <linearGradient id="doclet-ear-inner-left" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#38c2d8" />
          <stop offset="100%" stop-color="#1ea0b4" />
        </linearGradient>
        <linearGradient id="doclet-ear-inner-right" x1="100%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stop-color="#38c2d8" />
          <stop offset="100%" stop-color="#1ea0b4" />
        </linearGradient>

        <!-- Pupil Gradient -->
        <radialGradient id="doclet-pupil-grad" cx="40%" cy="35%" r="65%">
          <stop offset="0%" stop-color="#0d325a" />
          <stop offset="75%" stop-color="#021a36" />
          <stop offset="100%" stop-color="#010e1e" />
        </radialGradient>

        <!-- Tech Visor / Scan Beam -->
        <linearGradient id="doclet-scan-beam" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stop-color="#22a9bd" stop-opacity="0" />
          <stop offset="50%" stop-color="#22a9bd" stop-opacity="0.4" />
          <stop offset="100%" stop-color="#22a9bd" stop-opacity="0" />
        </linearGradient>

        <!-- Eye Clip Paths for Smooth Vector Blinking -->
        <clipPath id="left-eye-clip">
          <ellipse
            cx="58"
            cy="62"
            rx="14"
            :ry="leftEyeRy"
          />
        </clipPath>
        <clipPath id="right-eye-clip">
          <ellipse
            cx="102"
            cy="62"
            rx="14"
            :ry="rightEyeRy"
          />
        </clipPath>
      </defs>

      <!-- Ambient Ground Shadow -->
      <ellipse
        class="ground-shadow"
        cx="80"
        cy="166"
        :rx="shadowRx"
        :ry="shadowRy"
        fill="url(#doclet-shadow-grad)"
      />

      <!-- Whole Body Animated Floating Wrapper -->
      <g class="body-float-wrapper">
        <!-- Feet / Little Paws -->
        <g class="feet-group">
          <ellipse cx="62" cy="154" rx="11" ry="7" fill="#dce3ee" stroke="#b4c2d4" stroke-width="1.2" />
          <ellipse cx="98" cy="154" rx="11" ry="7" fill="#dce3ee" stroke="#b4c2d4" stroke-width="1.2" />
        </g>

        <!-- Torso & Belly -->
        <g class="torso-group">
          <!-- Torso Base -->
          <path
            d="M 50,96 C 42,112 44,142 55,150 C 66,158 94,158 105,150 C 116,142 118,112 110,96 Z"
            fill="url(#doclet-body-grad)"
            stroke="#cfd8e5"
            stroke-width="1.5"
          />
          <!-- Belly Highlight -->
          <ellipse cx="80" cy="126" rx="22" ry="17" fill="#ffffff" opacity="0.85" />
          <!-- Tech Collar Tag / Bowtie -->
          <rect x="73" y="96" width="14" height="6" rx="3" fill="#22a9bd" />
          <circle cx="80" cy="99" r="1.5" fill="#ffffff" />
        </g>

        <!-- Left Arm / Paw -->
        <g class="arm-left" :class="{ 'is-raised': motion === 'celebrate' }">
          <ellipse cx="44" cy="115" rx="8" ry="6" fill="#f0f4fa" stroke="#cfd8e5" stroke-width="1.2" />
        </g>

        <!-- Right Arm / Paw (Interactive Wave in Greet) -->
        <g
          class="arm-right"
          :class="{
            'is-waving': motion === 'greet',
            'is-raised': motion === 'celebrate',
          }"
        >
          <ellipse cx="116" cy="115" rx="8" ry="6" fill="#f0f4fa" stroke="#cfd8e5" stroke-width="1.2" />
        </g>

        <!-- Ears with Counter-Parallax Shift -->
        <g
          class="ears-group"
          :style="{
            transform: `translate3d(${earParallaxX}px, ${earParallaxY}px, 0)`,
          }"
        >
          <!-- Left Ear -->
          <g class="ear-left">
            <path
              d="M 46,30 C 35,16 26,2 32,-2 C 38,-4 50,8 56,22 Z"
              fill="url(#doclet-body-grad)"
              stroke="#02274e"
              stroke-width="2.5"
              stroke-linejoin="round"
            />
            <!-- Ear Inner Glow -->
            <path
              d="M 43,24 C 36,14 31,4 34,2 C 37,1 44,9 48,18 Z"
              fill="url(#doclet-ear-inner-left)"
            />
          </g>

          <!-- Right Ear -->
          <g class="ear-right">
            <path
              d="M 114,30 C 125,16 134,2 128,-2 C 122,-4 110,8 104,22 Z"
              fill="url(#doclet-body-grad)"
              stroke="#02274e"
              stroke-width="2.5"
              stroke-linejoin="round"
            />
            <!-- Ear Inner Glow -->
            <path
              d="M 117,24 C 124,14 129,4 126,2 C 123,1 116,9 112,18 Z"
              fill="url(#doclet-ear-inner-right)"
            />
          </g>
        </g>

        <!-- Head & Face Group (Follows Pointer with Smooth 3D Perspective) -->
        <g
          class="head-face-group"
          :style="{
            transform: `translate3d(${headParallaxX}px, ${headParallaxY}px, 0)`,
          }"
        >
          <!-- Head Contour -->
          <path
            d="M 36,54 C 36,28 54,16 80,16 C 106,16 124,28 124,54 C 124,84 114,104 80,104 C 46,104 36,84 36,54 Z"
            fill="url(#doclet-body-grad)"
            stroke="#02274e"
            stroke-width="2.6"
            stroke-linejoin="round"
          />

          <!-- Cheeks / Soft Blush -->
          <ellipse cx="44" cy="74" rx="7" ry="4" fill="#ff7a8a" opacity="0.4" />
          <ellipse cx="116" cy="74" rx="7" ry="4" fill="#ff7a8a" opacity="0.4" />

          <!-- ================= EYES ================= -->
          <g class="eyes-container">
            <!-- Left Eye White -->
            <ellipse
              cx="58"
              cy="62"
              rx="14"
              ry="17"
              fill="#ffffff"
              stroke="#02274e"
              stroke-width="2"
            />

            <!-- Left Eye Pupil with Dynamic 60fps Tracking -->
            <g clip-path="url(#left-eye-clip)">
              <!-- Pupil Body -->
              <ellipse
                :cx="leftPupilX"
                :cy="leftPupilY"
                :rx="pupilRx"
                :ry="pupilRy"
                fill="url(#doclet-pupil-grad)"
              />
              <!-- Iris Cyan Rim -->
              <ellipse
                :cx="leftPupilX"
                :cy="leftPupilY"
                rx="6"
                ry="8"
                fill="none"
                stroke="#22a9bd"
                stroke-width="1.2"
                opacity="0.8"
              />
              <!-- Catchlight Specular Dots (Life & Soul) -->
              <circle
                :cx="leftPupilX - 2.8"
                :cy="leftPupilY - 3.2"
                r="3.2"
                fill="#ffffff"
              />
              <circle
                :cx="leftPupilX + 2.5"
                :cy="leftPupilY + 3"
                r="1.6"
                fill="#ffffff"
                opacity="0.85"
              />
            </g>

            <!-- Left Eye Closed Arc (Visible during Blink) -->
            <path
              v-if="isLeftClosed"
              d="M 46,63 Q 58,70 70,63"
              stroke="#02274e"
              stroke-width="2.6"
              stroke-linecap="round"
              fill="none"
            />

            <!-- Right Eye White -->
            <ellipse
              cx="102"
              cy="62"
              rx="14"
              ry="17"
              fill="#ffffff"
              stroke="#02274e"
              stroke-width="2"
            />

            <!-- Right Eye Pupil with Dynamic 60fps Tracking -->
            <g clip-path="url(#right-eye-clip)">
              <!-- Pupil Body -->
              <ellipse
                :cx="rightPupilX"
                :cy="rightPupilY"
                :rx="pupilRx"
                :ry="pupilRy"
                fill="url(#doclet-pupil-grad)"
              />
              <!-- Iris Cyan Rim -->
              <ellipse
                :cx="rightPupilX"
                :cy="rightPupilY"
                rx="6"
                ry="8"
                fill="none"
                stroke="#22a9bd"
                stroke-width="1.2"
                opacity="0.8"
              />
              <!-- Catchlight Specular Dots -->
              <circle
                :cx="rightPupilX - 2.8"
                :cy="rightPupilY - 3.2"
                r="3.2"
                fill="#ffffff"
              />
              <circle
                :cx="rightPupilX + 2.5"
                :cy="rightPupilY + 3"
                r="1.6"
                fill="#ffffff"
                opacity="0.85"
              />
            </g>

            <!-- Right Eye Closed Arc (Visible during Blink or Wink) -->
            <path
              v-if="isRightClosed"
              d="M 90,63 Q 102,70 114,63"
              stroke="#02274e"
              stroke-width="2.6"
              stroke-linecap="round"
              fill="none"
            />

            <!-- Celebrate Sparkle Eyes (Star Burst) -->
            <template v-if="motion === 'celebrate'">
              <text x="58" y="68" font-size="20" text-anchor="middle" fill="#ffb703" class="star-eye">✦</text>
              <text x="102" y="68" font-size="20" text-anchor="middle" fill="#ffb703" class="star-eye">✦</text>
            </template>
          </g>

          <!-- ================= MOUTH ================= -->
          <g class="mouth-group">
            <!-- Normal Cute Smile -->
            <path
              v-if="motion !== 'celebrate' && motion !== 'greet' && motion !== 'error'"
              d="M 75,76 Q 80,80 85,76"
              stroke="#02274e"
              stroke-width="2.4"
              stroke-linecap="round"
              fill="none"
            />
            <!-- Happy Open Smile (Celebrate / Greet) -->
            <path
              v-else-if="motion === 'celebrate' || motion === 'greet'"
              d="M 74,75 Q 80,83 86,75 Z"
              fill="#ff6b81"
              stroke="#02274e"
              stroke-width="2"
            />
            <!-- Confused / Error Mouth -->
            <path
              v-else-if="motion === 'error'"
              d="M 73,78 Q 77,75 80,78 T 87,78"
              stroke="#02274e"
              stroke-width="2.2"
              stroke-linecap="round"
              fill="none"
            />
          </g>

          <!-- Review Visor / Holographic HUD when Reviewing -->
          <g v-if="motion === 'review' || motion === 'working'" class="hud-scanner">
            <line x1="42" y1="62" x2="118" y2="62" stroke="#22a9bd" stroke-width="1.5" stroke-dasharray="3 3" opacity="0.65" />
            <rect x="40" y="52" width="80" height="20" rx="3" fill="url(#doclet-scan-beam)" />
          </g>
        </g>
      </g>

      <!-- Click Confetti / Sparkle Bursts -->
      <g v-for="p in clickBursts" :key="p.id" class="click-particle">
        <text :x="p.x" :y="p.y" font-size="14" text-anchor="middle">{{ p.glyph }}</text>
      </g>
    </svg>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = defineProps({
  size: { type: Number, default: 120 },
  motion: {
    type: String,
    default: 'idle',
    validator: (v) =>
      ['idle', 'greet', 'celebrate', 'error', 'waiting', 'working', 'review', 'look'].includes(v),
  },
  lookX: { type: Number, default: null },
  lookY: { type: Number, default: null },
  trackPointer: { type: Boolean, default: true },
  interactive: { type: Boolean, default: true },
})

const emit = defineEmits(['click', 'nod'])

const rootRef = ref(null)

// Smoothed 60fps tracking coordinates (-1 to 1)
const currentLookX = ref(0)
const currentLookY = ref(0)
let targetLookX = 0
let targetLookY = 0
let animFrameId = null

// Blinking & Winking State Machine
const isBlinking = ref(false)
const isWinking = ref(false)
const isHovered = ref(false)
let blinkTimer = null

// Click burst particles
const clickBursts = ref([])

// Synchronize external look props if provided
watch(
  () => [props.lookX, props.lookY],
  ([x, y]) => {
    if (x !== null && x !== undefined) targetLookX = Math.max(-1, Math.min(1, x))
    if (y !== null && y !== undefined) targetLookY = Math.max(-1, Math.min(1, y))
  },
  { immediate: true }
)

// Global or container pointer move handler
function handleGlobalPointerMove(e) {
  if (!props.trackPointer || !rootRef.value) return
  if (props.lookX !== null && props.lookX !== undefined) return

  const rect = rootRef.value.getBoundingClientRect()
  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2
  const maxRange = Math.max(window.innerWidth / 2, 400)

  const dx = (e.clientX - centerX) / maxRange
  const dy = (e.clientY - centerY) / maxRange

  targetLookX = Math.max(-1, Math.min(1, dx * 1.5))
  targetLookY = Math.max(-1, Math.min(1, dy * 1.5))
}

function handlePointerLeaveWindow() {
  if (props.lookX !== null && props.lookX !== undefined) return
  targetLookX = 0
  targetLookY = 0
}

// 60FPS LERP Smooth Loop
function startRafLoop() {
  function tick() {
    // Smooth easing factor 0.14 for buttery responsive eye motion
    const ease = 0.14
    currentLookX.value += (targetLookX - currentLookX.value) * ease
    currentLookY.value += (targetLookY - currentLookY.value) * ease

    animFrameId = window.requestAnimationFrame(tick)
  }
  animFrameId = window.requestAnimationFrame(tick)
}

// Blinking loop (natural random blink every 3~5 seconds)
function scheduleNextBlink() {
  const delay = 2800 + Math.random() * 2600
  blinkTimer = setTimeout(() => {
    if (props.motion !== 'celebrate' && !isWinking.value) {
      isBlinking.value = true
      setTimeout(() => {
        isBlinking.value = false
        scheduleNextBlink()
      }, 160)
    } else {
      scheduleNextBlink()
    }
  }, delay)
}

function handleMouseEnter() {
  isHovered.value = true
}

function handleMouseLeave() {
  isHovered.value = false
}

function handleClick(e) {
  if (!props.interactive) return
  emit('click', e)

  // Trigger wink
  isWinking.value = true
  setTimeout(() => {
    isWinking.value = false
  }, 320)

  // Emit burst particles
  const glyphs = ['✨', '✦', '💫', '🎉', '💖']
  for (let i = 0; i < 4; i++) {
    const id = Date.now() + Math.random()
    const angle = (Math.PI * 2 * i) / 4 + (Math.random() - 0.5) * 0.5
    const dist = 32 + Math.random() * 20
    const x = 80 + Math.cos(angle) * dist
    const y = 60 + Math.sin(angle) * dist
    clickBursts.value.push({
      id,
      x,
      y,
      glyph: glyphs[Math.floor(Math.random() * glyphs.length)],
    })
    setTimeout(() => {
      clickBursts.value = clickBursts.value.filter((p) => p.id !== id)
    }, 700)
  }
}

// ================= Reactive Calculations =================
// Pupil Coordinates (clamped within sclera)
const leftPupilX = computed(() => {
  const base = 58
  const offset = currentLookX.value * 5.8
  return Number((base + offset).toFixed(2))
})
const leftPupilY = computed(() => {
  const base = 62
  const offset = currentLookY.value * 5.0
  return Number((base + offset).toFixed(2))
})
const rightPupilX = computed(() => {
  const base = 102
  const offset = currentLookX.value * 5.8
  return Number((base + offset).toFixed(2))
})
const rightPupilY = computed(() => {
  const base = 62
  const offset = currentLookY.value * 5.0
  return Number((base + offset).toFixed(2))
})

const pupilRx = computed(() => (props.motion === 'review' ? 7 : 7.6))
const pupilRy = computed(() => (props.motion === 'review' ? 8.5 : 9.5))

// Parallax 3D head & ear translations
const headParallaxX = computed(() => Number((currentLookX.value * 4.5).toFixed(2)))
const headParallaxY = computed(() => Number((currentLookY.value * 3.5).toFixed(2)))
const earParallaxX = computed(() => Number((currentLookX.value * -1.8).toFixed(2)))
const earParallaxY = computed(() => Number((currentLookY.value * -1.2).toFixed(2)))

// Eye closed status
const isLeftClosed = computed(() => isBlinking.value)
const isRightClosed = computed(() => isBlinking.value || isWinking.value)

const leftEyeRy = computed(() => (isLeftClosed.value ? 0.5 : 17))
const rightEyeRy = computed(() => (isRightClosed.value ? 0.5 : 17))

const shadowRx = computed(() => 36 + Math.abs(currentLookX.value) * 2)
const shadowRy = computed(() => 6)

onMounted(() => {
  startRafLoop()
  scheduleNextBlink()
  window.addEventListener('pointermove', handleGlobalPointerMove, { passive: true })
  window.addEventListener('pointerleave', handlePointerLeaveWindow)
})

onBeforeUnmount(() => {
  if (animFrameId) window.cancelAnimationFrame(animFrameId)
  if (blinkTimer) clearTimeout(blinkTimer)
  window.removeEventListener('pointermove', handleGlobalPointerMove)
  window.removeEventListener('pointerleave', handlePointerLeaveWindow)
})
</script>

<style scoped>
.doclet-svg-mascot {
  display: inline-block;
  position: relative;
  user-select: none;
  cursor: pointer;
  filter: drop-shadow(0 4px 12px rgba(10, 30, 60, 0.08));
  transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.mascot-svg {
  width: 100%;
  height: 100%;
  overflow: visible;
}

/* Floating breathing physics */
.body-float-wrapper {
  animation: doclet-float 3.4s ease-in-out infinite;
  transform-origin: 80px 150px;
}

@keyframes doclet-float {
  0%, 100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-4.5px);
  }
}

.ground-shadow {
  animation: shadow-scale 3.4s ease-in-out infinite;
  transform-origin: 80px 166px;
}

@keyframes shadow-scale {
  0%, 100% {
    transform: scale(1);
    opacity: 0.8;
  }
  50% {
    transform: scale(0.88);
    opacity: 0.55;
  }
}

/* Greet Wave Animation */
.arm-right.is-waving {
  transform-origin: 110px 110px;
  animation: wave-paw 1.2s ease-in-out infinite;
}

@keyframes wave-paw {
  0%, 100% {
    transform: rotate(0);
  }
  30% {
    transform: rotate(-24deg);
  }
  70% {
    transform: rotate(14deg);
  }
}

/* Celebrate Raised Arms & Jumping */
.is-celebrating .body-float-wrapper {
  animation: celebrate-bounce 0.85s cubic-bezier(0.34, 1.56, 0.64, 1) infinite;
}

@keyframes celebrate-bounce {
  0%, 100% {
    transform: translateY(0) scale(1);
  }
  35% {
    transform: translateY(-14px) scale(1.04) rotate(-2deg);
  }
  65% {
    transform: translateY(-8px) scale(1.02) rotate(2deg);
  }
}

.arm-left.is-raised {
  transform-origin: 48px 110px;
  transform: rotate(-38deg) translateY(-8px);
  transition: transform 0.3s ease;
}

.arm-right.is-raised {
  transform-origin: 112px 110px;
  transform: rotate(38deg) translateY(-8px);
  transition: transform 0.3s ease;
}

.star-eye {
  animation: star-spin 1.8s linear infinite;
  transform-origin: center;
}

@keyframes star-spin {
  0% { transform: scale(0.9); }
  50% { transform: scale(1.15); }
  100% { transform: scale(0.9); }
}

/* Reviewing HUD Pulse */
.hud-scanner {
  animation: hud-sweep 2s ease-in-out infinite alternate;
}

@keyframes hud-sweep {
  0% { opacity: 0.4; transform: translateY(-2px); }
  100% { opacity: 0.9; transform: translateY(4px); }
}

/* Click Particles */
.click-particle {
  pointer-events: none;
  animation: burst-fade 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

@keyframes burst-fade {
  0% {
    opacity: 1;
    transform: scale(0.5);
  }
  60% {
    opacity: 1;
    transform: scale(1.2);
  }
  100% {
    opacity: 0;
    transform: translateY(-16px) scale(0.9);
  }
}
</style>
