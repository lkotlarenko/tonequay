# Tonequay controls and local library

## Firmware folder

The library watches a configurable folder for `.fwsc` packages. The default is
`$XDG_DATA_HOME/fm1-studio/firmwares`, or `~/.local/share/fm1-studio/firmwares`.
Choose **Change** beside the folder in the library, or select a folder in
Preferences. Existing files stay where they are: choosing a folder never moves
or deletes them. The folder is checked about every two seconds in a worker.
Search includes its files. Scanning is non-recursive and excludes symlinks.
Adding or replacing a file does not interrupt the currently running firmware;
select it explicitly to boot it. `.ufw` and raw `.bin` imports are unsupported.

## Appearance

Preferences provides font size, UI scale and light/dark appearance. Sliders edit
a draft, so the interface stays stable while dragging. **Apply changes** commits
the draft and closes Preferences; Cancel discards it. Reset changes the draft.
Controls can be operated with keyboard focus and arrow keys. The instrument
scrolls when enlarged rather than squeezing the firmware display illegibly.

## Two MIDI modes

**Firmware MIDI** forwards notes and controllers to the guest according to your
channel settings. Firmware decides which messages it understands. Stock and
Felucca-family channel suggestions are defaults, not universal guarantees.

**FM-1 panel** drives the physical instrument interface instead. Its starter
keyboard range is MIDI 53–79; pad notes 36–43 on channel 10 map to panel buttons.
MiniLab-style CCs 74/71/76/77 control four encoders; 93/18/19 select additional
panel encoders, 114 navigates, and 82 controls master volume. These assignments
require the corresponding hardware preset. Use MIDI Learn to replace them.
Explicit mappings take precedence. Guest notes are not also forwarded in this
mode. Clock/transport forwarding is optional. Relative modes, absolute scaling
and pickup are configured in the mapping controls.

Sustain and overlapping input sources preserve held keys until their final
release. Panic, routing changes, switching firmware and MIDI disconnect clear
held input. The on-screen key state and MIDI inspector help diagnose mappings.
No MIDI Control Center installation is needed. WaveLoop’s custom preset is not
assumed; consult its author’s controller instructions before configuring it.

## Data compatibility

The Tonequay rebrand deliberately keeps the earlier `fm1-studio` data directory,
eframe preferences location and native `.fm1flash` backup format. Existing
favorites, mappings, cached firmware and saves therefore remain available.
Flash identity uses the exact package hash; saves are not silently shared
across incompatible firmware versions. The upstream `.fm1save` format is not
imported. See the original [guide](FM1_STUDIO.md) for audio, backup and troubleshooting.
