export function mmToPt(mm) {
  return (Number(mm || 0) * 72) / 25.4
}

export function ptToMm(pt) {
  return (Number(pt || 0) * 25.4) / 72
}
