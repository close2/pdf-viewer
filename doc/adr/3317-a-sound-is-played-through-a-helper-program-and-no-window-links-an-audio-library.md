# 3317 — A sound is played through a helper program, and no window links an audio library

Session 2154. Status: accepted. Supersedes ADR 3301 on one point: its section "The link" — `cpal`
linked into `quorra`, and `libasound.so.2` a `NEEDED` library of the window. Context:
`doc/questions/Q972` (recommendation B), ADRs 3297, 3301, 1945, `doc/stack.md`, `CLAUDE.md`'s
startup rules.

## Decision

1. **A program of its own, `quorra-sound`** (`crates/quorra-sound`), is the one program in the tree
   that links an audio library. `cpal` is an optional dependency of the crate behind its default
   feature `device`, and the binary requires that feature; a window depends on the crate through the
   workspace entry with no default features and so links the wire and the client alone. `quorra`
   drops `cpal`.
2. **The seam is unchanged.** `viewer_host::sound::Sink` stays what the player hands each converted
   sound to. `viewer_host::sound::helper::open` is the `OpenSink` every window takes: it finds the
   helper beside the executable (or where `QUORRA_SOUND` names it), starts it, and wraps
   `quorra_sound::Helper` as the sink. The player still calls it on its own thread at the first sound
   the reader's level lets play, and never at start.
3. **The wire** (`quorra_sound::wire`): tagged little-endian messages on the helper's standard input
   and output, no frame, no version — both ends are one crate from one tree, installed together.
   The window sends `PLAY` (serial, count, `f32` samples) and `STOP`; the helper answers `OPENED`
   (rate, channels) or `FAILED` (why) once, then `STARTED` or `REFUSED` per sound, and `ENDED` when a
   sound's last sample has been written. Counts are bounded before any allocation: a sound at
   `MAX_SAMPLES` (2²⁴, which a compile-time assertion in `viewer_host::sound::helper` holds at or
   above `MAX_PLAYED_SAMPLES`), a text at 4096 bytes.
4. **Lifecycle.** Started at the first sound; a second sound and a stop are a `STOP` before the next
   `PLAY` or alone; a document's close is the player's stop; the window's close drops the player,
   which drops the sink, which closes the pipe — the helper's order to stop its sound and exit — and
   waits two seconds before killing it, so a stuck device never holds a closing window. The helper
   writes its replies from a thread of its own, so a sound's end, reported on the device's thread,
   is a channel send and never a pipe write there. It closes the descriptors it inherited first,
   for `pdf-media-worker`'s reason (ADR 1945).
5. **What is said.** A helper that is missing, that the loader refuses (exit status 127 — a machine
   without `libasound.so.2`), that cannot open the device, or that does not answer within ten seconds
   is a sentence the player puts in the title and on the terminal, and the window goes on; the next
   sound tries again. The ten and two seconds are choices: a sound server that has not answered in
   ten is one the person is better told about; two is ample for dropping a stream.

## Why not the alternatives

- **Keep the link** (Q972's A): the loader maps `libasound` at every start, and a machine without
  it cannot start the window to read a document with no sound in it — work at launch for something
  page one does not need.
- **`dlopen` it at the first sound**: `alsa-sys` names the library to the linker and offers no
  switch to do otherwise; opening a C library by hand is `unsafe`, which the window's crates do not
  admit.
- **Inside the media worker**: that process gives away the filesystem and every device before it
  reads a byte (ADR 3297); a process that opens `/dev/snd` is not one that can be confined so.

## Proof

`viewer-ui`'s `tests/no_audio_library.rs` reads `readelf -d` of the test profile's `quorra` and fails
if `libasound`, `libpulse` or `libjack` is `NEEDED`; `quorra-sound`'s `tests/pipe.rs` reads the same
section of the helper and finds `libasound` there, so the reading is one that would see it. The
release binaries' readings and sizes are in round 2154's record.

## Cost

One more crate and binary, installed beside the windows (`tools/batch.sh`'s `install_featured`,
under `device`); one process and a pipe at the first sound, a few milliseconds a person clicking a
sound does not see; a copy of each converted sound through the pipe (at most 64 MiB). Nothing at
launch.

## What GTK (step 3) needs

Only what ADR 3301 listed less the link: the same `Player`, `viewer_host::sound::helper::open` as its
`OpenSink`, its `media` handler calling `handing_here`, and a main-loop wake for winit's proxy.
