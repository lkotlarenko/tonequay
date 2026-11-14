#!/usr/bin/env python3
"""Collect exact locked dependency metadata and existing license files for release."""
import json
import pathlib
import subprocess
import sys

out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
metadata = json.loads(subprocess.check_output([
    'cargo', 'metadata', '--locked', '--offline', '--format-version', '1',
    '--filter-platform', 'x86_64-unknown-linux-gnu'
]))
packages = {p['id']: p for p in metadata['packages']}
nodes = {n['id']: n for n in metadata['resolve']['nodes']}
root = next(p['id'] for p in packages.values() if p['name'] == 'tonequay')
visited = set()
def visit(key):
    if key in visited:
        return
    visited.add(key)
    for dep in nodes[key]['dependencies']:
        visit(dep)
visit(root)
notices = []
for key in sorted(visited):
    p = packages[key]
    notices.append({k: p.get(k) for k in ['name', 'version', 'license', 'repository', 'source']})
    base = pathlib.Path(p['manifest_path']).parent
    dest = out / (p['name'] + '-' + p['version'])
    for file in base.rglob('*'):
        name = file.name.lower()
        license_name = any(word in name for word in ['license', 'copying', 'notice', 'copyright'])
        font_notice = 'fonts' in file.relative_to(base).parts and file.suffix.lower() == '.txt'
        if file.is_file() and not file.is_symlink() and (license_name or font_notice):
            if file.suffix.lower() not in ['', '.txt', '.md', '.html']:
                continue
            target = dest / file.relative_to(base)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(file.read_bytes())
(out / 'metadata.json').write_text(json.dumps(notices, indent=2) + '\n')
