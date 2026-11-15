# Tonequay 0.2.0

- MIDI Learn now works directly on the instrument: enter MIDI Learn, right-click
  a knob, button, master slider or piano key, then touch the MIDI control. Repeat
  without reopening settings. Finish with Done or Esc.
- Map all 27 keyboard keys from one lowest controller key. Learned targets replace
  their previous assignment; assignment gestures are consumed without playing
  notes or triggering buttons. Relative encoder formats remain selectable.
- Panic releases notes and sustain and gates the actual audio output, including
  firmware-generated sequences and effect tails. Resume explicitly. Incoming MIDI does not bypass emergency silence. Firmware continues running; Panic does not reboot or discard saves.
- Correct the launcher icon name and install matching desktop/icon files before
  opening the window so KDE Wayland can resolve the application icon. AppImages
  include a real PNG .DirIcon as well as scalable and raster application icons.
- Theme-aware light canvas, a coherent instrument surface, a conventional piano
  keyboard and improved control grouping replace the detached-key layout.

No firmware is bundled. Existing firmware folders, mappings, favorites and saves
are retained. Reassigning a target through Learn replaces its old assignment;
whole-keyboard calibration replaces individual piano mappings while preserving
other panel mappings. The standalone executable requires its license archive.

Automated MIDI tests use a virtual ALSA source; physical MiniLab playing and
unplug/replug behavior remain unverified. Full assistive-technology and sustained
audio qualification remain pending. See the repository test report for evidence.
