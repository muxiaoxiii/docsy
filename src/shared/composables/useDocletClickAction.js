import { onBeforeUnmount, ref } from 'vue'
import { randomDocletMotion } from '../docletInteractions.js'
import { docletMotionDuration } from '../docletVideoAssets.js'
export function useDocletClickAction() {
  const motion = ref('')
  let previous = ''
  let timer
  function play() {
    window.clearTimeout(timer)
    previous = randomDocletMotion(previous)
    motion.value = previous
    timer = window.setTimeout(() => { motion.value = '' }, docletMotionDuration(previous) + 180)
  }
  onBeforeUnmount(() => window.clearTimeout(timer))
  return { motion, play }
}
