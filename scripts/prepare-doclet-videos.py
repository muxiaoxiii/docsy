"""Rebuild transparent Doclet animations from the untouched Gemini green-screen videos.
Requires ffmpeg, Pillow and numpy. Run from any directory with Python 3.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'src/assets/doclet-video'
DEST.mkdir(parents=True, exist_ok=True)
# Each clip is an actual contiguous action, never a reversed gesture.
CLIPS = {
    'idle': ('22a52a2a', 0, 1.125),
    'greet': ('22a52a2a', 1.125, 2.5),
    'review': ('22a52a2a', 2.625, 3.5),
    'working': ('22a52a2a', 3.75, 4.8333333333),
    'error': ('22a52a2a', 5.125, 6),
    'celebrate': ('22a52a2a', 6.125, 7.25),
    'waiting': ('22a52a2a', 7.25, 8.75),
    'look': ('077e946b', 0, 10),
    'stretch': ('8ac082e1', .75, 3.125),
    'wink': ('8ac082e1', 4.5, 6.3333333333),
    'yawn': ('8ac082e1', 6.875, 9.375),
}
frames = {}
for video_id in sorted({clip[0] for clip in CLIPS.values()}):
    source = ROOT / f'designs/videos/gemini_generated_video_{video_id}.mp4'
    process = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(source), '-an',
        '-vf', 'fps=24,crop=720:960:0:140', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'], stdout=subprocess.PIPE)
    result = []
    while True:
        raw = process.stdout.read(720 * 960 * 3)
        if not raw:
            break
        rgb = np.frombuffer(raw, np.uint8).reshape(960, 720, 3).astype(np.float32)
        # Green dominance leaves the blue/teal scarf and pen pouch intact.
        excess = rgb[:, :, 1] - np.maximum(rgb[:, :, 0], rgb[:, :, 2])
        alpha = np.clip((80 - excess) / 68, 0, 1)
        # Neutralize green spill only where green exceeds both other channels.
        rgb[:, :, 1] = np.minimum(rgb[:, :, 1], np.maximum(rgb[:, :, 0], rgb[:, :, 2]))
        rgba = np.dstack([rgb, alpha * 255]).astype(np.uint8)
        rgba = np.pad(rgba, ((0, 0), (24, 24), (0, 0)))
        image = Image.fromarray(rgba).resize((320, 400), Image.Resampling.LANCZOS)
        array = np.array(image)
        array[:, :, 3][array[:, :, 3] < 8] = 0
        result.append(Image.fromarray(array))
    if process.wait() != 0:
        raise RuntimeError(f'Could not decode {source}')
    frames[video_id] = result

# Static pose atlas, sampled chronologically from the full look-around video.
# Runtime uses this to follow the pointer with frame changes, without decoding a video.
gaze_frames = frames['077e946b']
layout = json.loads((DEST / 'gaze-layout.json').read_text())
columns, cell_w, cell_h = layout['columns'], layout['width'], layout['height']
rows = (len(layout['poses']) + columns - 1) // columns
atlas = Image.new('RGBA', (columns * cell_w, rows * cell_h))
for index, pose in enumerate(layout['poses']):
    atlas.paste(gaze_frames[pose['frame']].resize((cell_w, cell_h), Image.Resampling.LANCZOS), ((index % columns) * cell_w, (index // columns) * cell_h))
atlas.save(DEST / 'gaze-atlas.webp', quality=85, method=5)

def animation_path(name):
    if name == 'look':
        result = ROOT / 'designs/assets/doclet-look-preview.webp'
        result.parent.mkdir(parents=True, exist_ok=True)
        return result
    return DEST / f'{name}.webp'

manifest = {'fps': 24, 'size': [320, 400], 'processing': 'green dominance alpha matte; green spill suppression; fixed crop; Lanczos resize', 'clips': {}}
proof = Image.new('RGB', (320 * 4, 440 * 3), '#f6f3e9')
draw = ImageDraw.Draw(proof)
for index, (name, (video_id, start, end)) in enumerate(CLIPS.items()):
    selected = frames[video_id][round(start * 24):round(end * 24)]
    # Encoded at 24 fps with exact total duration (41/42 ms frame times).
    durations = [round((i + 1) * 1000 / 24) - round(i * 1000 / 24) for i in range(len(selected))]
    selected[0].save(animation_path(name), save_all=True, append_images=selected[1:],
        duration=durations, loop=0 if name in ('idle', 'review', 'working', 'waiting', 'look') else 1, quality=80, method=5, minimize_size=True)
    poster = selected[len(selected) // 2]
    poster.save(DEST / f'{name}-poster.webp', quality=92, method=5)
    proof.paste(poster, ((index % 4) * 320, (index // 4) * 440), poster)
    draw.text(((index % 4) * 320 + 12, (index // 4) * 440 + 410), name, fill='#163e38')
    source = ROOT / f'designs/videos/gemini_generated_video_{video_id}.mp4'
    manifest['clips'][name] = {'source': str(source.relative_to(ROOT)), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'start': start, 'end': end, 'frames': len(selected), 'loop': 0 if name in ('idle', 'review', 'working', 'waiting', 'look') else 1, 'bytes': animation_path(name).stat().st_size}
    print(name, len(selected), manifest['clips'][name]['bytes'], flush=True)
# Assert every encoded frame retains transparent margins (no cropped ears/feet).
for name in CLIPS:
    encoded = Image.open(animation_path(name))
    assert encoded.n_frames == manifest['clips'][name]['frames']
    for index in range(encoded.n_frames):
        encoded.seek(index)
        a = np.array(encoded.convert('RGBA'))[:, :, 3]
        assert max(a[0].max(), a[-1].max(), a[:, 0].max(), a[:, -1].max()) == 0, (name, index)
(DEST / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
proof.save(ROOT / 'designs/videos/processed-preview.jpg', quality=92)
