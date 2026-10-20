// Node canvas verifies the bundled PDF.js engine, not Tauri WebView installation.
import fs from 'node:fs/promises'
import path from 'node:path'
import { createRequire } from 'node:module'
import { pathToFileURL } from 'node:url'
const require = createRequire(path.join(process.cwd(), 'package.json'))
const canvas = require('@napi-rs/canvas')
Object.assign(globalThis, { DOMMatrix: canvas.DOMMatrix, Path2D: canvas.Path2D, ImageData: canvas.ImageData })
const pdfjs = await import(pathToFileURL(require.resolve('pdfjs-dist/legacy/build/pdf.mjs')))
const root = process.argv[2]
const entries = []
const fixtures = ['normal_widget', 'rotated_ap', 'missing_ap'].map(name=>({name,file:path.join(root,'inputs',`${name}.pdf`)}))
for (const name of ['consecutive-text','winansi-text','extract-inherited-geometry']) {
  fixtures.push({name,file:path.join(root,'../pdf-engine-review-phase34-recheck-2026-10-19/outputs',`${name}.pdf`)})
}
for (const {name,file} of fixtures) {
  const data = new Uint8Array(await fs.readFile(file))
  const task = pdfjs.getDocument({data,enableXfa:true})
  const doc = await task.promise
  const page = await doc.getPage(1)
  const viewport = page.getViewport({scale:1})
  const target = canvas.createCanvas(Math.ceil(viewport.width),Math.ceil(viewport.height))
  await page.render({canvasContext:target.getContext('2d'),viewport}).promise
  await fs.writeFile(path.join(root,'renders',`${name}-pdfjs.png`), target.toBuffer('image/png'))
  const text = await page.getTextContent()
  entries.push({name,pages:doc.numPages,view:page.view,rotate:page.rotate,userUnit:page.userUnit,width:viewport.width,height:viewport.height,textItems:text.items.map(({str,width,height,transform})=>({str,width,height,transform}))})
  await task.destroy()
}
await fs.writeFile(path.join(root,'pdfjs.json'),JSON.stringify(entries,null,2))
console.log(JSON.stringify(entries,null,2))
