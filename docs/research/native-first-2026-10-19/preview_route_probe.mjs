// Executes the component's actual fallback functions with mocked renderers.
import fs from 'node:fs/promises'
import vm from 'node:vm'
const source = await fs.readFile('src/shared/pdf-tools/components/PdfJsPreview.vue','utf8')
function take(start,end) { return source.slice(source.indexOf(start),source.indexOf(end)) }
const functions = take('async function renderCurrentPage()', 'async function renderWithBackend(')
  + take('async function tryRenderWithBackend(', 'async function renderWithPdfJs(')
const results = []
for (const engine of ['auto','pdfjs','backend']) {
  const events = []
  const context = vm.createContext({props:{filePath:'/fixture.pdf',engine},renderSeq:0,
    errorText:{value:''},imageDataUrl:{value:''},loading:{value:false},
    cancelRender(){},isCurrent(){return true},
    async renderWithBackend(){events.push('backend');throw new Error('pdftoppm unavailable')},
    async renderWithPdfJs(){events.push('pdfjs')},emit(...args){events.push(args)},String})
  vm.runInContext(functions,context)
  await context.renderCurrentPage()
  results.push({engine,events,error:context.errorText.value})
}
await fs.writeFile(process.argv[2],JSON.stringify(results,null,2))
console.log(JSON.stringify(results,null,2))
