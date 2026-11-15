#!/usr/bin/env python3
"""Repeatable app-only GUI review; requires a real desktop session, never contacts Hub."""
import os
from pathlib import Path
import subprocess
import sys

binary = str(Path(sys.argv[1]).resolve())
root = Path(__file__).resolve().parent.parent
cases = [
    ('v02-review-dark', {}),
    ('v02-review-light', {'FM1_STUDIO_SMOKE_LIGHT': '1'}),
    ('v02-review-compact', {'FM1_STUDIO_SMOKE_LIGHT': '1', 'FM1_STUDIO_SMOKE_WIDTH': '940', 'FM1_STUDIO_SMOKE_SCALE': '1.5', 'FM1_STUDIO_SMOKE_FONT': '20'}),
    ('v02-review-max', {'FM1_STUDIO_SMOKE_LIGHT': '1', 'FM1_STUDIO_SMOKE_WIDTH': '940', 'FM1_STUDIO_SMOKE_SCALE': '1.8', 'FM1_STUDIO_SMOKE_FONT': '24'}),
    ('v02-review-learn', {'FM1_STUDIO_SMOKE_LEARN': '1'}),
]
for label, options in cases:
    env = {k:v for k,v in os.environ.items() if not k.startswith('FM1_STUDIO_SMOKE_')}
    env.update(options, FM1_STUDIO_SMOKE_LABEL=label)
    subprocess.run([sys.executable, str(root / 'tools/studio_desktop_smoke.py'), binary], env=env, cwd=root, check=True, timeout=45)
    print(label + ': passed', flush=True)
