# Credits and licensing

## Emulator foundation

**Joakim Eriksson** created [fm1-emu](https://github.com/joakimeriksson/fm1-emu),
the MIT-licensed emulator on which Tonequay is built. The original CPU, decoder,
JIT, SoC, display, flash and audio hardware emulation remain its foundation.
The root LICENSE preserves the upstream notice. Original command-line, browser
and plugin sources remain in this repository; their presence is not a claim
that Tonequay provides newly tested releases for every upstream platform.

The alternative [simonjohansson/fm1-emulator](https://github.com/simonjohansson/fm1-emulator)
was consulted as a reference only. No QEMU implementation code was incorporated.

## Firmware and catalog

[The Firmware Hub](https://thefirmwarehub.com/) provides catalog metadata under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Tonequay attributes the
Hub and preserves author/project links. Downloads use original creator release
endpoints, with catalog SHA-256 verification. Catalog refresh is limited to
one attempt per 24 hours. Firmware authors retain their own licensing rights.

[Felucca](https://github.com/hugelton/Felucca), by Leo Kuroshita (Hügelton
Instruments) and contributors, and [WaveLoop](https://github.com/ELI7VH/Felucca),
by ELI7VH and contributors, informed compatibility testing and controller
documentation. [AMB-1](https://github.com/charlesvestal/fm1-amb), by Charles Vestal
and contributors, is also used for compatibility checks.
Test packages are downloaded separately and are not included in app releases.
Arturia’s MiniLab documentation informed the controller defaults; customized
hardware presets may use different assignments. No Arturia software is required.

## Native application dependencies

The application uses egui/eframe for UI, CPAL for audio, midir for MIDI, rtrb for
bounded audio transport, rfd for file dialogs, reqwest/rustls for HTTPS, serde for
serialization, sha2 for integrity checks, and png for icon decoding. Individual
licenses, including fonts shipped by egui, are collected from the locked Cargo
sources by `tools/studio_notices.py` into every AppImage’s license directory.
Cargo.lock records exact dependency versions.

AppImage runtime notices and corresponding source links are retained in
`packaging/studio/runtime-licenses/` and included in releases. Host graphics
drivers and glibc are not bundled. The Tonequay waveform/Q icon is original SVG
artwork supplied with this project under its MIT license.
