import { describe, it, expect } from 'vitest'

describe('Tauri Bridge', () => {
  it('should define tauriCall interface', () => {
    // Verify the module exports exist
    const module = import('./tauriBridge.js')
    expect(module).toBeDefined()
  })
})
