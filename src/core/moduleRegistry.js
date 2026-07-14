const modules = import.meta.glob('../modules/*/index.js', { eager: true })

export const moduleRegistry = Object.values(modules)
  .map(m => m.default)
  .filter(Boolean)
  .sort((a, b) => (a.order ?? 1000) - (b.order ?? 1000))

export function getRoutes() {
  return moduleRegistry.flatMap(m => m.routes || [])
}

export function getMenuItems(settings = {}) {
  return orderedModules(settings)
    .filter(m => m.menuItems)
    .flatMap(m =>
      m.menuItems.map(item => ({
        ...item,
        moduleId: m.id,
      }))
    )
    .filter(item => isModuleVisible(item.moduleId, settings) && isModuleVisible(item.route, settings))
}

export function getHomeCards(settings = {}) {
  return orderedModules(settings)
    .filter(m => isModuleVisible(m.id, settings))
    .flatMap(m => (m.homeCards || []).map(card => ({ ...card, moduleId: m.id })))
}

export function getModule(id) {
  return moduleRegistry.find(m => m.id === id)
}

export function getModuleSettings() {
  return moduleRegistry
    .filter(m => m.settings)
    .map(m => ({ moduleId: m.id, ...m.settings }))
}

export function getMenuModules() {
  return moduleRegistry
    .filter(m => m.menuItems?.length)
    .map(m => ({
      id: m.id,
      name: m.name,
      icon: m.icon,
      defaultVisible: m.defaultVisible !== false,
    }))
}

export function defaultMenuOrder() {
  return getMenuModules().map(m => m.id)
}

function orderedModules(settings = {}) {
  const order = Array.isArray(settings.menu_order) ? settings.menu_order : []
  const orderIndex = new Map(order.map((id, index) => [id, index]))
  return [...moduleRegistry].sort((a, b) => {
    const aRank = orderIndex.has(a.id) ? orderIndex.get(a.id) : Number.MAX_SAFE_INTEGER
    const bRank = orderIndex.has(b.id) ? orderIndex.get(b.id) : Number.MAX_SAFE_INTEGER
    if (aRank !== bRank) return aRank - bRank
    return (a.order ?? 1000) - (b.order ?? 1000)
  })
}

function isModuleVisible(id, settings = {}) {
  const visibility = settings.menu_visibility || {}
  return visibility[id] !== false
}
