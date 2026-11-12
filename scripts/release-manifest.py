#!/usr/bin/env python3
"""Record a source snapshot and build assets for local or release builds."""
import argparse
import hashlib
import json
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--out', required=True)
parser.add_argument('--published', action='store_true', help='Prepare a manifest for the GitHub release workflow')
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
listing = ['ls-files', '-z', '--cached']
if not args.published:
    listing += ['--others', '--exclude-standard']
paths = sorted(set(subprocess.check_output(['git', *listing], cwd=root).decode().split('\0')) - {''})
if args.published and git('status', '--porcelain', '--untracked-files=no'):
    raise SystemExit('Refusing to publish a manifest with modified tracked source files')
sources = {name: digest(root / name) for name in paths
           if (root / name).is_file() and name not in {'REVIEW_1.0.6.md', 'release-notes.md'} and not name.startswith('release-assets/')}
artifacts = []
for name in args.artifacts:
    path = Path(name).resolve()
    artifacts.append({'path': str(path), 'bytes': path.stat().st_size, 'sha256': digest(path)})
manifest = {
    'version': json.loads((root / 'package.json').read_text())['version'],
    'status': 'published' if args.published else 'local-unpublished-candidate',
    'base_commit': git('rev-parse', 'HEAD'),
    'working_tree_dirty': bool(git('status', '--porcelain', '--untracked-files=no' if args.published else '--untracked-files=normal')),
    'source_scope': 'tracked source files' if args.published else 'tracked and unignored local source files',
    'source_sha256': hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest(),
    'generated_at': datetime.now(timezone.utc).isoformat(),
    'platform': platform.platform(), 'artifacts': artifacts, 'source_files': sources,
}
out = Path(args.out)
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
print(f'{out}: {len(sources)} source files, {len(artifacts)} artifacts')
