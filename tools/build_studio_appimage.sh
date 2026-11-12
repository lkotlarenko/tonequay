#!/usr/bin/env bash
# Requires Rust, ALSA dev files, linuxdeploy, appimagetool and squashfs-tools.
set -euo pipefail
cd "$(dirname "$0")/.."
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export CARGO_INCREMENTAL=0
# Packager tools may themselves be AppImages on FUSE-less hosts.
export APPIMAGE_EXTRACT_AND_RUN=1
# Host strip understands modern RELR ELF sections; the tool-bundled one may not.
export NO_STRIP=1
runtime="${FM1_APPIMAGE_RUNTIME:-$PWD/build/packaging/runtime-x86_64}"
if [[ ! -f "$runtime" ]]; then
    mkdir -p "$(dirname "$runtime")"
    curl --fail --location --max-time 60 --max-filesize 4194304 \
        https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64 -o "$runtime"
fi
python3 - "$runtime" packaging/studio/runtime.sha256 <<'PYVERIFY'
import hashlib, pathlib, sys
expected = pathlib.Path(sys.argv[2]).read_text().split()[0]
actual = hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest()
if actual != expected:
    raise SystemExit('AppImage runtime SHA-256 mismatch: supply the pinned runtime with FM1_APPIMAGE_RUNTIME; do not silently accept changed code')
PYVERIFY
cargo build --locked --release -p fm1-studio
mkdir -p dist
stage=$(mktemp -d "${TMPDIR:-/tmp}/fm1-studio-appdir.XXXXXX")
trap 'rm -rf "$stage"' EXIT
appdir="$stage/FM1-Studio.AppDir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/licenses/fm1-studio" "$appdir/usr/share/doc/fm1-studio"
install -m755 target/release/fm1-studio "$appdir/usr/bin/fm1-studio"
install -m644 LICENSE "$appdir/usr/share/licenses/fm1-studio/LICENSE"
mkdir -p "$appdir/usr/share/licenses/fm1-studio/appimage-runtime"
install -m644 packaging/studio/runtime-licenses/* "$appdir/usr/share/licenses/fm1-studio/appimage-runtime/"
install -m644 docs/FM1_STUDIO.md docs/FM1_STUDIO_PLAN.md docs/FM1_STUDIO_TEST_REPORT.md docs/FM1_STUDIO_RELEASE_NOTES.md "$appdir/usr/share/doc/fm1-studio/"
# Dependency license texts and metadata are generated from the locked crate sources.
python3 tools/studio_notices.py "$appdir/usr/share/licenses/fm1-studio/dependencies"
linuxdeploy --appdir "$appdir" --executable "$appdir/usr/bin/fm1-studio" \
    --desktop-file packaging/studio/com.fm1studio.Studio.desktop --icon-file packaging/studio/fm1-studio.svg
# Honor linuxdeploy's platform exclusions: ALSA/PipeWire, GL/EGL and display
# libraries are host integration components. Rust dependencies are linked in.
# linuxdeploy creates AppRun as a symlink to the ELF. Replace the link itself,
# never redirect through it and overwrite the staged executable.
if [[ -L "$appdir/AppRun" ]]; then unlink "$appdir/AppRun"; fi
cat > "$appdir/AppRun" <<'RUN'
#!/usr/bin/env sh
APPDIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$APPDIR/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$APPDIR/usr/bin/fm1-studio" "$@"
RUN
chmod +x "$appdir/AppRun"
readelf -h "$appdir/usr/bin/fm1-studio" >/dev/null
"$appdir/AppRun" --version
ARCH=x86_64 appimagetool --runtime-file "$runtime" --no-appstream "$appdir" "$stage/FM1-Studio-x86_64.AppImage"
mv -f "$stage/FM1-Studio-x86_64.AppImage" dist/FM1-Studio-x86_64.AppImage
install -m755 target/release/fm1-studio dist/fm1-studio.pending
mv -f dist/fm1-studio.pending dist/fm1-studio
sha256sum dist/FM1-Studio-x86_64.AppImage dist/fm1-studio > dist/SHA256SUMS.pending
mv -f dist/SHA256SUMS.pending dist/SHA256SUMS
