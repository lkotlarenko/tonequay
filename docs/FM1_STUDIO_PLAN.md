# FM-1 Studio architecture and compatibility audit

Historical initial audit, before the Tonequay rebrand. For the implemented
application see [ARCHITECTURE.md](ARCHITECTURE.md).

Inspected checkout: upstream workspace 0.4.0, MIT (Joakim Eriksson), clean at start.

## Existing implementation

- fm1-core contains the ISA executor and x86-64/AArch64/Wasm JITs. Retain unchanged.
- fm1-soc supplies peripherals, 240×240 RGB565 LCD, flash, FWSC unpacker, UART MIDI,
  stock boot parameters and shared dual-core scheduling (`machine::run_dual/run_blocks`).
- fm1-live is a monolithic minifb desktop/benchmark program. Default is 240 MHz;
  audio producer targets 50 ms with a 60 ms startup lead. Its CPAL callback locks
  a VecDeque and accepts only f32. Preserve CLI and regression behavior.
- fm1-plugin has an egui editor, worker-owned emulator, rtrb audio and portable
  .fm1save support. It is a separate workspace with nih-plug git dependencies;
  Studio must not pull plugin APIs into its standalone dependency graph.
- Existing web catalog is generated from Hub metadata but package URLs point at
  Hub mirrors. Do not use those URLs for Studio downloads. Existing fetch tool
  follows the web behavior; leave it intact.

## Studio boundaries

Add fm1-studio using eframe/egui with native Wayland and X11, direct core/SoC use,
CPAL and midir. UI, runtime, audio, MIDI, catalog/downloads and persistence are
separate modules. Worker exclusively owns CPU/SoC; bounded command and display
queues; audio callback owns SPSC consumer and does no locks/I/O/allocation.
Maintain accurate 240 MHz scheduling and preserve both cores and SFC boot setup.
Save full flash under exact package SHA-256 identity, atomically, with backups.
No firmware or third-party artwork is bundled.

## Research (2026-11-11)

Read README.md, PLAN.md, manifests, core/SoC/live/plugin sources, tests,
web/firmwares.json, tools/hub_catalog.mjs and tools/fetch_firmwares.mjs.
Primary references inspected: https://thefirmwarehub.com/, /kb/, /catalog.json;
https://github.com/ELI7VH/Felucca ; Arturia MiniLab 3 General Questions;
https://docs.appimage.org/packaging-guide/ .
Catalog actual root: license, updated, firmwares, smk37, creators, stock.
Firmware records include id, kind, author, repo, what, lic, version, pkg, size,
sha256, release, tag and sometimes asset. `pkg` is a mirror path; `release` may
be a page rather than an asset. Resolve creator release assets and require
matching catalog SHA-256; never guess a mirror fallback. CC-BY-4.0 list credit,
maker/project links and once-per-24h refresh are required. Persist refresh attempt
time even on errors. Cached catalog supports offline use.

Alternative QEMU repository reviewed only as reference; no code copied. The
current root LICENSE retrieved from its main branch says GPL v3 (the request
called it GPL-2.0); individual QEMU components may differ. Neither is mixed in.
MiniLab presets vary; general profile must be opt-in, with MIDI Learn and explicit
consume/forward rules. WaveLoop's custom preset requires dedicated verification.

## Risks / release gates

Untrusted parser has unchecked indexing in some internal paths; audit bounds and
exercise malformed inputs before accepting packages. JIT is native executable
code and firmware must remain emulated; never run package scripts. No .bin/.ufw
support until validated. Existing .fm1save is versioned JSON; native full-flash
saves must not masquerade as compatible backups. Hardware MiniLab latency and
hotplug, physical audio, KDE Wayland/X11, AppImage outside-tree execution and
cross-distro runtime compatibility need separate evidence. No invented CPU or
latency measurements. Compile/tests/build outputs and outstanding requirements
are tracked in DEVELOPMENT_PROGRESS.md.

Baseline started with `cargo test --workspace --locked`; sandbox DNS blocked the
registry, retried with network approval. Results pending at time of initial audit.
