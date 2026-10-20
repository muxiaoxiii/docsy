import manifest from '../assets/doclet-video/manifest.json'
// URLs are bundled eagerly; animated pixels are fetched only for the visible pose.
const files = import.meta.glob(['../assets/doclet-video/*.webp', '!../assets/doclet-video/gaze-atlas.webp'], {
  eager: true,
  query: '?url',
  import: 'default',
})
export const DOCLET_MOTIONS = ['idle', 'greet', 'celebrate', 'error', 'waiting', 'working', 'review', 'look', 'stretch', 'wink', 'yawn']
export function docletVideoAsset(motion) {
  const name = DOCLET_MOTIONS.includes(motion) ? motion : 'idle'
  return {
    animation: files[`../assets/doclet-video/${name}.webp`],
    poster: files[`../assets/doclet-video/${name}-poster.webp`],
  }
}

export function docletMotionDuration(motion) {
  return Math.round((manifest.clips[motion]?.frames || 24) / manifest.fps * 1000)
}
