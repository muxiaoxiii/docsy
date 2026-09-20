// Libraries are loaded only when a Markdown source actually contains rich content.
let mathEngine
let diagramEngine
let serial = Promise.resolve()
let nextId = 0

async function renderMath(source, display, context) {
  if (!mathEngine) {
    mathEngine = Promise.all([
      import('mathjax-full/js/mathjax.js'),
      import('mathjax-full/js/input/tex.js'),
      import('mathjax-full/js/output/svg.js'),
      import('mathjax-full/js/adaptors/browserAdaptor.js'),
      import('mathjax-full/js/handlers/html.js'),
      import('mathjax-full/js/input/tex/AllPackages.js'),
      import('mathjax-full/js/core/MmlTree/SerializedMmlVisitor.js'),
    ])
      .then(
        ([
          { mathjax },
          { TeX },
          { SVG },
          { browserAdaptor },
          { RegisterHTMLHandler },
          { AllPackages },
          { SerializedMmlVisitor },
        ]) => {
          RegisterHTMLHandler(browserAdaptor())
          return () => {
            const input = new TeX({
              packages: AllPackages.filter(
                (p) => !['autoload', 'require', 'html', 'noerrors', 'noundefined'].includes(p),
              ),
            })
            const output = new SVG({ fontCache: 'none' })
            return {
              // Unknown Unicode glyphs need a live document for font measurement.
              document: mathjax.document(window.document, { InputJax: input, OutputJax: output }),
              visitor: new SerializedMmlVisitor(),
            }
          }
        },
      )
      .catch((error) => {
        mathEngine = null
        throw error
      })
  }
  context.engine ||= (await mathEngine)()
  const engine = context.engine
  const node = engine.document.convert(source, { display, em: 16, ex: 8, containerWidth: 900 })
  // Use the exact compiled tree that produced the SVG, without compiling macros twice.
  const mathml = engine.visitor.visitTree(engine.document.outputJax.math.root)
  if (mathml.includes('<merror') || node.querySelector('[data-mml-node="merror"]')) {
    throw new Error('公式语法无效或包含不支持的 LaTeX 命令')
  }
  return { svg: node.querySelector('svg'), mathml, kind: 'math' }
}

async function renderDiagram(source) {
  if (!diagramEngine)
    diagramEngine = import('mermaid')
      .then((m) => m.default)
      .catch((error) => {
        diagramEngine = null
        throw error
      })
  const mermaid = await diagramEngine
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    theme: 'default',
    htmlLabels: false,
    flowchart: { htmlLabels: false, useMaxWidth: false },
    fontFamily: 'Arial, "PingFang SC", "Microsoft YaHei", sans-serif',
    suppressErrorRendering: true,
    secure: ['securityLevel', 'startOnLoad', 'maxTextSize', 'suppressErrorRendering', 'htmlLabels', 'flowchart'],
  })
  const host = document.createElement('div')
  host.style.cssText = 'position:fixed;left:-20000px;top:0;width:1200px;visibility:hidden;pointer-events:none'
  document.body.append(host)
  try {
    const { svg } = await mermaid.render(`docsy-mermaid-${++nextId}`, source, host)
    const doc = new window.DOMParser().parseFromString(svg, 'image/svg+xml')
    if (doc.querySelector('parsererror')) throw new Error('Mermaid 图片生成失败')
    return { svg: doc.documentElement, mathml: null, kind: 'mermaid' }
  } finally {
    host.remove()
  }
}

async function pngFromSvg(svg, kind) {
  if (!svg) throw new Error('未生成有效的公式或图表')
  // Never resolve remote image/font resources or foreign HTML while rasterizing.
  if (svg.querySelector('foreignObject,script,image')) throw new Error('图表包含外部图片或 HTML 标签，请改用纯文本节点')
  const box = svg
    .getAttribute('viewBox')
    ?.split(/[\s,]+/)
    .map(Number)
  if (!box || box.length !== 4 || !box.every(Number.isFinite) || box[2] <= 0 || box[3] <= 0)
    throw new Error('公式或图表尺寸无效')
  const ex = (value) => (value?.endsWith('ex') ? parseFloat(value) * 8 : parseFloat(value))
  let width = kind === 'math' ? ex(svg.getAttribute('width')) : box[2]
  let height = kind === 'math' ? ex(svg.getAttribute('height')) : box[3]
  if (!(width > 0 && height > 0)) throw new Error('公式或图表尺寸无效')
  const fit = Math.min(1, 1600 / width, 1600 / height)
  width *= fit
  height *= fit
  svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg')
  svg.setAttribute('width', String(width))
  svg.setAttribute('height', String(height))
  svg.style.maxWidth = 'none'
  svg.style.color = '#111111'
  const source = new window.XMLSerializer().serializeToString(svg)
  const image = new window.Image()
  // data URLs also work under the packaged Tauri CSP and don't need filesystem access.
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(source)}`
  await image.decode()
  const scale = 3
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, Math.ceil(width * scale))
  canvas.height = Math.max(1, Math.ceil(height * scale))
  const context = canvas.getContext('2d')
  if (!context) throw new Error('无法创建公式与图表画布')
  context.fillStyle = '#ffffff'
  context.fillRect(0, 0, canvas.width, canvas.height)
  context.drawImage(image, 0, 0, canvas.width, canvas.height)
  return { png: canvas.toDataURL('image/png'), width, height }
}

export function renderMarkdownMedia(preparation, onProgress = () => {}) {
  // Mermaid and TeX keep mutable parser state. File and paste conversions share a queue.
  const run = async () => {
    const items = []
    const context = {}
    for (const [index, request] of preparation.items.entries()) {
      onProgress(index + 1, preparation.items.length)
      try {
        const rendered =
          request.kind === 'math'
            ? await renderMath(request.source, request.display, context)
            : await renderDiagram(request.source)
        items.push({ id: request.id, mathml: rendered.mathml, ...(await pngFromSvg(rendered.svg, rendered.kind)) })
      } catch (error) {
        throw new Error(
          `第 ${index + 1} 个${request.kind === 'math' ? '公式' : ' Mermaid 图'}渲染失败：${error.message || error}`,
          { cause: error },
        )
      }
    }
    return { sourceHash: preparation.sourceHash, items }
  }
  const task = serial.then(run, run)
  serial = task.catch(() => {})
  return task
}
