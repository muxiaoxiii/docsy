import { readFileSync } from 'node:fs'
import assert from 'node:assert/strict'

const read = path => readFileSync(new URL('../' + path, import.meta.url), 'utf8')
const version = JSON.parse(read('package.json')).version
const lock = JSON.parse(read('package-lock.json'))
assert.equal(lock.version, version)
assert.equal(lock.packages[''].version, version)
assert.equal(JSON.parse(read('src-tauri/tauri.conf.json')).version, version)
assert.equal(read('src-tauri/Cargo.toml').match(/^version = "([^"]+)"/m)?.[1], version)
assert.equal(read('src-tauri/Cargo.lock').match(/name = "docsy"\r?\nversion = "([^"]+)"/)?.[1], version)
assert.ok(read('CHANGELOG.md').includes(`## [${version}]`))

// Source candidates and publicly downloadable artifacts have separate provenance.
const siteIndex = read('site/public/index.html')
assert.ok(siteIndex.includes(`name="docsy-source-version" content="${version}"`))
const publishedVersion = JSON.parse(read('site/public/data/releases.json')).latest
assert.ok(siteIndex.includes(`id="dl-version-static">v${publishedVersion}<`))
assert.ok(siteIndex.includes(`Docsy_${publishedVersion}_`))
const readme = read('README.md')
assert.ok(
  readme.includes(`version-${version}-blue`),
  `README.md 版本徽章应为 ${version}`,
)

// 本地 changelog 数据文件首条也必须是当前版本（部署脚本会再从 CHANGELOG 重生成）。
const siteChangelog = JSON.parse(read('site/public/data/changelog.json'))
assert.equal(
  siteChangelog[0]?.version,
  version,
  `site/public/data/changelog.json 首条版本应为 ${version}`,
)

if (process.env.GITHUB_REF_TYPE === 'tag') assert.equal(process.env.GITHUB_REF_NAME, `v${version}`)
console.log(`Release versions agree: ${version}`)
