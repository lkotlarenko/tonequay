# Tonequay test and compatibility report

This first working release reuses the upstream 0.4.0 emulator. The outstanding
items below are explicit qualification limits and deferred features.

## Build and tests

- Upstream baseline: 190 Rust tests passed before implementation.
- Final workspace: **216 Rust tests passed**, including 24 application tests and two
  new malformed/truncated FWSC tests. All six existing web test scripts passed.
- `cargo fmt -p tonequay --check` and Studio Clippy with strict warnings passed.
- Whole-workspace fmt and strict Clippy were also run; inherited formatting,
  dead-code and new-toolchain lint failures remain in upstream files. No broad
  CPU/CLI rewrite or mass formatting was done to conceal these failures.
- Targeted checks cover catalog string/array categories, SHA mismatch rejection,
  refresh failure rate limiting, MIDI remapping/Learn/relative encoders/pickup,
  WaveLoop translation, running-status note cleanup, save isolation and backups.
- Release executable and genuine type-2 AppImage built. AppRun's staged ELF/version
  check, archive contents, runtime/dependency/font notices and desktop entries
  were inspected. The archive contains no firmware, glibc or graphics drivers.
- Copied AppImage launched from an outside-repository path containing spaces.
  Wayland and X11 GUI/audio smokes exited successfully. Preferences, original
  brand icon, window display and controller connection were visually inspected.
- Desktop installation tested with isolated XDG data: correct quoted executable,
  icon files and desktop ID. Installed AppImage launchers use extraction mode.
  This host lacks working FUSE; mounted/default FUSE execution is not qualified.

## 2026-11-14 interface and routing revision

The Tonequay rebrand retains all data formats and existing preferences. A native
Wayland smoke loaded Felucca from the default firmware folder, rendered the new
icon and layout, connected MIDI/audio and exited successfully. New tests replay
actual egui pointer dragging to verify appearance controls keep their layout and
live style unchanged until Apply. Folder-worker tests cover add/remove/configure;
MIDI tests cover source ownership, sustain, channel precedence and panel routing.

A virtual ALSA sequencer integration smoke exercised panel notes, sustain,
encoders, master volume and a Home pad, with no held keys after release. The
headless physical-key path produced 175084 nonzero samples. This is a mock MIDI
source, not a claim of physical MiniLab playing. See `measurements/panel-mode-smoke.json`.

## Audio and firmware measurements

Felucca 1.8.2-beta, WaveLoop 1.0.5.2-midi23 and AMB-1 1.3.2 came from their
creators' original releases. Each matches the Hub catalog SHA-256 and size. They
remain ignored local fixtures and are never bundled. Five-second headless runs
included realistic UART chord notes and releases:

| Firmware | Wall time | Compute speed | Peak process RSS | Nonzero samples |
|---|---:|---:|---:|---:|
| felucca | 2.156 s | 2.32× | 69.2 MiB | 284383 |
| waveloop | 2.102 s | 2.39× | 68.9 MiB | 270392 |
| amb | 2.190 s | 2.29× | 69.0 MiB | 384815 |

Raw runs and upstream fm1-live comparisons are in `measurements/headless.json`.
The shared CPU/JIT/SoC behavior was retained; timings/audio counts were compared,
not a claim of bit-exact PCM or full hardware equivalence. CPU and memory vary
with firmware and host load. `tools/measure_studio.py` records per-child wait4
CPU time and peak RSS, sequentially.

The nine-stage native runtime smoke tested notes, backup export, firmware switch,
panic, switching back, a rejected missing-file load preserving the live session,
backup import/reboot and save isolation. The normal-buffer run recorded **zero
missing output frames**. Observed callbacks were 1024 frames, with approximately
50 ms queued after warmup. Counters reset for a new stream/session; switching
still includes a deliberate stop/reboot gap, not a zero-gap audio guarantee. Before requesting smaller device buffers, this host
used 4410-frame callbacks; a 50 ms queue was too small. A measured callback-size
floor now prevents that configuration mismatch. Early stress failures are kept
in the raw measurements rather than omitted.

Desktop samples were approximately 173 MiB peak RSS and 50% of one CPU core,
including full 240 MHz emulation. These are process samples, not system-wide CPU
percentages or GPU power measurements. First firmware DMA output is measured;
physical round-trip MIDI/audio latency is not. Stable and experimental low
profiles are available but have not received the same hardware qualification.
Non-44.1-kHz output uses a linear rate converter and is not audio-quality qualified.

## Catalog, MIDI and security

- The actual public Hub catalog validates all 193 current entries, including its
  multi-category entry. No extra Hub refresh was needed for desktop smokes.
- Studio's HTTPS worker downloaded WaveLoop from its original creator release,
  verified size/SHA-256 and parsed it before installing it in the cache.
- Actual attached MiniLab 3 standard MIDI connected successfully. Separate DIN,
  MCU/HUI and ALV ports were enumerated. **Physical key/pad/encoder performance and
  unplug/replug behavior have not been exercised**; automated note inputs are mocks.
- OSV checked 472 locked registry versions. rustix/Tokio soundness findings were
  fixed by locking rustix 1.1.6 and Tokio 1.54.1. Five remaining findings are
  maintenance advisories (instant, ttf-parser, zbus_names, zvariant/derive), not
  represented as fixed vulnerabilities. instant belongs to the preserved CLI graph.

## Compatibility and deferred requirements

Build host is CachyOS x86-64, Rust 1.99.1, glibc 2.44. The native executable imports
GLIBC_2.43. This artifact targets the specified current CachyOS environment;
older-glibc/cross-distro compatibility and byte-for-byte reproduction on different
toolchains remain unverified. Host ALSA/PipeWire and graphics integration libraries
are intentionally retained as platform components under linuxdeploy exclusions.

- Catalog entries remain marked untested in Studio. Three audio smokes do not
  certify every sound engine, peripheral, firmware UI or storage workflow.
- One unambiguous GitHub .fwsc asset or an original direct .fwsc URL is supported
  for automatic downloads; other creators need local import. HTTP timeout,
  cancellation and audio-device disappearance have not all been fault-injected.
- Studio accepts FM-1 .fwsc only; existing CLI formats and behavior are preserved.
- Identity-checked native `.fm1flash` is supported. Browser/plugin `.fm1save`
  conversion is deferred because that implementation remains coupled to plugin
  state/types; the native format is documented and never masquerades as .fm1save.
- Arbitrary browser command bindings, native Shift detection, MiniLab LED feedback,
  continuous on-screen key pressure and pickup from actual guest parameter values
  are deferred. Center pickup, Learn, consume/forward mappings and explicit
  WaveLoop preset/translation choices provide working alternatives.
- Adaptive tuning beyond callback-size protection and a full screen-reader/access
  technology audit are deferred. Font/scale/contrast/large targets/reduced motion,
  keyboard navigation and scrollable preferences are implemented.
- Proprietary M-VAVE packages remain Bring Your Own Firmware. No firmware or
  third-party firmware artwork is distributed in the application.

## Tonequay package verification (2026-11-14)

The branded native executable and AppImage were built successfully. The AppImage
launched from a temporary working directory under Wayland and X11, loaded local
Felucca firmware, displayed the watched folder and exited with status 0. Relative
SHA256SUMS verify both release files. Actionlint 1.7.12 validates the new workflows;
GitHub CI run [43630547207](https://github.com/lkotlarenko/tonequay/actions/runs/43630547207)
also passed on Ubuntu 22.04 at commit 7be7a25, including the full workspace/web
checks, release build and pinned original-release audio smokes. Cloud AppImage
packaging is qualified separately.

Measured ten-second desktop smokes peaked at 170.7 MiB RSS / 39.2% of one CPU core
for the Wayland AppImage, and 173.8 MiB / 45.9% for X11, including full emulation
and startup. These short samples are not idle-only or sustained-load guarantees.
A three-second Felucca headless run measured 2.70× compute speed and first DMA
audio at 47 ms wall time; this excludes device/physical round-trip latency.

CI pins original WaveLoop and AMB-1 release downloads with sizes and SHA-256.
The catalog-linked Felucca v1.8.2-beta release returned HTTP 404 during repository
preparation; its previously verified local fixture remains usable for testing,
but CI does not claim it can fetch that missing release. No Hub mirror is used.
New creator releases require deliberate manifest updates and license review.


Cloud AppImage workflow [43630558971](https://github.com/lkotlarenko/tonequay/actions/runs/43630558971)
passed on Ubuntu 22.04. The uploaded AppImage was downloaded and launched on
CachyOS Wayland with Felucca, audio and MIDI connected, exiting successfully.
The cloud native executable imports up to GLIBC_2.35, compared with GLIBC_2.43
for the locally built native executable. This verifies the older build baseline,
not every distribution or a physical controller performance test. The manual
workflow correctly uploads artifacts without creating a versioned release.

An empty-instrument overview smoke used 103.3 MiB peak RSS and approximately
1.95% of one CPU core, including startup and screenshot capture. No third-party
firmware artwork is used in the README overview screenshot.
