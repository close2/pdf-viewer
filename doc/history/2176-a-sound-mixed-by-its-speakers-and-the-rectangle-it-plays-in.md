# Round 2176 — a sound mixed by its speakers, and the rectangle it plays in

Clause 13 sound's remainder (A946, RFC 0015 section 5): ADRs 3405–3408.

## What it did

- **Speakers (ADR 3405).** `pdf_media_worker::Sound::speakers` carries a WAVE channel mask: a WAVE
  file's `dwChannelMask`, or the mask `rusty_aac`'s layout states for an AAC configuration's output
  order (a new `u32` in the run reply). `viewer_host::sound::converted` mixes by speaker on a device
  of one or two channels with ITU-R BS.775's two-channel weights (`1/√2` centre and surrounds, the
  low-frequency channel left out), a stated choice since §13.3 says nothing of how; a device of
  more channels states no positions and keeps the order, said where the mask is not the first
  speakers. The worker's channel-mask note moved to the host's sentence.
- **Markers (ADR 3406).** `played_sounds` plays a section whose `MH` `/B` is a marker over a WAVE
  file's cue point, end to end; no code changed.
- **Table 294 `/W` 3 (ADR 3407).** Honoured for a sound played here: `/B` at `/O` is painted over
  the `/AN` screen annotation's rectangle while the sound plays (default opaque black, a choice).
  `Clip::background` → `viewer_core::Screen` on `SoundData` → `Player::repainted` follows the
  player's answers → `Command::PlayingIn` → `ViewState::playing_in` → a fill after the appearance,
  on the screen only. GTK, Qt (`native_window`) and `quorra` send it; it crosses the confined wire.
- **Platforms (ADR 3408).** `quorra-sound` (cpal/WASAPI) and `pdf-media-worker` check clean for
  both Windows targets; `viewer-host` cannot cross-check here (`ring` needs a target C compiler);
  macOS's target is not installed. The C ABI's window hands sounds to its host's player as before.

**Left, in order.** A hand-off fallback in `quorra` where no helper is installed; a macOS check;
al03 filed upstream by a person; the "Rendition: stop … nothing plays in this program" sentence in
`viewer_core::interact::rendered`, stale since sounds play here.

**Ledger.** §13.2.6.1, §13.3, §13.2.4.3 stay `implemented`; notes and tests name the new work.

**Gates.** One `-p` set, `pdf-media-worker pdf-model viewer-core viewer-host viewer-qt
viewer-confined quorra-sound viewer-ui viewer-gtk`: fmt exit 0; clippy `-D warnings --all-targets`
exit 0 after five fixes of this round's own code; `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` under `xvfb-run -a`: 4 466 run, 4 466 passed, difference from the manifest empty;
rustdoc `-D warnings --document-private-items` of seven crates exit 0; `fuzz` `cargo check --bins`
exit 0; Windows `-msvc` and `-gnu` checks of `quorra-sound` and `pdf-media-worker` exit 0;
`cargo test -p conformance` exit 0 after the new test file was named in `navigation_suite`.
