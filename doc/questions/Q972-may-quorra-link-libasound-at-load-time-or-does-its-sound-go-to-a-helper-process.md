# Q972 — May `quorra` link `libasound` at load time, or does its sound go to a helper process?

Source: round 2150, RFC 0015 section 5's second step (ADR 3301).

## What the round found

`cpal` reaches ALSA through `alsa-sys`, which links `libasound` with `-lasound`. The release
`quorra` therefore names `libasound.so.2` as a `NEEDED` library, and it is now the only one besides
the C library's three (`readelf -d`): `winit`, `wgpu` and `softbuffer` open their system libraries
with `dlopen` when they are first used, and `alsa-sys` cannot.

Two consequences, neither of which the RFC priced:

1. **The loader maps and relocates `libasound` at every start**, before `main`, whether or not a
   document ever plays a sound. Measured with `LD_DEBUG=statistics` on a loaded machine, the
   loader's best of six runs went from 386 thousand cycles (the installed build of main, 56 commits older) to 472 thousand
   (this one), about 30 µs warm; cold, the library's pages are read from disk as well. The
   counted `launch_path` figures exclude the loader by construction (ADR 2666), so no band sees it.
   It is small, but it is exactly what `CLAUDE.md`'s "Nothing eager" rule names: work at launch
   for something page one does not need.
2. **A machine without `libasound.so.2` cannot start `quorra` at all**, to read a document that
   has no sound in it. Every desktop distribution ships the library, but a minimal or container
   install may not.

The audio device itself is still opened only at the first sound; that part of the brief holds.

## The choice

- **A. Keep the link** (what this round built). Cost: the two points above. Simplest; GTK's step 3
  inherits it.
- **B. A helper program, `quorra-sound`, started at the first sound.** It links `cpal`, reads
  converted `f32` samples from a pipe and plays them; `quorra` links no audio library and loads
  nothing at launch. `viewer_host::sound::Sink` is already the seam: the helper's sink is a child
  process and a pipe instead of a stream. Cost: one more binary beside the windows, one more
  process at the first sound (a few milliseconds, which a person clicking a sound does not see),
  and about a round of work.

## Recommendation

**B**, as the next audio round, before GTK (step 3) takes the same link. It keeps the launch path
free of anything a sound needs, which is the rule the round was briefed with, and it makes the
missing-library case a sentence ("this machine has no ALSA library, so the sound was not played")
instead of a program that will not start.
