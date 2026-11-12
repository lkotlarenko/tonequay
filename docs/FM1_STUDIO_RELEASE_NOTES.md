# FM-1 Studio 0.4.0 — first native workstation build

Built on Joakim Eriksson's MIT emulator 0.4.0. Core CPU, instruction decoder and
JIT remain the existing implementation; the Studio crate adds the workstation.

- Native egui/Glow app with Wayland and X11, a slate/rose visual identity, original
  app icon, desktop metadata and an explicit desktop launcher installer.
- Firmware library with public Hub JSON, offline metadata/cache, search, favorites,
  recent/local packages, creator links and SHA-verified original release downloads.
- Live firmware LCD, matrix buttons/keys/LEDs and relative encoders, keyboard input,
  native MIDI and audio, bounded queues and clean worker ownership.
- MiniLab standard-port auto-detection, manual inputs, reconnect, MIDI inspector,
  channel routing, Learn, consume/forward profiles and opt-in WaveLoop translation.
- Exact-package flash saves/backups, identity-checked portable .fm1flash,
  confirmed reset, safe switching and transactional rejection of bad loads.
- Preferences: font size, interface scale, larger controls, high contrast, reduced
  animations and light appearance. Keyboard shortcuts and hideable side panes.
- Normal/stable/experimental-low audio queues, device selection, sample-format
  conversion, output-rate conversion, callback-aware queue sizing and honest metrics.
- Native executable and AppImage, build tasks, license notices and repeatable
  headless/desktop/lifecycle measurement scripts.

Read FM1_STUDIO_TEST_REPORT.md before treating this as a fully qualified release.
Not all requested advanced mappings, .fm1save conversion or firmware/peripheral
combinations are implemented or tested. No firmware is bundled. Physical MiniLab
performance, round-trip latency, older-glibc compatibility and bit-for-bit build
reproducibility are not claimed.
