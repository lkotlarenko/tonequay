#!/usr/bin/env python3
"""Fetch only manifest-pinned packaging executables; never run unverified bytes."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def fetch():
    destination = ROOT / 'build/packaging/pinned'
    destination.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((ROOT / 'packaging/studio/tools.json').read_text())
    for name, spec in manifest.items():
        target = destination / name
        if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest() == spec['sha256']:
            print(f'{name}: verified cached file')
            continue
        if not spec['url'].startswith('https://github.com/'):
            raise ValueError('packaging tool URL must use GitHub HTTPS')
        request = urllib.request.Request(spec['url'], headers={'User-Agent': 'Tonequay-build/0.1'})
        pending = target.with_suffix('.pending')
        try:
            with urllib.request.urlopen(request, timeout=60) as response, pending.open('wb') as output:
                digest = hashlib.sha256()
                size = 0
                while chunk := response.read(65536):
                    size += len(chunk)
                    if size > spec['size']:
                        raise ValueError(f'{name}: download exceeds pinned size')
                    digest.update(chunk)
                    output.write(chunk)
            if size != spec['size'] or digest.hexdigest() != spec['sha256']:
                raise ValueError(f'{name}: pinned SHA-256/size mismatch; review upstream before updating tools.json')
            pending.chmod(0o755)
            os.replace(pending, target)
        finally:
            pending.unlink(missing_ok=True)
        print(f'{name}: downloaded and verified')


if __name__ == '__main__':
    fetch()
