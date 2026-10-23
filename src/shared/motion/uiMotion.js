// Adapted interaction vocabulary from TuneShape (MIT), ff7871f.
// Vue/Element Plus retain all focus, keyboard, selection and click behavior.
const running = new Map()
const easing = 'cubic-bezier(.22,1,.36,1)'

function canAnimate(element) {
  return (
    !!element?.animate &&
    !document.hidden &&
    !window.matchMedia('(prefers-reduced-motion: reduce)').matches &&
    element.getClientRects().length > 0
  )
}

function renderedPose(element, frame) {
  const style = window.getComputedStyle(element)
  return Object.fromEntries(Object.keys(frame).map((key) => [key, style[key]]))
}

/** Bounded, interruptible animations; nothing runs while the interface is idle. */
export function playMotion(element, frames, options = {}, settled = () => {}) {
  const previous = running.get(element)
  const from = previous ? renderedPose(element, frames[0]) : frames[0]
  previous?.cancel()
  if (!canAnimate(element)) {
    settled()
    return
  }
  let timer,
    finished = false
  const animation = element.animate([from, ...frames.slice(1)], { duration: 260, easing, ...options })
  const finish = () => {
    if (finished) return
    finished = true
    clearTimeout(timer)
    if (running.get(element) === control) running.delete(element)
    animation.onfinish = null
    animation.cancel()
    settled()
  }
  const control = {
    finish,
    cancel() {
      if (finished) return
      finished = true
      clearTimeout(timer)
      animation.onfinish = null
      animation.cancel()
      if (running.get(element) === control) running.delete(element)
    },
  }
  running.set(element, control)
  animation.onfinish = finish
  // Webviews may suspend compositor callbacks in background windows. Never let
  // an animation callback hold up dialog closing or the next import batch.
  timer = setTimeout(finish, (options.duration ?? 260) + 100)
}

export function stopMotion(element) {
  running.get(element)?.cancel()
}
export function finishAllMotion() {
  for (const control of [...running.values()]) control.finish()
}

function dialogSurface(element) {
  return element.querySelector('.el-dialog') || element
}
export const dialogTransition = {
  name: 'docsy-dialog',
  css: false,
  onEnter(element, done) {
    playMotion(
      dialogSurface(element),
      [
        { opacity: 0, translate: '0 20px', scale: '.96' },
        { opacity: 1, translate: '0 -1px', scale: '1.004', offset: 0.75 },
        { opacity: 1, translate: '0 0', scale: '1' },
      ],
      { duration: 300 },
      done,
    )
  },
  onLeave(element, done) {
    const surface = dialogSurface(element)
    playMotion(
      surface,
      [
        renderedPose(surface, { opacity: 1, translate: '0 0', scale: '1' }),
        { opacity: 0, translate: '0 10px', scale: '.98' },
      ],
      { duration: 140, fill: 'forwards' },
      done,
    )
  },
}

function activePanel(element) {
  return [...element.querySelectorAll(':scope > .el-tabs__content > .el-tab-pane')].find(
    (pane) => pane.style.display !== 'none' && !pane.hidden,
  )
}
const tabStates = new WeakMap()
export const motionTabs = {
  mounted(element) {
    const nav = element.querySelector(':scope > .el-tabs__header .el-tabs__nav-wrap')
    if (nav) mountSelection(nav, '.el-tabs__item.is-active', 'docsy-motion-tab-nav')
    tabStates.set(element, { panel: activePanel(element), nav })
  },
  updated(element, binding) {
    if (binding.value === binding.oldValue) return
    const state = tabStates.get(element)
    if (!state) return
    if (state.nav) placeSelection(state.nav, true)
    const panel = activePanel(element)
    const panels = [...element.querySelectorAll(':scope > .el-tabs__content > .el-tab-pane')]
    const direction = panels.indexOf(panel) >= panels.indexOf(state.panel) ? 1 : -1
    if (state.panel !== panel) stopMotion(state.panel)
    state.panel = panel
    if (panel)
      playMotion(panel, [
        { opacity: 0, translate: `${direction * 18}px 0`, scale: '.99' },
        { opacity: 1, translate: '0 0', scale: '1' },
      ])
  },
  beforeUnmount(element) {
    const state = tabStates.get(element)
    stopMotion(state?.panel)
    if (state?.nav) unmountSelection(state.nav)
    tabStates.delete(element)
  },
}

const selectionStates = new WeakMap()
function placeSelection(element, animate = false) {
  const state = selectionStates.get(element)
  if (!state) return
  const selected = element.querySelector(state.selector)
  if (!selected) {
    state.indicator.style.visibility = 'hidden'
    state.placed = false
    stopMotion(state.indicator)
    return
  }
  if (!element.offsetWidth || !element.offsetHeight) return
  state.indicator.style.visibility = 'visible'
  const root = element.getBoundingClientRect(),
    rect = selected.getBoundingClientRect()
  const rootStyle = window.getComputedStyle(element)
  // offsetWidth rounds fractional pixels and visibly drifts on the later tabs.
  const sx = root.width / (parseFloat(rootStyle.width) || root.width) || 1,
    sy = root.height / (parseFloat(rootStyle.height) || root.height) || 1
  const borderX = parseFloat(rootStyle.borderLeftWidth) || 0,
    borderY = parseFloat(rootStyle.borderTopWidth) || 0
  const target = {
    transform: `translate(${(rect.left - root.left) / sx - borderX + (element.scrollLeft || 0)}px, ${(rect.top - root.top) / sy - borderY + (element.scrollTop || 0)}px)`,
    width: `${rect.width / sx}px`,
    height: `${rect.height / sy}px`,
    borderRadius: window.getComputedStyle(selected).borderRadius,
  }
  const from = state.placed ? renderedPose(state.indicator, target) : target
  Object.assign(state.indicator.style, target)
  if (animate && state.placed) playMotion(state.indicator, [from, target], { duration: 280 })
  else stopMotion(state.indicator)
  state.placed = true
  element.classList.add('docsy-selection-ready')
}
export const motionSelection = {
  mounted(element) {
    mountSelection(element, '.el-radio-button.is-active .el-radio-button__inner', 'docsy-motion-selection')
  },
  updated(element, binding) {
    placeSelection(element, binding.value !== binding.oldValue)
  },
  beforeUnmount: unmountSelection,
}

function mountSelection(element, selector, className) {
  const indicator = document.createElement('span')
  indicator.className = 'docsy-selection-indicator'
  indicator.setAttribute('aria-hidden', 'true')
  element.prepend(indicator)
  element.classList.add(className)
  const observer = window.ResizeObserver ? new window.ResizeObserver(() => placeSelection(element)) : null
  selectionStates.set(element, { indicator, observer, selector, className, placed: false })
  observer?.observe(element)
  placeSelection(element)
}
function unmountSelection(element) {
  const state = selectionStates.get(element)
  state?.observer?.disconnect()
  stopMotion(state?.indicator)
  state?.indicator.remove()
  if (state) element.classList.remove(state.className, 'docsy-selection-ready')
  selectionStates.delete(element)
}
export const motionNavigation = {
  mounted(element, binding) {
    mountSelection(
      element,
      binding.arg === 'footer' ? '.footer-btn.active' : '.el-menu-item.is-active',
      'docsy-motion-navigation',
    )
  },
  updated(element, binding) {
    placeSelection(element, binding.value !== binding.oldValue)
  },
  beforeUnmount: unmountSelection,
}

export function installUiMotion(app) {
  app.directive('motion-tabs', motionTabs)
  app.directive('motion-selection', motionSelection)
  app.directive('motion-navigation', motionNavigation)
  const preference = window.matchMedia('(prefers-reduced-motion: reduce)')
  const settle = () => {
    if (document.hidden || preference.matches) finishAllMotion()
  }
  preference.addEventListener('change', settle)
  document.addEventListener('visibilitychange', settle)
  const dispose = () => {
    finishAllMotion()
    preference.removeEventListener('change', settle)
    document.removeEventListener('visibilitychange', settle)
  }
  app.onUnmount?.(dispose)
  return dispose
}
