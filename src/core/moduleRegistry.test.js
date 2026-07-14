import { describe, it, expect } from 'vitest'

// Test module registry logic without actual imports
describe('Module Registry', () => {
  const mockModules = [
    { id: 'home', name: '首页', menuItems: [{ label: '首页', route: 'home' }] },
    { id: 'pdf-tools', name: 'PDF 工具', menuItems: [{ label: 'PDF 工具', route: 'pdf-tools' }], homeCards: [{ title: 'PDF 工具' }] },
    { id: 'settings', name: '设置', menuItems: [] },
  ]

  it('should extract menu items from modules', () => {
    const menuItems = mockModules
      .filter(m => m.menuItems)
      .flatMap(m => m.menuItems.map(item => ({ ...item, moduleId: m.id })))

    expect(menuItems).toHaveLength(2)
    expect(menuItems[0].moduleId).toBe('home')
    expect(menuItems[1].moduleId).toBe('pdf-tools')
  })

  it('should extract home cards from modules', () => {
    const homeCards = mockModules.flatMap(m => m.homeCards || [])
    expect(homeCards).toHaveLength(1)
    expect(homeCards[0].title).toBe('PDF 工具')
  })

  it('should find module by id', () => {
    const found = mockModules.find(m => m.id === 'pdf-tools')
    expect(found).toBeDefined()
    expect(found.name).toBe('PDF 工具')
  })
})
