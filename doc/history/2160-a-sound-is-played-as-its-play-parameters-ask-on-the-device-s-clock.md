# Round 2160 — a sound is played as its play parameters ask, on the device's clock

RFC 0015 section 5, steps 6 and 7 (A946): §13.2.5's play parameters, Table 212's `/Synchronous`,
and a media clock whose master is the audio device (ADRs 3341, 3342, 3343).

## What it did

- `pdf_model::media::Playing` (`media/playing.rs`): Table 291's `/V` as a level, Table 292's
  simple duration, `/RC`, `/A`, and Table 212's `/Synchronous` and `/Mix`; `of_sound_action` reads
  Table 212 into it (negative `/Volume` silent, a stated choice; `/Repeat` present sets
  `/Synchronous` aside). It rides `Clip::playing`, `SoundData::playing` and `viewer-confined`'s wire.
- `viewer_host::sound`: `passes.rs` (the level on every sample; the simple duration's cut and silent
  freeze; `/RC`'s whole and fractional passes, zero for ever), `clock.rs` (`MediaClock`, the
  device's frame count as master, a freeze on the monotonic clock), `Player::position` and
  `resume`, `Heard::Holding` and `Heard::Paused`; `Sounds::holds_the_reader` and `HELD`.
- `quorra-sound`: `Progress`/`Report` replace the one-shot `ended`; the helper's callback reports
  frames every twentieth of a second as `PLAYED` (serial, frames).
- `quorra`, GTK and Qt: while a synchronous sound sounds, every key but Escape, every press and the
  wheel are refused, a key or press said with `HELD`.
- No person has heard any of it: this machine has no sound card.

**Premise that did not hold.** "An MH option becomes honourable now that this program is a player":
the player plays §13.3 sound objects only, which no rendition reaches (Table 285's `/D` is a file or
a form XObject), so Table 291's `/MH`/`/BE` still reach only the hand-off and ADR 1960 stands. The
brief's §12.6.4.8 is §12.6.4.9 (Sound actions); Table 307 is the movie activation dictionary.

## Ledger and gates

**Ledger.** §12.6.4.9 and §13.2.5 stay `implemented`; code, tests and notes name the player.
**Gates.** One `-p` set — `pdf-model viewer-core viewer-confined quorra-sound viewer-host viewer-ui
viewer-gtk viewer-qt` (clippy and fmt also `viewer-ffi`): fmt exit 0; clippy `-D warnings
--all-targets` exit 0; `pdf-sandbox`, `pdf-media-worker`, `quorra-sound` `--bins` exit 0; nextest
`--lib --bins --tests` under `xvfb-run -a`: 4365 run, 4362 passed, 30 skipped, 3 failed — the
manifest's two `pdf-model` audit tests and `viewer-qt`'s `unsafe_position` (Q979), nothing else;
rustdoc `-D warnings --document-private-items` exit 0; `fuzz` `cargo check --bins` exit 0;
`cargo test -p conformance` exit 0 but `records` (this record's draft), then `--test records` exit 0.
`tools/drive-windows.sh` not run.
