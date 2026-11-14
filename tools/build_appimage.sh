#!/usr/bin/env bash
# Requires Rust, ALSA development files, Python 3, binutils and squashfs-tools.
set -euo pipefail
cd "$(dirname "$0")/.."
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export CARGO_INCREMENTAL=0
# Packager tools may themselves be AppImages on FUSE-less hosts.
export APPIMAGE_EXTRACT_AND_RUN=1
# Host strip understands modern RELR ELF sections; the tool-bundled one may not.
export NO_STRIP=1
python3 tools/fetch_packaging_tools.py
export PATH="$PWD/build/packaging/pinned:$PATH"
runtime="${FM1_APPIMAGE_RUNTIME:-$PWD/build/packaging/pinned/runtime-x86_64}"
python3 - "$runtime" packaging/studio/runtime.sha256 <<'PYVERIFY'
import hashlib, pathlib, sys
expected = pathlib.Path(sys.argv[2]).read_text().split()[0]
if hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest() != expected:
    raise SystemExit('AppImage runtime SHA-256 mismatch')
PYVERIFY
cargo build --locked --release -p tonequay
mkdir -p dist
stage=$(mktemp -d "${TMPDIR:-/tmp}/tonequay-appdir.XXXXXX")
trap 'rm -rf "$stage"' EXIT
appdir="$stage/Tonequay.AppDir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/licenses/tonequay" "$appdir/usr/share/doc/tonequay"
install -m755 target/release/tonequay "$appdir/usr/bin/tonequay"
install -m644 LICENSE CREDITS.md "$appdir/usr/share/licenses/tonequay/"
mkdir -p "$appdir/usr/share/licenses/tonequay/appimage-runtime"
install -m644 packaging/studio/runtime-licenses/* "$appdir/usr/share/licenses/tonequay/appimage-runtime/"
install -m644 docs/FM1_STUDIO.md docs/FM1_STUDIO_PLAN.md docs/FM1_STUDIO_TEST_REPORT.md docs/FM1_STUDIO_RELEASE_NOTES.md docs/USER_GUIDE.md docs/BUILDING.md "$appdir/usr/share/doc/tonequay/"
# Dependency license texts and metadata are generated from the locked crate sources.
python3 tools/studio_notices.py "$appdir/usr/share/licenses/tonequay/dependencies"
linuxdeploy --appdir "$appdir" --executable "$appdir/usr/bin/tonequay" \
    --desktop-file packaging/studio/tonequay.desktop --icon-file packaging/studio/tonequay.svg
# Honor linuxdeploy's platform exclusions: ALSA/PipeWire, GL/EGL and display
# libraries are host integration components. Rust dependencies are linked in.
# linuxdeploy creates AppRun as a symlink to the ELF. Replace the link itself,
# never redirect through it and overwrite the staged executable.
if [[ -L "$appdir/AppRun" ]]; then unlink "$appdir/AppRun"; fi
cat > "$appdir/AppRun" <<'RUN'
#!/usr/bin/env sh
APPDIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$APPDIR/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$APPDIR/usr/bin/tonequay" "$@"
RUN
chmod +x "$appdir/AppRun"
readelf -h "$appdir/usr/bin/tonequay" >/dev/null
"$appdir/AppRun" --version
ARCH=x86_64 appimagetool --runtime-file "$runtime" --no-appstream "$appdir" "$stage/Tonequay-x86_64.AppImage"
mv -f "$stage/Tonequay-x86_64.AppImage" dist/Tonequay-x86_64.AppImage
install -m755 target/release/tonequay dist/tonequay.pending
mv -f dist/tonequay.pending dist/tonequay
(cd dist && sha256sum Tonequay-x86_64.AppImage tonequay) > dist/SHA256SUMS.pending
mv -f dist/SHA256SUMS.pending dist/SHA256SUMS
