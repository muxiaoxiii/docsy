export const DOCLET_CLICK_MOTIONS = ['greet', 'celebrate', 'wink', 'stretch', 'yawn', 'review', 'working', 'error', 'waiting']
export function randomDocletMotion(previous, random = Math.random) {
  const choices = DOCLET_CLICK_MOTIONS.filter(motion => motion !== previous)
  return choices[Math.min(choices.length - 1, Math.floor(random() * choices.length))]
}
