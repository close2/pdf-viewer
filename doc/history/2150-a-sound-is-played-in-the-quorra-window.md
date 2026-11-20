# Round 2150 — a sound is played in the `quorra` window

RFC 0015 section 5's second step (A946): `quorra` plays a sound's own samples, through `cpal`.

## What it did

- `viewer_core::Event::Media` carries `sound: Option<Box<SoundData>>` — Table 305's entries and the
  filtered, undecoded samples — beside the WAVE file; `viewer-confined`'s wire carries it.
- `viewer_host::handing_here`, `Handed::Here`: four `--media-files=` words, `ask` by default.
- `viewer_host::sound`: `converted` (§13.3's format, rate and channel conversion, exact rational
  positions, bounded at 2²⁴ samples), `Sink`, and `Player` — nothing until the first sound, then a
  `media-player` thread holding the worker and the sink; a second sound, a stop and either close
  end the sound, and an order arriving during a decode is read before the sound starts.
- `quorra`: the player made at the first sound, Escape stops a sound, the player's word in the
  title and on the terminal; `quorra/sound.rs` is the `cpal` sink. `cpal` 0.18.2, no default
  features, `viewer-ui` only (`doc/stack.md`, ADR 3301); `libasound.so.2` is now `NEEDED` (Q972).
- Tests: nine conversion cases from the arithmetic; `played_sounds.rs` runs a document's sound
  through the core, the level, the real worker, the conversion and a recording sink. No person has
  heard it: this machine has no sound card.

## Ledger

No status moves; §13.3 stays `implemented`, its note, code and tests naming the conversion.

## Gates

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` over `-p viewer-core -p viewer-confined
-p viewer-host -p viewer-ui` exit 0; `pdf-sandbox` and `pdf-media-worker` `--bins` exit 0; nextest
`--lib --bins --tests` of the four (`xvfb-run -a`): 1322 run, 1322 passed, the manifest's 0 expected;
rustdoc exit 0; `cargo deny check` exit 0 (main's 33 duplicate warnings); `fuzz` check exit 0;
`cargo test -p conformance` exit 0 on the second run, the first finding this record one line long.
`platform-check.sh x86_64-pc-windows-msvc` exit 101 on `ring`'s build script (no `lib.exe` here);
`--keep-going` checked `cpal` and `pdf-media-worker` clean, `viewer-host` and `viewer-ui` were not
reached. `launch_path --release` exit 101: three
`open_kinstructions` past their bands and `opt_demo`'s peak 0.08 MiB under its floor — main's
train at 5f89fb63 read the same three to within 0.33 thousand instructions.

## Habit

No device is no reason to leave its path untested: a sink trait and a recording double.
