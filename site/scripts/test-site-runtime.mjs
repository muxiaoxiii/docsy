import { test } from 'node:test'
import assert from 'node:assert/strict'
import vm from 'node:vm'
import { readFileSync } from 'node:fs'

const source = readFileSync(new URL('../public/js/main.js', import.meta.url), 'utf8')
async function loadPage(failedFeed) {
  const nodes = new Map()
  for (const id of ['dl-version-static', 'dl-mac-name', 'dl-win-name', 'dl-mac-link', 'dl-win-link', 'changelog-list']) {
    nodes.set(id, { textContent: '', innerHTML: '', children: [], attrs: {}, setAttribute(key, value) { this.attrs[key] = value }, appendChild(child) { this.children.push(child) } })
  }
  const fresh = { version: '9.0.0', assets: [{ platform: 'windows', name: 'windows-only.exe', size: 100, github_url: 'https://example.com/windows-only.exe' }] }
  const feeds = {
    'data/releases.json': { releases: [fresh] },
    'data/changelog.json': [{ version: '9.0.0', date: '2026-10-23', sections: [{ title: '更新', items: ['独立加载测试'] }] }],
    'data/screenshots.json': { items: [] },
  }
  vm.runInNewContext(source, {
    document: {
      readyState: 'complete',
      querySelectorAll: () => [], querySelector: () => null,
      getElementById: (id) => nodes.get(id) || null,
      createElement: () => ({ innerHTML: '' }),
    },
    window: {}, navigator: { userAgent: 'test' },
    fetch: async (url) => { if (url === failedFeed) throw new Error('unavailable'); return { ok: true, json: async () => feeds[url] } },
  })
  await new Promise((resolve) => setImmediate(resolve))
  return nodes
}

test('a missing changelog does not block current download links or cross-wire platforms', async () => {
  const nodes = await loadPage('data/changelog.json')
  assert.equal(nodes.get('dl-version-static').textContent, 'v9.0.0')
  assert.equal(nodes.get('dl-win-link').attrs.href, 'https://example.com/windows-only.exe')
  assert.notEqual(nodes.get('dl-mac-link').attrs.href, 'https://example.com/windows-only.exe')
})
test('a missing release feed does not block the changelog', async () => {
  const nodes = await loadPage('data/releases.json')
  assert.match(nodes.get('changelog-list').children.at(-1).innerHTML, /独立加载测试/)
})
