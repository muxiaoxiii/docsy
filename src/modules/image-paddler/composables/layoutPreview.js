export function safeImageWidth(images, cellWidth, imageCellHeight) {
  return Math.max(
    0.1,
    images.reduce((width, image) => {
      return Math.min(width, (imageCellHeight * image.width) / Math.max(1, image.height))
    }, cellWidth),
  )
}

export function effectiveImageWidth(requested, maximum) {
  return Math.min(Math.max(0.1, Number(requested) || 160), 500, maximum)
}

export function layoutOptionLabel(value, flow) {
  const options = {
    1: ['1 张', '1 张'],
    '1x2': ['2 张（左右）', '2 张（上下）'],
    '2x1': ['2 张（上下）', '2 张（上下）'],
    '1x3': ['3 张（左右）', '3 张（上下）'],
    4: ['4 张', '4 张（上下）'],
    '2x3': ['6 张', '6 张（上下）'],
    '3x3': ['9 张', '9 张（上下）'],
    custom: ['自定义', '自定义张数（上下）'],
  }
  return options[value]?.[flow ? 1 : 0] || value
}
