const modules = import.meta.glob('../modules/*/index.js', { eager: true })

export const moduleRegistry = Object.values(modules)
  .map(m => m.default)
  .filter(Boolean)

export function getRoutes() {
  return moduleRegistry.flatMap(m => m.routes || [])
}

export function getMenuItems() {
  return moduleRegistry
    .filter(m => m.menuItems)
    .flatMap(m =>
      m.menuItems.map(item => ({
        ...item,
        moduleId: m.id,
      }))
    )
}

export function getHomeCards() {
  return moduleRegistry.flatMap(m => m.homeCards || [])
}

export function getModule(id) {
  return moduleRegistry.find(m => m.id === id)
}

export function getModuleSettings() {
  return moduleRegistry
    .filter(m => m.settings)
    .map(m => ({ moduleId: m.id, ...m.settings }))
}
