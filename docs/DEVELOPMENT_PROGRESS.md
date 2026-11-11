# FM-1 Studio development progress

## Phase 1 — audit and baseline

Initial architecture audit recorded in FM1_STUDIO_PLAN.md before implementation.
Baseline tests in progress; initial sandbox registry lookup failed, approved retry
can fetch dependencies. No firmware binaries present in work/ at baseline.

## Phases 1–4 — working native application

Upstream baseline: 190 Rust tests passed; all six existing web test scripts passed.
Added fm1-studio with native egui/Glow Wayland/X11, direct shared dual-core runner,
240 MHz JIT, SFC boot parameters, worker-owned CPAL and bounded SPSC audio.
CLI behavior remains intact. Catalog and file/dialog jobs avoid the render/audio
callbacks. MIDI bypasses UI frame timing, supports auto/manual ports, Learn,
consume/forward mappings, relative encoders, pickup, WaveLoop modes and profiles.
Exact package flash isolation, atomic writes/backups, quiet-flash switching,
portable identity-checked .fm1flash import/export and confirmed reset implemented.
New parser guards reject app offsets/lengths outside the image.

Latest full workspace check: 203 Rust tests pass, including 11 Studio tests and
2 new malformed FWSC tests. Strict Studio Clippy and scoped fmt pass. Whole-tree
fmt/Clippy were attempted and fail on inherited upstream formatting/lints; no
mass formatting or unrelated CPU rewrites were made.

## Phases 5–6 — measurements and desktop validation

Three creator packages downloaded only as ignored local fixtures: Felucca
1.8.2-beta, WaveLoop midi23, AMB-1 1.3.2. SHA-256/size match Hub metadata.
Headless benchmarks with real UART chord input and upstream fm1-live comparisons
recorded under docs/measurements. First Wayland GUI successfully booted Felucca
and ran CPAL. Actual ALSA enumeration found the attached MiniLab MIDI/DIN/MCU/ALV
ports; physical playing is not asserted. Studio's own creator download path was
verified against WaveLoop's exact hash. Actual full Hub schema (193 entries)
validates, including one multi-category array entry.

Lifecycle smoke completed all nine steps: notes, export, switch, panic, switch
back, rejected missing file without losing the running session, import and save.
Concurrent stress testing exposed missing frames; callback size measurement and
a minimum queue floor of one callback plus 5 ms were added. Final isolated rerun
and final packaging results follow. A bounded ring replaced the old CLI's locking
pattern in Studio only; no claim of improved round-trip hardware latency is made.

Dependency OSV review checked 472 locked registry packages. Fixed soundness
advisories by upgrading rustix to 1.1.6 and Tokio to 1.54.1. Remaining findings
are maintenance advisories (instant, ttf-parser, zbus_names, zvariant/derive);
instant belongs to the preserved CLI graph. Not claimed fully risk-free.

## Phase 7 — packaging in progress

Reproducible build tasks, AppDir desktop/SVG icon, dependency license collection,
AppRun and AppImage script implemented. Host is CachyOS, glibc 2.44; executable
currently imports GLIBC_2.43, so older distro compatibility is not claimed.
Host lacks working FUSE; tools are run with APPIMAGE_EXTRACT_AND_RUN=1 and final
outside-tree tests will use extraction mode. No firmware is packaged.

## UI/accessibility follow-up

Implemented the requested visual refresh: original SVG/PNG FM-1 Studio brand,
embedded native window icon, slate/rose palette, larger typography, firmware cards,
framed instrument with flanking encoders, and a calmer status bar. Preferences
(F10) remembers 13–24 point font size, 80–180% scaling, large targets, high contrast,
reduced interface/scroll animation and light mode. Side panes can be hidden;
the inspector floats at narrow sizes and the device can scroll at high zoom.
Tab navigation, keyboard panel-button pulses, slider arrows and Ctrl+O/Ctrl+F
shortcuts were added. UI code separated into library, inspector, panel,
preferences and theme modules. Firmware fonts remain firmware-owned.

Desktop icon/launcher installation is an explicit preference action or
--install-desktop. Verified the installer using an isolated XDG directory and an
outside-tree AppImage path containing spaces. Native icon and desktop ID match
KDE's Wayland lookup. No user desktop files were modified during these tests.

## Packaging and latest runtime evidence

Genuine type-2 AppImage built at dist/FM1-Studio-x86_64.AppImage (about 7.4 MiB),
plus an approximately 18 MiB native executable. AppRun now replaces linuxdeploy's
symlink before writing the wrapper; an ELF/version check protects against
accidentally overwriting the staged executable. Host ALSA/PipeWire and graphics
platform libraries follow linuxdeploy exclusions. Runtime input is SHA-256 pinned
in packaging/studio/runtime.sha256 and cached under ignored build/packaging.

Outside-tree AppImage --version and explicit desktop installation passed in a
path containing spaces using extraction mode (host FUSE unavailable). Final
branded GUI and final archive checks follow. Latest completed workspace run:
204 Rust tests and all six web scripts pass; Studio strict Clippy still passes.

Normal-profile runtime smoke with requested 1024-frame buffers completed all nine
switch/save/error steps with ZERO missing output frames. Observed callbacks were
1024 frames, versus the original host default of 4410. Effective queue remained
about 50 ms after warmup. Stable/low profile hardware qualification and true
round-trip latency remain unmeasured. Latest desktop process samples were roughly
173 MiB peak RSS and 50% of one CPU core (includes full 240 MHz emulation).
