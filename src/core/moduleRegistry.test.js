import { describe, it, expect } from 'vitest'

// Test module registry logic without actual imports
describe('Module Registry', () => {
  const mockModules = [
    { id: 'home', name: '首页', menuItems: [{ label: '首页', route: 'home' }] },
    { id: 'doc-gen', name: '文档生成', menuItems: [{ label: '文档生成', route: 'doc-gen-form' }], homeCards: [{ title: '生成文档' }] },
    { id: 'settings', name: '设置', menuItems: [{ label: '设置', route: 'settings' }] },
  ]

  it('should extract menu items from modules', () => {
    const menuItems = mockModules
      .filter(m => m.menuItems)
      .flatMap(m => m.menuItems.map(item => ({ ...item, moduleId: m.id })))

    expect(menuItems).toHaveLength(3)
    expect(menuItems[0].moduleId).toBe('home')
    expect(menuItems[1].moduleId).toBe('doc-gen')
  })

  it('should extract home cards from modules', () => {
    const homeCards = mockModules.flatMap(m => m.homeCards || [])
    expect(homeCards).toHaveLength(1)
    expect(homeCards[0].title).toBe('生成文档')
  })

  it('should find module by id', () => {
    const found = mockModules.find(m => m.id === 'doc-gen')
    expect(found).toBeDefined()
    expect(found.name).toBe('文档生成')
  })
})
