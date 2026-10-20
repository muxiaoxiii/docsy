import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref } from 'vue'

// No polling: the browser tells us when a surface can animate.
export function useMotionActivity(root) {
  const inView = ref(false)
  const visible = ref(true)
  const reduced = ref(false)
  const activated = ref(true)
  const active = computed(() => inView.value && visible.value && activated.value && !reduced.value)
  let observer
  let media
  const updateVisibility = () => { visible.value = !document.hidden }
  const updatePreference = () => { reduced.value = media.matches }

  onMounted(() => {
    updateVisibility()
    media = window.matchMedia('(prefers-reduced-motion: reduce)')
    updatePreference()
    media.addEventListener('change', updatePreference)
    document.addEventListener('visibilitychange', updateVisibility)
    if (window.IntersectionObserver) {
      observer = new window.IntersectionObserver(([entry]) => { inView.value = entry.isIntersecting })
      observer.observe(root.value)
    } else inView.value = true
  })
  onActivated(() => { activated.value = true })
  onDeactivated(() => { activated.value = false })
  onBeforeUnmount(() => {
    observer?.disconnect()
    media?.removeEventListener('change', updatePreference)
    document.removeEventListener('visibilitychange', updateVisibility)
  })
  return { active, reduced }
}
