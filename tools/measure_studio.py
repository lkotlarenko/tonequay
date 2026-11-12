#!/usr/bin/env python3
"""Sequential repeatable headless measurements; no competing build should run."""
import hashlib
import json
import pathlib
import os
import tempfile
import subprocess
import time

root = pathlib.Path(__file__).resolve().parent.parent
out = root / 'docs/measurements'
out.mkdir(exist_ok=True)
measurements = []
for name in ['felucca', 'waveloop', 'amb']:
    package = root / 'reference/firmwares' / (name + '.fwsc')
    for implementation in ['studio', 'live']:
        command = [str(root / 'target/release' / ('fm1-' + implementation))]
        if implementation == 'studio':
            command += ['--bench', str(package), '5000']
        else:
            command += ['--fwsc', str(package), '--bench', '5000', '--midi', 'none']
        start = time.monotonic()
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
            proc = subprocess.Popen(command, stdout=output, stderr=errors,
                                    env={**os.environ, 'FM1_BENCH_KEYS': '1000:m903c64,1100:m904064,1200:m904364,2000:m803c00,2100:m804000,2200:m804300'})
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            output.seek(0)
            errors.seek(0)
            stdout, stderr = output.read(), errors.read()
        elapsed = time.monotonic() - start
        row = {'firmware': name, 'implementation': implementation,
               'sha256': hashlib.sha256(package.read_bytes()).hexdigest(),
               'exit_code': proc.returncode, 'wall_seconds': elapsed,
               'user_seconds': usage.ru_utime,
               'system_seconds': usage.ru_stime,
               'peak_rss_kib': usage.ru_maxrss,
               'stdout': stdout.decode(errors='replace'), 'stderr': stderr.decode(errors='replace')}
        measurements.append(row)
        print(name, implementation, proc.returncode, round(elapsed, 3), flush=True)
(out / 'headless.json').write_text(json.dumps(measurements, indent=2) + '\n')
