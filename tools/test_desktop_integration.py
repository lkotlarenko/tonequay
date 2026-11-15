#!/usr/bin/env python3
"""Exercise actual launcher installation with isolated XDG data and a spaced path."""
import configparser
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='tonequay-desktop-') as directory:
    root = Path(directory)
    executable = root / 'Application with spaces' / 'tonequay'
    executable.parent.mkdir()
    shutil.copy2(binary, executable)
    env = dict(os.environ, XDG_DATA_HOME=str(root / 'data'))
    env.pop('APPIMAGE', None)
    subprocess.run([str(executable), '--install-desktop'], env=env, check=True)
    entry_path = root / 'data/applications/tonequay.desktop'
    entry = configparser.ConfigParser(interpolation=None)
    entry.read(entry_path)
    desktop = entry['Desktop Entry']
    assert desktop['Icon'] == 'tonequay'
    assert desktop['StartupWMClass'] == 'tonequay'
    assert desktop['Exec'] == f'"{executable}" %f'
    for kind in ['256x256/apps/tonequay.png', 'scalable/apps/tonequay.svg']:
        assert (root / 'data/icons/hicolor' / kind).stat().st_size > 0
    subprocess.run(['desktop-file-validate', str(entry_path)], check=True)
print('Desktop icon/identity/launcher integration passed')
