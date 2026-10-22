import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { readFileSync } from 'node:fs'
import { fileURLToPath, URL } from 'node:url'

const mathjaxVersion = JSON.parse(
  readFileSync(new URL('./node_modules/mathjax-full/package.json', import.meta.url), 'utf8'),
).version

const packageJson = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'))

function optionalRendererBoundary() {
  return {
    name: 'optional-renderer-boundary',
    generateBundle(_, bundle) {
      const visited = new Set()
      const walk = (file) => {
        if (visited.has(file)) return
        visited.add(file)
        const chunk = bundle[file]
        if (chunk?.type !== 'chunk') return
        const eager = Object.keys(chunk.modules).find((id) =>
          /node_modules\/(?:mathjax-full|mermaid|cytoscape|katex)\//.test(id),
        )
        if (eager) this.error(`Optional renderer loaded at startup: ${eager}`)
        chunk.imports.forEach(walk)
      }
      Object.values(bundle)
        .filter((chunk) => chunk.type === 'chunk' && chunk.isEntry)
        .forEach((chunk) => walk(chunk.fileName))
    },
  }
}

export default defineConfig({
  plugins: [vue(), optionalRendererBoundary()],
  clearScreen: false,
  optimizeDeps: {
    esbuildOptions: { define: { PACKAGE_VERSION: JSON.stringify(mathjaxVersion) } },
    include: [
      'mathjax-full/js/mathjax.js',
      'mathjax-full/js/input/tex.js',
      'mathjax-full/js/output/svg.js',
      'mathjax-full/js/adaptors/browserAdaptor.js',
      'mathjax-full/js/handlers/html.js',
      'mathjax-full/js/input/tex/AllPackages.js',
      'mathjax-full/js/core/MmlTree/SerializedMmlVisitor.js',
    ],
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  define: {
    // MathJax uses this compile-time constant to avoid its Node-only eval(require) fallback.
    PACKAGE_VERSION: JSON.stringify(mathjaxVersion),
    'import.meta.env.PACKAGE_VERSION': JSON.stringify(packageJson.version),
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2021',
    minify: 'esbuild',
    sourcemap: false,
    rollupOptions: {
      output: {
        // Keep shared helpers out of optional renderers: otherwise the entry
        // imports Mermaid just to obtain a helper and defeats lazy loading.
        manualChunks(id) {
          if (id === '\0commonjsHelpers.js' || id === '\0vite/preload-helper.js') {
            return 'runtime'
          }
          if (id.includes('node_modules/element-plus')) {
            return 'element-plus'
          }
        },
      },
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.js'],
  },
})
