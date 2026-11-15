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

## Phase 8 — delivered first working release

Final workspace run passes all 204 Rust tests. Studio scoped fmt and strict Clippy
pass; all six web regression scripts pass. Release/AppImage and copied outside-tree
Wayland/X11 GUI tests pass, including the scrollable branded Preferences screen.
Actual MiniLab standard port connects; physical playing remains unverified.
Normal-buffer lifecycle reports zero missing frames. The final report lists
remaining feature/qualification limits explicitly rather than claiming full coverage.

AppImage packaging also includes runtime and embedded-font licenses, checks the
staged ELF, and replaces finished artifacts atomically so rebuilding while an old
release is running cannot corrupt it. Desktop entries validate, filenames with
spaces work, and checksums are recorded in dist/SHA256SUMS. Launcher installation
was tested in a temporary XDG root, preserving the user's actual desktop settings.
Source, build tasks, installation/troubleshooting instructions, release notes and
raw measurements are included. No firmware binaries are tracked or packaged.

## 2026-11-14 — interaction, MIDI-panel and local-folder revision

User reported accessibility sizing jitter and requested a cleaner interface,
physical FM-1 panel routing from MIDI, and a configurable persistent firmware
folder. Implemented draft/apply appearance editing (including a multi-frame egui
pointer-drag regression), a compact toolbar and tabbed controls, finite-width
instrument layout, focusable fine-grained encoders, and virtualized library rows.
Library rendering no longer scans the filesystem or clones the full catalog.

Added explicit Firmware MIDI / FM-1 panel modes. Panel mode converts notes into
velocity-sensitive matrix keys, pad notes into buttons, standard MiniLab CCs into
encoders/master, and supports sustain/pressure/clock. Explicit Learn assignments
override presets. Guest note duplication is prevented. Input ownership merges
keyboard/mouse, timed taps and MIDI holds, so releasing one source cannot cut off
another. Configuration changes serialize panic with MIDI event delivery.
Fixed the previous u16 overflow in master CC scaling and stale queued LCD frames.
Device enumeration is now asynchronous and MIDI shutdown no longer waits on a
polling sleep. A latest-value display mailbox avoids stale frame backlog.

A configurable, non-recursive firmware folder is scanned on a dedicated worker.
New/removed .fwsc files appear automatically; scans do not execute files, follow
symlinks or reload a live firmware. Default: the data directory's firmwares/
subdirectory. Folder selection persists; local list and search are integrated.

24 Studio tests and strict scoped Clippy pass at this checkpoint. Full workspace,
native GUI and virtual-ALSA/CPAL integration checks are in progress. Rebrand and
professional GitHub/release preparation are queued after native validation, as
requested; no new remote has been selected or pushed.


## 2026-11-14 — Tonequay rebrand and repository preparation

Completed the interface/panel/folder revision: 216 workspace Rust tests and six
web regressions pass, application formatting and strict Clippy pass. Native
Wayland and virtual ALSA/CPAL panel smoke checks passed. Physical playing and
unplug/replug testing remain unverified. Accessibility draft editing has a real
egui input regression test, and the firmware folder is visible in the native UI.

Rebranded the app/package/executable to Tonequay 0.1.0; upstream engine versions,
MIT attribution, data paths, preferences and native save identity remain intact.
Added an original waveform/Q icon, desktop identity, project README, user/build/
release guides, credits, contribution/security/support policies, templates and
Dependabot. The user created https://github.com/lkotlarenko/tonequay; its existing
branch will be inspected before any push. The upstream remote is preserved.

Archived inherited plugin/browser/catalog workflows rather than running them in
the desktop fork. New CI scopes strict app lints while testing the workspace.
Tagged AppImage automation prepares draft releases, uses least-privilege tokens,
SHA-pinned actions and digest/size-pinned official packaging tools. CI runner and
older-glibc compatibility remain unverified until an actual runner completes.
Final branded packaging succeeded. The AppImage passed outside-tree Wayland and
X11 smokes (exit 0), and relative checksums verify the native and AppImage files.
Actionlint validates both workflows; pinned WaveLoop/AMB-1 guest-note smokes pass.
The original Felucca catalog release now returns 404, so that CI fixture is not
silently substituted from a mirror. Cached local Felucca tests remain valid.
The new remote contains an upstream master branch; publishing uses a separate
main branch to preserve it. No version tag or public release is created yet.


First GitHub runner execution found a new Rust 1.100 Clippy range-loop lint in
MIDI cleanup, while local Rust 1.99.1 passed. Converted the traversal to iterators
without changing note-release order and pinned CI/release Rust to the qualified
1.99.1 toolchain. The original failure is retained in Actions history. Main is
now the repository default; upstream master is preserved and issues are enabled.


GitHub CI run 43630547207 passed on Ubuntu 22.04 at application commit 7be7a25:
qualified Rust install, strict app formatting/Clippy, workspace/web regressions,
desktop/script checks, release build, pinned original WaveLoop/AMB-1 audio smokes
and native --version. The cloud AppImage workflow is being verified separately.
The final local AppImage also passed virtual ALSA/CPAL panel smoke. A separate
license archive now accompanies the standalone native executable, and checksums
cover all three distribution assets. Added current module/thread architecture
and exact upstream baseline provenance rather than rewriting the historical audit.


Cloud AppImage workflow run 43630558971 completed successfully on Ubuntu 22.04:
packaging-tool digest checks, AppImage creation, outside-tree --version, release
checksums, original firmware audio smokes and artifact upload all passed. Manual
workflow dispatch correctly skipped draft-release creation (no version tag).

The Ubuntu-built cloud AppImage was downloaded and passed a native CachyOS
Wayland firmware/audio/MIDI smoke (exit 0). Its binary imports at most GLIBC_2.35.
The final packaging update adds the standalone license archive and includes the
current architecture guide. No public release/tag is created; real hardware
playing/hotplug, sustained audio profiles and a full accessibility audit remain
explicit qualification limits.


## 2026-11-14 — automatic public releases

The user explicitly requested publishing the latest successful Actions artifact
and automatic public releases after successful AppImage runs. Final revision
3397070 passed both CI (43631619684) and AppImage build (43631631012). Preparing
v0.1.0 from those exact artifacts. Updated automation: version tags publish normal
releases; manual runs publish unique prereleases without changing latest stable.
Uploads finish in a temporary draft before automatic publication. Failed builds
never reach the publish job, and published stable assets are not overwritten.


Published https://github.com/lkotlarenko/tonequay/releases/tag/v0.1.0 using exactly
the four artifacts from Actions run 43631631012. Verified remote uploaded asset
SHA-256 values, sizes and source commit before publication; no local binary was
substituted. Added automatic publication for future successful AppImage runs.
Final release-document housekeeping distinguishes historical Studio 0.4 notes
from Tonequay 0.1.0 and packages the actual public notes. Artifact upload supports
workflow reruns; published release assets remain immutable. Automatic preview
publication is undergoing a live workflow test on this final revision.


Automatic publication verified end to end: run 43632835803 at cf1c135 passed the
build and publish jobs using GitHub's built-in token, creating the public
prerelease preview-43632835803-1-cf1c135c with all four expected assets. The
corresponding CI run 43632782869 also passed. GitHub's latest-release endpoint
still returns v0.1.0, confirming previews do not replace the stable download.
The published stable SHA256SUMS was downloaded without authentication and matched
the exact verified Actions manifest. Replaced an earlier in-progress preview test
with the final revision and canceled the redundant tag build from initial manual
publication; neither canceled run is reported as a passed test.


## 2026-11-15 — direct learning, emergency silence and desktop polish (0.2.0)

Replaced target-dropdown Learn with instrument-first interaction: a header toggle,
right-click selection on every knob/button/piano key/master, silent gesture capture,
repeatable target assignment and one-note calibration of the whole keyboard.
Assignments replace old target bindings, relative modes are retained and absolute
encoder calibration survives leaving Learn. Added real egui secondary-click and
routing regressions, including overlapping keyboard/pad channels. Learn mode
suppresses guest/host performance actions until finished.

Panic now gates CPAL output after releasing notes/keys/sustain, so firmware-run
sequences and effect tails are silent too. Resume restores output;
the emulator continues at its original clock. Added a callback gate test and
extended the real ALSA/CPAL integration smoke to verify gate frames and Resume.

Fixed the stale Icon=fm1-studio launcher reference. Normal startup registers the
matching tonequay desktop entry and PNG/SVG icons before Wayland creates the
window; unchanged files are not rewritten. AppImage root .DirIcon is a real PNG.
Added isolated desktop installer validation to CI. Light-theme screenshot review
also found a clipped sidebar path row over the renderer's dark default clear
color; replaced fixed text/button sizing and made the renderer background opaque
and theme-aware. Added a regression for the light clear color.

Redesigned the instrument surface and piano keyboard, including overlap-safe black
key hit testing. At high scaling/narrow widths Library and Controls use full-width
views instead of squeezing the instrument; normal sidebar preferences remain.
Final native, themed/compact GUI, packaging and release checks are in progress.


Completed 229 workspace tests (37 app), scoped strict Clippy and six web tests.
Live virtual ALSA/CPAL captured real callback Learn events, played the new binding,
gated 941 output frames with Panic, resumed and released all keys. Desktop startup
and spaced-path installer tests passed; the tested AppImage was also registered
in the user's actual KDE applications directory with Icon=tonequay. AppImage
contains raster .DirIcon plus hicolor PNG and scalable SVG.

Native screenshot review identified remaining enlarged-toolbar issues. Wrapped
layouts must use available_size_before_wrap, rather than full-row available_width,
to place atomic controls and right-side spacing. Added a measured toolbar helper,
font-aware full-width navigation breakpoints and bounded compact encoder grids.
Learning highlights only hovered/armed targets, including a dedicated click layer
for sliders. Final screenshots and cloud release validation follow these fixes.


The corrected five-case native review passed after inspecting both themes and
maximum scaling; toolbar controls now wrap without overlap and font-aware
navigation preserves useful instrument width. Release AppImage passed X11
firmware/audio smoke and startup icon checks. Its real callback integration
verified learning, 1412 silenced output frames, Resume and no held keys.
Embedded .DirIcon was extracted and verified as a non-symlink PNG. The final
Panic regression also drains old MIDI UI feedback while holding the routing lock,
preventing stale held-key highlights from reappearing after Panic. Pending Learn
is canceled on firmware load or mapping import. Preparing the verified 0.2.0
source and automatic release; physical MiniLab playing remains unverified.


Emergency output control now uses one atomic shared by Runtime and every CPAL
stream, independent of the command queue and emulation/file I/O. Panic and Resume
cannot be dropped by queue saturation; a full-queue unit test observes the same
callback flag. Panic latches until explicit Resume, so incoming note streams do
not reopen audio. Shutdown closes the gate before joining the worker, and command
processing is bounded per emulation iteration. Added ControlSignals to keep
thread ownership explicit. These changes preserve the CPU/JIT and normal audio
samples. Final local build/CPAL and GitHub release checks follow.


Release preparation found inherited local v0.2.0 pointing to upstream engine
commit 7856393, not the Tonequay app. It is preserved. New app releases use the
`tonequay-v` namespace, beginning tonequay-v0.2.0. The workflow and version guard
accept namespaced tags; legacy app v-tags remain compatible. This avoids replacing
upstream tags or accidentally building old upstream plugin workflows.
