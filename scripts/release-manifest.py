#!/usr/bin/env python3
"""Record the exact source snapshot and local build assets; never imply publication."""
import argparse
import hashlib
import json
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--out', required=True)
parser.add_argument('artifacts', nargs='*')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
def git(*argv):
    return subprocess.check_output(['git', *argv], cwd=root).decode().strip()
def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()
paths = sorted(set(git('ls-files', '--cached', '--others', '--exclude-standard').splitlines()))
sources = {name: digest(root / name) for name in paths
           if (root / name).is_file() and name not in {'REVIEW_1.0.6.md', 'release-notes.md'} and not name.startswith('release-assets/')}
artifacts = []
for name in args.artifacts:
    path = Path(name).resolve()
    artifacts.append({'path': str(path), 'bytes': path.stat().st_size, 'sha256': digest(path)})
manifest = {
    'version': json.loads((root / 'package.json').read_text())['version'],
    'status': 'local-unpublished-candidate',
    'base_commit': git('rev-parse', 'HEAD'),
    'working_tree_dirty': bool(git('status', '--porcelain')),
    'source_sha256': hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest(),
    'generated_at': datetime.now(timezone.utc).isoformat(),
    'platform': platform.platform(), 'artifacts': artifacts, 'source_files': sources,
}
out = Path(args.out)
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
print(f'{out}: {len(sources)} source files, {len(artifacts)} artifacts')
