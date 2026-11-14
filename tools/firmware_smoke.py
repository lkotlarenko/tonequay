#!/usr/bin/env python3
"""Headless guest-note smoke using pinned original releases; never bundle fixtures."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / 'target/release/tonequay').resolve()
    fixtures = json.loads((ROOT / 'tests/studio-firmware-fixtures.json').read_text())
    cache = ROOT / 'reference/firmwares'
    cache.mkdir(parents=True, exist_ok=True)
    report = []
    for fixture in fixtures['firmwares']:
        path = cache / (fixture['id'] + '.fwsc')
        def verified(data):
            return len(data) == fixture['size'] and hashlib.sha256(data).hexdigest() == fixture['sha256']
        if not path.exists() or not verified(path.read_bytes()):
            request = urllib.request.Request(fixture['url'], headers={'User-Agent': 'Tonequay-tests/0.1'})
            with urllib.request.urlopen(request, timeout=60) as response:
                data = response.read(fixture['size'] + 1)
            if not verified(data):
                raise ValueError(f"{fixture['id']}: SHA-256/size verification failed")
            path.write_bytes(data)
        environment = dict(os.environ, FM1_BENCH_KEYS='1000:m903c64,1100:m904064,1200:m904364,2000:m803c00,2100:m804000,2200:m804300')
        result = subprocess.run([str(binary), '--bench', str(path), '3000'], env=environment, capture_output=True, text=True, timeout=90, check=True)
        measurement = json.loads(result.stdout)
        if measurement.get('nonzero_samples', 0) <= 0:
            raise ValueError(f"{fixture['id']}: no nonzero audio samples")
        report.append({'fixture': fixture['id'], 'sha256': fixture['sha256'], **measurement})
    output = ROOT / 'build/firmware-smoke.json'
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
