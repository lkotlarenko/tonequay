#!/usr/bin/env python3
"""Isolated native GUI/audio smoke with an app-only screenshot and process metrics."""
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import shutil
import time

binary = pathlib.Path(sys.argv[1]).resolve()
package = pathlib.Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else None
backend = sys.argv[3] if len(sys.argv) > 3 else 'wayland'
report_dir = pathlib.Path.cwd() / 'docs/measurements'
report_dir.mkdir(exist_ok=True)
label = os.environ.get('FM1_STUDIO_SMOKE_LABEL', backend)
with tempfile.TemporaryDirectory(prefix='fm1-studio-smoke-') as tmp:
    env = dict(os.environ)
    env.update(XDG_DATA_HOME=tmp + '/data', XDG_CONFIG_HOME=tmp + '/config',
               FM1_STUDIO_SMOKE_SECONDS='10',
               FM1_STUDIO_SCREENSHOT=str(report_dir / ('studio-' + label + '.png')))
    if backend == 'x11':
        env.pop('WAYLAND_DISPLAY', None)
        env.pop('WAYLAND_SOCKET', None)
        env['WINIT_UNIX_BACKEND'] = 'x11'
    else:
        env['WINIT_UNIX_BACKEND'] = 'wayland'
    catalog = pathlib.Path('/tmp/fm1-hub.json')
    if catalog.exists():
        dest = pathlib.Path(tmp) / 'data/fm1-studio'
        dest.mkdir(parents=True)
        (dest / 'catalog.json').write_bytes(catalog.read_bytes())
        (dest / 'catalog-attempt').write_text(str(int(time.time())))
    if package:
        local = pathlib.Path(tmp) / 'data/fm1-studio/firmwares'
        local.mkdir(parents=True, exist_ok=True)
        shutil.copy2(package, local / package.name)
    command = [str(binary)] + ([str(package)] if package else [])
    with (report_dir / ('desktop-' + label + '.log')).open('w') as log:
        start = time.monotonic()
        proc = subprocess.Popen(command, env=env, stdout=log, stderr=log, cwd=tmp)
        samples = []
        while proc.poll() is None and time.monotonic() - start < 30:
            try:
                fields = pathlib.Path(f'/proc/{proc.pid}/stat').read_text().rsplit(')', 1)[1].split()
                status = pathlib.Path(f'/proc/{proc.pid}/status').read_text()
                rss = next(int(line.split()[1]) for line in status.splitlines() if line.startswith('VmRSS:'))
                samples.append({'elapsed': time.monotonic() - start, 'cpu_ticks': int(fields[11]) + int(fields[12]), 'rss_kib': rss})
            except (FileNotFoundError, StopIteration):
                pass
            time.sleep(0.25)
        if proc.poll() is None:
            proc.terminate()
        code = proc.wait(timeout=10)
        report = {'backend': backend, 'exit_code': code, 'binary': str(binary), 'firmware': str(package),
                  'wall_seconds': time.monotonic() - start, 'clock_ticks': os.sysconf('SC_CLK_TCK'), 'samples': samples}
    (report_dir / ('desktop-' + label + '.json')).write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'samples'}))
    sys.exit(code)
