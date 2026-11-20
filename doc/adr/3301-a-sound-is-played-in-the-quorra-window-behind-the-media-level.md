# 3301 — A sound is played in the `quorra` window, behind the `Media` level

Session 2150. Status: accepted. Context: `doc/questions/A946` and A254, RFC 0015 sections 4.2, 4.6
and 5 (step 2), ADRs 3201, 3202, 3297, `doc/stack.md`.

## Decision

RFC 0015 section 5's second step: the `quorra` window plays a sound object's own samples itself.

1. **What crosses from the core.** `viewer_core::Event::Media` gains `sound: Option<Box<SoundData>>` (boxed, so a field most clips leave empty does not widen every event):
   Table 305's `/R`, `/C`, `/B`, `/E` and the sound object's data after Table 5's filters, *undecoded*,
   beside the RIFF WAVE file the hand-off has carried since ADR 2398. The samples are the
   document's bytes, so they are decoded only in the confined media worker (ADR 3297); the core
   hands them over as they stand. `viewer-confined`'s wire carries them, so `quorra-confined`
   loses nothing when its turn comes.
2. **The level's shape.** `MediaFiles` keeps its four words and `ask` by default. RFC 0015 section
   4.2 asked that playing here be "a second answer of the *same* level"; it is the *route* the
   level's answer takes in a window that plays: `viewer_host::handing_here` sends a clip carrying
   samples to `Handed::Here` and every other clip to `handing` as before. `refuse` plays nothing and
   says so, `ask` asks "Play this sound here?" in words that say no program is started, `warn`
   plays and says so afterwards, `open` plays. Nothing is checked before the level on this route,
   because nothing is handed to a program and no name or URL is followed.
3. **The conversion** (`viewer_host::sound::converted`), §13.3's "convert between formats, downsample
   rates, and combine channels as necessary": a top-aligned `i32` becomes `sample / 2³¹`; channel
   `c` lands on device channel `c mod m`, averaged where several land, and a device with more
   channels repeats the sound's `d mod n`; a slower device averages the input frames each output
   frame covers, a faster one interpolates on the straight line, at exact rational positions. The
   clause names no channel positions, so the arrangement is a **choice**, stated in the module: it
   puts left and right on the device's first two channels, WAVE's and ALSA's order alike. Bounded at
   `MAX_PLAYED_SAMPLES` (2²⁴ samples, 64 MiB) before anything is allocated.
4. **Threads.** `viewer_host::sound::Player` starts nothing when made, and `quorra` makes it only at
   the first sound. The first `play` spawns `media-player`; on it the worker is started (at the
   decode) and the sink opened (after the decode, so a refused sound probes no device). The window
   sends orders and is woken through its `EventLoopProxy`; it takes `Heard` sentences on the
   terminal and the last of them into the title. A second sound stops the first (Table 210's `/Mix`
   at its default); closing the document stops its sound; Escape stops one that sounds; an order
   that arrives while a sound is decoded is read before the sound starts, so a stop is never a
   click of sound; dropping the player (closing the window) sends `Quit` and joins, which closes the
   stream and kills the worker. A sound's end is posted by the sink's `ended` and said.
5. **The sink** is a trait, `viewer_host::sound::Sink`, built on the player's thread by an
   `OpenSink`, so a stream that may not cross threads never does. `quorra`'s is `cpal`'s default
   output (`viewer-ui`'s `quorra/sound.rs`): the device's configuration read at the opening, one
   output stream per sound, written in the device's own sample type (`f32`, `i16`, `u16`, `i32`,
   `f64`; any other is said).

## The dependency

`cpal` 0.18.2, `default-features = false`, in `viewer-ui` only; the argument is `doc/stack.md`'s
row and the workspace manifest's comment, the principle 3 justification ADR 3297's. `cargo deny
check` passes it, with main's 33 duplicate warnings and no new one.

**Size.** The release `quorra`, stripped, is 41,610,144 bytes. Read off its symbols: `cpal` 27.8
KiB, `alsa` 23.1 KiB, `quorra`'s sink 18.7 KiB (`cpal`'s stream code made for five sample types),
`viewer_host::sound` 53.7 KiB with the media worker's client inlined into it, `pdf_media_worker`
2.9 KiB — about 126 KiB of code, beside RFC 0015 section 9's 105 KiB for `cpal` alone. A
before-and-after of the whole binary was not taken: one fat-LTO build of `quorra` took 35 minutes on
this machine at a load of 90 to 116, and the post-merge install's binary is the next reading.

**The link.** `alsa-sys` links `libasound` at load time, so `libasound.so.2` is now a `NEEDED`
library of `quorra`, the only one besides the C library's: the loader maps it at every start, and a
machine without it cannot start the window. The device is still opened only at the first sound.
`doc/questions/Q972` puts the remedy — a helper program started at the first sound, behind the same
`Sink` — to the owner.

## What was not done, and why

- **No human has heard it.** The build machine has no sound card. `viewer-host`'s
  `tests/played_sounds.rs` runs the whole path — a document's sound annotation, the core's event,
  the level, the real confined worker, the conversion, a recording `Sink` — and checks the samples
  the device would be handed against the arithmetic. The `cpal` sink itself is compiled and linted,
  not run.
- **A reader who wants this machine's own player for a sound** has no word for it in this window:
  the route follows what the window can do. A word of its own would be the "new switch" RFC 0015
  section 4.2 declined; it is the thing to add if a reader asks.
- GTK (step 3) needs the same `Player` with `cpal` behind its own `OpenSink`, its `media` handler
  calling `handing_here`, and a main-loop wake in place of winit's proxy.

## Cost

A field on one event and one wire record, a module of about 450 lines with its tests in
`viewer-host`, 130 lines in `quorra`, one C library linked into `quorra` and loaded at the first
sound, and a copy of the sound's filtered data beside the written file (both at most the
document's stream limit).
