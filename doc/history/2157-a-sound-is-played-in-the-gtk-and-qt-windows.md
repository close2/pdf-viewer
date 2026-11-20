# Round 2157 — a sound is played in the GTK and Qt windows

RFC 0015 section 5, steps 3 and 4 (A946): GTK and Qt play a sound via `quorra-sound` (ADR 3329).

## What it did

- `viewer_host::sound::Sounds` (`sound/window.rs`): the `Player` made at the first sound, the
  sentence that holds the title, Escape's question and the timer's interval (`sound::LOOK`,
  250 ms while a sound is decoded or sounds). `WindowState::sounds` holds it; `answers_interval`
  and `take_the_answers` take it in; `title_notes` carries its sentence.
- `NativeWindow::plays_sounds` (default `false`, so the C ABI hands the WAVE file over as before)
  and `sound_wake` (default: nothing); `media` calls `handing_here`, and `play_here` plays a
  `Handed::Here` at once, on a `warn` or on an `ask`'s yes — `Asked::Media` now carries the id.
- GTK: `host/sounds.rs` invokes GTK's main context from the player's thread and looks at every
  window that played a sound; `close-request` ends the player. Qt: no wake (`device_lane`'s
  `unsafe` reason); `drawing_wait`'s timer, re-asked after every update, looks. Both: Escape
  stops a sounding sound before the key table; a document's close stops its own; the last
  document's close ends the player.
- Tests: `played_here.rs` in each window (GTK under `#[gtk4::test]` and Xvfb, Qt through its
  callbacks) with a shared recording sink in `tools/conformance/test-support/sounding.rs`;
  `tests/no_audio_library.rs` in each crate. No person has heard a sound through either window.

**Proof.** `readelf -d`: `quorra-gtk` names GTK's, Pango's, Cairo's and GLib's libraries and
`libgcc_s`, `libm`, `libc`; `quorra-qt` Qt's four, `libstdc++`, `libgcc_s`, `libm`, `libc`. `ldd`
finds no `libasound`, `libpulse` or `libjack` in either closure.

## Ledger and gates

**Ledger.** §13.3 stays `implemented`; its code, test and note name the two windows.
**Gates.** fmt exit 0; clippy `-D warnings --all-targets` over `-p viewer-gtk -p viewer-qt -p
viewer-host` exit 0; `pdf-sandbox`, `pdf-media-worker`, `quorra-sound` `--bins` exit 0; nextest
`--lib --bins --tests` of the three (`xvfb-run -a`): 485 run, 484 passed, 3 skipped, 1 failed —
`viewer-qt`'s `unsafe_position` sweep, failing on main since `df503353` and `75457369`, whose two
new names the round was stopped from adding: Q979 asks; the manifest's 0 expected. rustdoc exit 0;
`viewer-ffi` (three test calls gain `media`'s new argument): clippy exit 0, nextest 92 of 92;
`cargo test -p conformance` exit 0 on the second run (the first:
this record's draft). `fuzz` not owed: it reads no crate whose public items moved.

**Habit.** A wake from a worker thread into a toolkit's loop is the toolkit's; the timer that
already looks for a thread's answers is the guarantee, so a host without a safe wake needs none.
