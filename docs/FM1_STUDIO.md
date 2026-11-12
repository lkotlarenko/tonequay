# FM-1 Studio

Native Linux workstation for the MIT FM-1 emulator. Studio runs the existing CPU,
JIT and SoC; it does not substitute a software synthesizer. Firmware is supplied
by you or downloaded from its creator with a matching Firmware Hub SHA-256.
No firmware, factory bank, proprietary artwork or firmware installer is bundled.

## Run

```sh
chmod +x dist/FM1-Studio-x86_64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./dist/FM1-Studio-x86_64.AppImage
# Or start with a local update package:
APPIMAGE_EXTRACT_AND_RUN=1 ./dist/FM1-Studio-x86_64.AppImage /absolute/path/Felucca.fwsc
```

The native executable is `dist/fm1-studio`. `--version`, `--devices` and
`--bench PACKAGE.fwsc 5000` work without a graphical window. AppImage systems
without FUSE can use `--appimage-extract-and-run` or `--appimage-extract` and
launch `squashfs-root/AppRun`. Running directly from Downloads is supported.

Import .fwsc using the file dialog or drag a file onto the window. M-VAVE stock
firmware is Bring Your Own Firmware. Other boards and .ufw/.bin/zip files are
not accepted by Studio. The original fm1-live and other CLI tools retain their
existing formats and behavior.

## Appearance, accessibility and desktop icon

Open **Preferences** in the header, or press **F10**. Studio remembers font size
(13–24 points), interface scale (80–180%), larger control targets, high contrast,
reduced animations and light/dark appearance. Tab/Shift+Tab move focus; arrow keys
adjust sliders; Enter/Space activate focused buttons. Ctrl+O imports firmware and
Ctrl+F focuses library search. Hide Library or Inspector from the header for more
instrument space. At narrower sizes the inspector becomes a floating window;
the instrument can scroll at large accessibility scales.

The application has an original FM-1 Studio icon embedded in its native window
and supplied as SVG/PNG desktop assets. On Wayland, KDE resolves taskbar icons
using the desktop entry, rather than an arbitrary window bitmap. Use
**Preferences → Install desktop launcher & icon**, or:

```sh
APPIMAGE_EXTRACT_AND_RUN=1 ./dist/FM1-Studio-x86_64.AppImage --install-desktop
```

This explicit action writes the launcher/icon into your XDG data directory.
The installed AppImage launcher uses extraction mode and works without FUSE.
It launches the application at its current absolute path, so move the AppImage
to its permanent location first. It installs no service and changes no firmware.
The firmware display uses the firmware's own fonts; Studio's font-size preference
does not modify emulated firmware. Scale enlarges the display with nearest sampling.

## Library and offline operation

Refresh Hub fetches the public JSON catalog, with an identifying User-Agent, at
most once per 24 hours. Failed refresh attempts count toward this limit too.
Cached metadata and packages remain usable offline. Creator release assets are
resolved through GitHub's API, or a catalog's direct original .fwsc URL. An
ambiguous/missing asset, absent digest or hash/size mismatch is an error: use a
local package rather than a Hub mirror. Some creators publish through endpoints
which this first release cannot resolve automatically.

Firmware list: [The Firmware Hub](https://thefirmwarehub.com),
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Entries show each
maker, license and original project link. Compatibility reads **untested in
Studio**; a listing is not a compatibility guarantee. No automatic network
refresh or firmware update occurs at startup. Refresh and downloads have
progress, cancellation, HTTPS-only transport, size limits and timeouts. A blocked
read can take up to the 60-second network timeout to observe cancellation.

## Playing and controllers

Studio auto-selects the MiniLab standard MIDI input, excluding MCU/HUI and ALV.
Use Controller & MIDI to choose a generic/manual port. Discovery reconnects every
second. Disconnects and queue failures request panic. MIDI delivery runs directly
from the MIDI connection to the emulator worker, independently of rendering.
Notes, velocity, CC, sustain, aftertouch, bend and real-time bytes pass to the
UART; whether each does anything is decided by the firmware. The inspector shows
input timestamps. Audio callback execution never depends on a MIDI/UI lock.

Guest channel 0 preserves input channels. Try 2 for M-VAVE stock; Felucca-family
routing commonly uses 1–4. These are configurable starting points.

Default is guest passthrough. The opt-in MiniLab user profile maps documented
CC74/71/76/77/93/18/19/16 to virtual encoders, CC114 to SELECT (relative binary
offset), and faders CC82/83/85/17 to guest CC7 on channels 1–4. Presets vary:
use Learn and inspect actual messages. Mapped inputs are consumed unless Thru is
checked. Learn accepts CCs and notes for panel buttons; relative two's complement
and binary-offset modes, absolute deltas, center pickup, inversion and dead zones
are available. Pickup waits for crossing center; it does not read firmware
parameter positions. Profiles can be imported/exported as JSON and stored for an
exact firmware. Native shift detection and arbitrary browser command bindings
are not implemented in this first release.

WaveLoop has separate opt-in modes: custom-preset passthrough, or software
translation of standard user knobs/faders into its documented custom CCs. This
translation does not fabricate pad/Shift messages. Use the author's preset or
Learn for those. See the [author's controller document](https://github.com/ELI7VH/Felucca/blob/master/controllers/MiniLab-3.md).
No Arturia MIDI Control Center is required to use standard MIDI or Learn.

The on-screen buttons, keys, LEDs and display use firmware matrix state. Drag or
scroll virtual encoders, or use their one-detent buttons. Lower on a piano key
means higher initial velocity. A/W/S/E/D/F/T/G/Y/H/U/J/K play C4–C5; Space is PLAY.
Panic releases tracked MIDI notes, sustain and all panel keys. Browser text fields
retain normal keyboard behavior.

## Saves

Data lives in `$XDG_DATA_HOME/fm1-studio` or `~/.local/share/fm1-studio`:

- `catalog.json` and `catalog-attempt`: offline metadata and refresh limit.
- `packages/<sha256>.fwsc`: verified creator packages.
- `saves/<sha256>.flash`: exact-package internal flash, with `.flash.bak` backup.

Studio waits for quiet flash for automatic/manual saves; switches finish pending
flash operations, save the old machine, discard its audio/events and boot the new
machine. A failed new package load leaves the previous session running.
Autosave is every 30 seconds and on clean shutdown. Changes still in firmware RAM
need its normal Save command first. A forced kill or power loss can lose recent
changes. Save files are written through a temporary file and atomic rename.

Export creates a `.fm1flash` backup with a format/version marker, exact firmware
SHA-256 and complete flash. Import rejects another firmware identity, backs up the
old state and restarts the emulator. Reset requires confirmation, retains a
`reset-backup`, and restarts. Browser/plugin `.fm1save` conversion is not yet
implemented; Studio never treats a raw flash file as a portable backup.

## Audio and Linux troubleshooting

CPAL uses ALSA, including PipeWire's ALSA compatibility device. Install/enable
`pipewire`, `pipewire-alsa` and an active session manager on CachyOS. The normal
profile targets 50 ms of queued audio; stable is 80 ms and experimental low is
20 ms. These exclude device/server buffering and are not measured round-trip
latencies. The effective queue floor is one observed callback block plus 5 ms;
large device callbacks can therefore increase the queue beyond the chosen target. All CPAL integer and floating-point PCM sample formats are supported. Unknown
future formats report an explicit error. Advertised device buffer ranges are used to request 512/1024/2048-frame device
buffers for low/normal/stable profiles; actual callback size is measured.
A continuous linear sample-rate converter bridges 44.1 kHz
firmware audio to the selected output rate; no guest clock changes are made.

The status reports emulated/wall-clock speed, compute headroom, queued frames
converted to milliseconds, missing output frames and first-audio boot time.
Missing frames are counted individually, not called underrun callback counts.
A bounded SPSC audio consumer performs only pops, sample conversion and atomics.
Audio device loss is retried every two seconds. Switching audio device flushes
the old audio queue. Start with Normal or Stable if low latency starves.

Wayland is preferred where available. X11 fallback uses the host X display; when
troubleshooting use `env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET ./dist/fm1-studio`.
Host graphics drivers, GL/EGL/Wayland/X11 libraries, glibc and the ALSA configuration
remain system components. Do not copy graphics drivers from another distribution.
The AppImage honors linuxdeploy’s system-library exclusions and uses the host
ALSA/PipeWire stack, including its installed ALSA adapters and configuration. A newer-distribution build is not promised to run
on older glibc distributions. See the test report for the actual build host.

## Build and validate

Install Rust stable, a C toolchain, pkgconf, ALSA development files, Wayland/X11
runtime development support, linuxdeploy, appimagetool and squashfs-tools.

```sh
make studio-release
make studio-test
make studio-appimage
```

`Cargo.lock` pins the resolved graph. `tools/build_studio_appimage.sh` stages an
AppDir, desktop file, original SVG icon, licenses and runtime libraries, verifies a SHA-256-pinned type-2 runtime, sets
SOURCE_DATE_EPOCH and creates the AppImage/native executable plus SHA256SUMS.
It uses packaging tools already installed; it never executes downloaded firmware
scripts. Reproducible instructions are provided; byte-for-byte reproducibility
across different toolchains/packaging-tool versions has not been established.

`tools/measure_studio.py` compares three local creator packages with fm1-live.
`tools/studio_desktop_smoke.py` runs a temporary-data GUI smoke on Wayland/X11.
`--runtime-smoke FIRST.fwsc SECOND.fwsc ISOLATED_DIR` exercises saves, switching,
failed loading and note cleanup. `--check-catalog FILE` validates an offline
catalog; `--fetch CATALOG ID CACHE_DIR` tests a verified creator download.
`tools/studio_security_audit.py` checks lockfile package names/versions with OSV.
The native app has no Python, webview, Electron or background-service dependency.
