#!/usr/bin/env python3
"""Check the locked dependency graph against OSV; network required, no fixes applied."""
import json
import pathlib
import urllib.request
import tomllib

lock = tomllib.loads(pathlib.Path('Cargo.lock').read_text())
packages = [p for p in lock['package'] if p.get('source', '').startswith('registry+')]
results = []
for offset in range(0, len(packages), 100):
    batch = packages[offset:offset + 100]
    body = json.dumps({'queries': [{'package': {'name': p['name'], 'ecosystem': 'crates.io'},
                                   'version': p['version']} for p in batch]}).encode()
    request = urllib.request.Request('https://api.osv.dev/v1/querybatch', data=body,
                                     headers={'Content-Type': 'application/json', 'User-Agent': 'FM1-Studio audit'})
    with urllib.request.urlopen(request, timeout=60) as response:
        findings = json.load(response)['results']
    for package, finding in zip(batch, findings):
        if finding.get('vulns'):
            results.append({'name': package['name'], 'version': package['version'], **finding})
print(json.dumps({'checked_packages': len(packages), 'findings': results}, indent=2))
