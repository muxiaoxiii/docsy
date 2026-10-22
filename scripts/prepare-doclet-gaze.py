"""Build a compact gaze atlas from actual radial-turn footage, without changing the source.
Requires ffmpeg, Pillow and numpy. Route boundaries and source hash live in gaze-layout.json.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'src/assets/doclet-video'
layout = json.loads((DEST / 'gaze-layout.json').read_text())
source = ROOT / layout['source']
assert hashlib.sha256(source.read_bytes()).hexdigest() == layout['sha256'], 'Source video changed'
indices = layout['frames']
assert len(indices) == len(set(indices))
assert all(frame in indices for route in layout['routes'].values() for frames in route.values() for frame in frames)
columns, width, height = layout['columns'], layout['width'], layout['height']
atlas = Image.new('RGBA', (columns * width, ((len(indices) + columns - 1) // columns) * height))
cells = {frame: i for i, frame in enumerate(indices)}
process = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(source), '-an',
    '-vf', f"fps={layout['fps']},crop=720:960:0:140", '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'], stdout=subprocess.PIPE)
seen = set()
try:
    frame = 0
    while True:
        raw = process.stdout.read(720 * 960 * 3)
        if not raw:
            break
        if frame in cells:
            rgb = np.frombuffer(raw, np.uint8).reshape(960, 720, 3).astype(np.float32)
            excess = rgb[:, :, 1] - np.maximum(rgb[:, :, 0], rgb[:, :, 2])
            alpha = np.clip((80 - excess) / 68, 0, 1)
            rgb[:, :, 1] = np.minimum(rgb[:, :, 1], np.maximum(rgb[:, :, 0], rgb[:, :, 2]))
            rgba = np.dstack([rgb, alpha * 255]).astype(np.uint8)
            rgba = np.pad(rgba, ((0, 0), (24, 24), (0, 0)))
            image = Image.fromarray(rgba).resize((width, height), Image.Resampling.LANCZOS)
            array = np.array(image)
            array[:, :, 3][array[:, :, 3] < 8] = 0
            a = array[:, :, 3]
            assert max(a[0].max(), a[-1].max(), a[:, 0].max(), a[:, -1].max()) == 0, frame
            i = cells[frame]
            atlas.paste(Image.fromarray(array), ((i % columns) * width, (i // columns) * height))
            seen.add(frame)
        frame += 1
    assert process.wait() == 0, 'Video decoding failed'
    assert seen == set(indices), 'Missing source frames'
finally:
    if process.poll() is None:
        process.terminate()
        process.wait()
atlas.save(DEST / 'gaze-atlas.webp', quality=85, method=5)
print(f'Gaze atlas: {len(indices)} poses, {atlas.width}x{atlas.height}, {(DEST / "gaze-atlas.webp").stat().st_size} bytes')
