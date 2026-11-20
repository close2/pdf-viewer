# Round 2154 — a sound is played through a helper program

Q972's recommendation B, on round 2150's commit: `quorra` links no audio library (ADR 3317).

## What it did

- `crates/quorra-sound`: the helper program, the one that links `cpal` (an optional dependency
  behind the default `device` feature, which the binary requires); its library is the wire, the
  window's half (`Helper`) and the helper's (`serve`, generic over a `Device`). Tagged
  little-endian messages on the helper's standard input and output: `PLAY`/`STOP` in;
  `OPENED`/`FAILED` once, then `STARTED`/`REFUSED`/`ENDED` per sound; every count bounded first.
- `viewer_host::sound::helper::open` is the window's `OpenSink`: found beside the executable or
  by `QUORRA_SOUND`, started at the first sound. A pipe's end stops the helper; the window's
  close waits two seconds, then kills. Missing, loader-refused (status 127), no device or ten
  seconds silent: a sentence in the title and on the terminal, and the window goes on.
- `quorra` drops `cpal` and `quorra/sound.rs`; `tools/batch.sh` installs the helper under
  `install_featured`; `doc/stack.md`'s row and the workspace manifest's note follow.
- Tests: `quorra-sound`'s `pipe.rs` (in process with a recording device; the real program, which
  here has no device; missing; a stand-in exiting 127; its `NEEDED` `libasound`), `viewer-ui`'s
  `no_audio_library.rs` (`readelf -d` of `quorra`), `played_sounds.rs`'s missing helper.

## Proof and sizes

Release, stripped: `quorra` 41,612,064 bytes, `NEEDED` `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`
only, and no `cpal` or `alsa` symbol (the client is 23.0 KiB); ADR 3301 read 41,610,144 for
round 2150's tree, so fat LTO's layout hides the 70 KiB that left. `quorra-sound` 442,568 bytes,
`NEEDED` `libasound.so.2`. One fat-LTO build of `quorra` took 1,393 s.
## Ledger and gates

**Ledger.** No status moves; §13.3 stays `implemented`, its code list and note naming the helper.
**Gates.** fmt exit 0; clippy `-D warnings --all-targets` over `-p quorra-sound -p viewer-host -p
viewer-ui` exit 0; `pdf-sandbox`, `pdf-media-worker`, `quorra-sound` `--bins` exit 0; nextest
`--lib --bins --tests` of the three (`xvfb-run -a`): 554 run, 554 passed, 2 skipped, the
manifest's 0 expected; rustdoc exit 0; `cargo deny check` exit 0 (main's 33 duplicate warnings);
`cargo test -p conformance` exit 0 on the second run (the first: a `required-features` program
belongs in `install_featured`, and this record's draft). `fuzz` not owed: no crate it reads moved.

**Habit.** A library the linker names is a program's to carry, never a window's: move it to a helper and
read the window's dynamic section in a test.
