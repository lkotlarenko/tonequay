# Building Tonequay

Use x86-64 Linux with a current stable Rust toolchain, C compiler, pkg-config,
ALSA development files, and Wayland/X11/OpenGL development libraries.
On Arch/CachyOS these include `base-devel rust alsa-lib wayland libxkbcommon
libx11 libxcursor libxi libxrandr mesa`. Portal file dialogs need a working
xdg-desktop-portal backend for your desktop.

```sh
cargo build --locked --release -p tonequay
make test
```

The executable is `target/release/tonequay`. Upstream CLI builds and arguments
are unchanged. The app version is independent of the engine workspace version.

## AppImage

Install `squashfs-tools`, Python 3 and binutils, then run `make appimage`.
The builder downloads linuxdeploy, appimagetool and the runtime from official
release endpoints and checks exact sizes and SHA-256 hashes recorded in
`packaging/studio/tools.json` before executing anything. Cached verified tools
work offline. The builder also verifies the runtime against
`packaging/studio/runtime.sha256`; if the continuous runtime asset has changed,
supply the recorded runtime using `FM1_APPIMAGE_RUNTIME=/path/to/runtime`.
Never bypass its digest check. The manifest pins packaging-tool contents; updating it requires reviewing the
official release and its notices. A moved continuous asset causes a digest
failure rather than accepting new executable code.

Outputs are `dist/Tonequay-x86_64.AppImage`, `dist/tonequay`, and `dist/SHA256SUMS`.
The AppImage includes dependency notices and runtime source/license references.
It excludes firmware, glibc and host graphics drivers. SOURCE_DATE_EPOCH defaults
to the latest source commit timestamp. Locked inputs improve repeatability;
byte-for-byte reproducibility across different hosts is not yet established.

Build on the oldest supported distribution. The current CachyOS-built artifact
requires recent glibc and must not be advertised as compatible with older Linux
systems. Release CI targets Ubuntu 22.04; its output needs a successful runner
build and external smoke test before broader compatibility can be claimed.

Without FUSE: `APPIMAGE_EXTRACT_AND_RUN=1 dist/Tonequay-x86_64.AppImage --version`.
Native GUI and physical MIDI tests require a desktop session and hardware;
headless tests cannot establish those results.
