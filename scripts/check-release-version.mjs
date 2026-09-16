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
if (process.env.GITHUB_REF_TYPE === 'tag') assert.equal(process.env.GITHUB_REF_NAME, `v${version}`)
console.log(`Release versions agree: ${version}`)
