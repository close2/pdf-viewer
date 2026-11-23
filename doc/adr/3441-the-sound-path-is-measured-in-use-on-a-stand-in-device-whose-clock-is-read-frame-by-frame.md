# 3441 — The sound path is measured in use, on a stand-in device whose clock is read frame by frame

Session 2185. Status: accepted. Context: A946 (video stays closed "until audio has shipped and its
clock and sinks are measured in use"); RFC 0015 sections 5, 6 and 9; ADRs 3297, 3317, 3342, 3349,
3381, 3433.

## Decision

1. **The instrument is a test, ignored, and a tool that runs it.**
   `crates/viewer-host/tests/audio_in_use.rs` drives the window's own `viewer_host::sound::Player`
   — the player's thread, the confined media worker, the conversion and the sound helper, exactly
   as a window starts them — and `tools/audio-in-use.sh` builds the helper and the worker in the
   `gates` profile, makes or fetches the clips and runs it. It is a test because what it measures is
   the library's API in a process of its own (`/proc/self/task/*/children` finds the worker and the
   helper it started); it is ignored and not a gate because it needs a device this file builds, ISO
   streams the tree does not hold and a quarter of an hour, and because its readings are read
   against RFC 0015's budgets by a person — a dummy device's figure is not a speaker's, and a gate
   that failed on it would be failing on the stand-in. It prints the command's readings; no document
   carries them as constants except dated beside the command (RFC 0015 section 9).
2. **The device is ALSA's `file` plugin over `null`, writing into a named pipe this file reads.**
   The VM's user is not in the `audio` group and its one card with playback devices is the RX 580's
   HDMI output with no monitor attached; `snd-dummy` and `snd-aloop` need the kernel's modules
   loaded by root, and no sound server is installed. `null` alone has no clock — a probe played
   five seconds into it in three milliseconds — so it measures nothing in time. The pipe gives it
   one: the helper's ALSA configuration (`HOME` pointed at a directory holding an `.asoundrc`, through
   a wrapper the player starts in the helper's place) makes `cpal`'s default output a `plug` to
   48 kHz stereo floats into `file` into the pipe, the pipe is shrunk to one page (512 frames,
   10.7 ms — the device's buffer), and a thread reads a millisecond's frames when they are due on
   the monotonic clock. A writer that has not supplied due frames makes the read wait: that wait,
   past 2 ms, is the underrun. What the stand-in measures is everything this program does up to the
   last byte handed to a device; what it cannot is a card's own buffer and converter, and a
   crystal's drift against the processor's, since its clock is the processor's.
3. **The samples carry their own index.** The measured sounds are 48 kHz stereo — the device's own
   format, so the conversion is the identity — and each frame's left channel is its index's low
   sixteen bits less 32 768 and its right the rest plus one, so no frame is silent and the reader
   knows which frame of the sound the device takes at each instant. A pause's resumption point, a
   lost or repeated frame, and the media clock's error against the frames heard are read off the
   frames, not inferred from the order of replies.
4. **`rustix` becomes a dev-dependency of `viewer-host`**, with its `fs` and `pipe` modules, for
   `mknodat`, the non-blocking open and `fcntl_setpipe_size` — safe functions of a crate the sandbox
   already links, so the test stays `unsafe`-free and the tree gains no package.

## Consequences

- The figures RFC 0015 section 9 now carries for audio are each a command and a dated reading, and
  `doc/reviews/2185-audio-measured-in-use.md` holds the run and what the stand-in leaves unmeasured.
- A machine with a card the user may open runs the same file against it by changing the wrapper's
  configuration; the readings that depend on the device (first frame, pause, stop) then include the
  card's buffer, and the clock's drift becomes the crystal's.
- The HE-AAC clips are ISO/IEC 14496-26's, fetched by the tool from ISO's Publicly Available
  Standards and pinned by SHA-256, never committed (the same standing as `tests/aac_conformance.rs`).
