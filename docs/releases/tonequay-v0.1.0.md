# Tonequay 0.1.0

First public native Linux release of Tonequay, based on Joakim Eriksson’s
MIT-licensed FM-1 emulator. Tonequay runs real M-VAVE FM-1 firmware; it does not
replace the emulator with a new synthesizer.

- Native dark/light interface, original icon, adjustable font size and UI scale.
- Firmware library, verified original-source downloads and offline cache.
- Configurable firmware folder with automatic discovery of local `.fwsc` files.
- Guest MIDI and FM-1 panel control modes, MiniLab defaults, MIDI Learn and panic.
- Native audio, isolated firmware saves, backups and AppImage packaging.

## Downloads

`Tonequay-x86_64.AppImage` is the recommended Linux x86-64 download. Make it
executable before launching. Without FUSE, use
`APPIMAGE_EXTRACT_AND_RUN=1 ./Tonequay-x86_64.AppImage`.

The standalone `tonequay` executable must be distributed with
`Tonequay-Licenses.tar.gz`. `SHA256SUMS` covers all three assets.

**No firmware is bundled.** Supply your own legal `.fwsc` package or download a
community package through its creator’s original release endpoint.

## Verification and limitations

These assets come directly from successful Ubuntu 22.04 GitHub Actions
[run 43631631012](https://github.com/lkotlarenko/tonequay/actions/runs/43631631012),
commit `3397070fd7211cbe8c83ce6138d965789a38f0bd`. The corresponding
[CI run](https://github.com/lkotlarenko/tonequay/actions/runs/43631619684) also passed.
216 Rust tests, six browser regressions, strict app Clippy/formatting, release
compilation, pinned original firmware audio smokes, packaging and checksums passed.

Native and AppImage desktop smokes passed on CachyOS KDE Wayland and X11. The
Ubuntu-built AppImage was also downloaded and launched successfully on CachyOS
Wayland. Automated MIDI panel tests use a virtual ALSA source. **Physical MiniLab
playing and unplug/replug behavior, sustained audio profiles, broad distribution
compatibility and a full accessibility audit remain unverified.** This is an
early release, not a guarantee of complete hardware or firmware compatibility.

The Ubuntu native executable imports at most GLIBC_2.35. Firmware compatibility
is deliberately shown as untested rather than certified. Official proprietary
M-VAVE firmware is Bring Your Own Firmware; `.ufw`, raw `.bin` and upstream
`.fm1save` imports are unsupported. Native backups use `.fm1flash`.

See the [compatibility report](https://github.com/lkotlarenko/tonequay/blob/3397070fd7211cbe8c83ce6138d965789a38f0bd/docs/FM1_STUDIO_TEST_REPORT.md)
and [credits](https://github.com/lkotlarenko/tonequay/blob/3397070fd7211cbe8c83ce6138d965789a38f0bd/CREDITS.md).
The original emulator copyright, firmware-author credits, Firmware Hub CC BY 4.0
attribution, dependency/font licenses and AppImage runtime notices are preserved.
