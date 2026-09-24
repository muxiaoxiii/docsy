import { createHash } from 'node:crypto'
import { chmod, copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { join, dirname, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import { execFileSync } from 'node:child_process'
import { mkdtemp, rm } from 'node:fs/promises'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const destination = join(root, 'src-tauri/runtime/bin')
const licenseDestination = join(root, 'src-tauri/runtime/licenses/doc2x-BSD-3-Clause.txt')
const cache = join(root, 'src-tauri/target/runtime-cache/doc2docx')

/** Pinned @jitword/doc2docx platform packages (b2xtranslator-derived native doc2x). */
const packages = {
  'darwin-arm64': ['@jitword/doc2docx-darwin-arm64', '0.1.0', '503cfe45ae93885aece34a0c3fdef799bdc73a81610c1aa123291a60b08d5ce8'],
  'darwin-x64': ['@jitword/doc2docx-darwin-x64', '0.1.0', '37d051390b70aab6edfeb60934a456d9a02e42753195e77127bdc18208077ce9'],
  'linux-x64': ['@jitword/doc2docx-linux-x64', '0.1.0', '0d0a82def78f68baf8cb53de8d40b91cd22f7c9357634d71ff4c35b9b990ae71'],
  'linux-arm64': ['@jitword/doc2docx-linux-arm64', '0.1.0', 'ec758e8c81f10df5d67cbd0d0615c64a5ff318eb8d804466eff4875ef65fb081'],
  'win32-x64': ['@jitword/doc2docx-win32-x64', '0.1.0', '6b9c7c6e725d2019e86ebdaae6c13b7b51de772110a9ae486ff2dd364ca8bae5'],
}

const key = `${process.platform}-${process.arch}`
const release = packages[key]
if (!release) {
  throw new Error(`No pinned doc2docx package for ${key}; set DOCSY_DOC2X_PATH to a self-built doc2x binary`)
}

const [name, version, expected] = release
const short = name.split('/').pop()
const exe = process.platform === 'win32' ? 'doc2x.exe' : 'doc2x'
const targetExe = join(destination, exe)

await mkdir(destination, { recursive: true })
await mkdir(cache, { recursive: true })
const archive = join(cache, `${short}-${version}.tgz`)
const valid = async () =>
  createHash('sha256').update(await readFile(archive).catch(() => Buffer.alloc(0))).digest('hex') === expected

if (!(await valid())) {
  const url = `https://registry.npmjs.org/${name}/-/${name.split('/').pop()}-${version}.tgz`
  execFileSync('curl', ['--fail', '--location', '--retry', '2', '--max-time', '300', '--output', archive, url], {
    stdio: 'inherit',
  })
}
if (!(await valid())) throw new Error('doc2docx platform package checksum mismatch')

const temporary = await mkdtemp(join(tmpdir(), 'docsy-doc2x-'))
try {
  // Git Bash tar on Windows treats `D:\...` as a remote host; use Python tarfile there.
  if (process.platform === 'win32') {
    execFileSync(
      'python',
      ['-c', 'import sys,tarfile; tarfile.open(sys.argv[1]).extractall(sys.argv[2])', archive, temporary],
      { stdio: 'inherit' },
    )
  } else {
    execFileSync('tar', ['-xzf', archive, '-C', temporary], { stdio: 'inherit' })
  }
  await copyFile(join(temporary, 'package', 'bin', exe), targetExe)
  if (process.platform !== 'win32') await chmod(targetExe, 0o755)
  if (process.platform === 'darwin') {
    execFileSync('codesign', ['--force', '--sign', '-', targetExe])
  }
  await copyFile(join(temporary, 'package', 'LICENSE'), licenseDestination)
} finally {
  await rm(temporary, { recursive: true, force: true })
}

const provenance = {
  package: name,
  version,
  sha256: expected,
  license: 'BSD-3-Clause',
  derivedFrom: 'b2xtranslator',
  source: 'https://github.com/EvolutionJobs/b2xtranslator',
  wrapper: 'https://github.com/jitOffice/doc2docx',
  platform: key,
}
await writeFile(join(root, 'src-tauri/runtime/licenses/doc2x-provenance.json'), `${JSON.stringify(provenance, null, 2)}\n`)

console.log(`doc2x verified: ${targetExe}`)
