#!/usr/bin/env python3
"""Reject release tags that disagree with the desktop app (not engine) version."""
import os
from pathlib import Path
import re

manifest = Path('crates/tonequay/Cargo.toml').read_text()
match = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', manifest, re.MULTILINE)
if not match:
    raise SystemExit('expected an explicit semantic application version')
ref = os.environ.get('RELEASE_REF', '')
if ref.startswith('refs/tags/') and ref != 'refs/tags/v' + match[1]:
    raise SystemExit(f'tag {ref} disagrees with Tonequay {match[1]}')
print('Tonequay', match[1])
