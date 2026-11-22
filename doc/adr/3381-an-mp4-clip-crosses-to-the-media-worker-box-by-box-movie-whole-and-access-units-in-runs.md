# 3381 — An MP4 clip crosses to the media worker box by box, its movie whole, and its access units in runs

Session 2170. Status: accepted. Context: RFC 0015 sections 4.3 and 5 (step 9), ADRs 3297 (the
worker and its wire), 3349 (a WAVE clip's layout request), 3369 (the MP4 reader), 3382 (the
decoder); `pdf_sandbox::lockdown`'s `MEDIA_ADDRESS_SPACE_LIMIT`.

## Context

A request is capped at `MAX_RUN_BYTES`, 256 KiB, and `Profile::Media`'s address space at 64 MiB,
sized against one such request (ADR 3297). An MP4 clip is megabytes — the held conformance streams
reach 3 MB, and a song is several — and its movie box, which holds the sample tables, may stand at
either end of the file. Two shapes were open: raise the bound so the whole clip crosses, or have the
host walk the file on the worker's answers and send only what each step needs. The host may not
read the document's bytes as structure either way; it may cut the file where the worker says and
check the worker's answers against the file's length.

The whole clip cannot cross under the present ceiling: a 3 MB file held once, its table, and its
decoded samples (a three-minute stereo song is 69 MB of `i32`) do not fit 64 MiB, and raising the
ceiling to the largest clip a document may embed is no ceiling. And a decoder carries state from one
access unit to the next, so cutting the decode into independent requests would cost the transform's
overlap and SBR's envelopes at every cut.

## Decision

1. **The host walks the top of the file on the worker's answers** (`FRAME_BOX` → `FRAME_BOXED`):
   it sends at most 32 bytes from a place (`mp4::TOP_HEADER_BYTES`: a 64-bit size and a `uuid`'s
   extended type), and the worker answers the box's type, where its payload begins and where it ends
   (`mp4::top_level_box`). The host checks that the payload begins at least eight bytes on, that the
   box ends inside the file and not before its payload — so the walk always moves on — and stops at
   `MAX_TOP_LEVEL_BOXES` (1 024) steps. Two movie boxes are declined, as ADR 3369's reader declines
   them.
2. **The movie crosses whole** (`FRAME_MOVIE` → `FRAME_TRACK`), the one request past
   `MAX_RUN_BYTES`: at most `MAX_MOVIE_BYTES`, 8 MiB, with the file's length. The worker reads the
   track (`mp4::movie_sound_track`), starts the clip's decoder from its `AudioSpecificConfig`, and
   answers every access unit's offset and size and what the edit list presents (ADR 3383). The host
   checks every unit lies inside the file and fits one run, the count is at most `MAX_ACCESS_UNITS`,
   and the time scale is a clock.
3. **The access units cross in runs to a decoder the worker holds for the clip** (`FRAME_UNITS` →
   `FRAME_PCM`): at most `MAX_UNITS_PER_RUN` (32) units and `MAX_REQUEST_BYTES` a run, in order. The
   worker holds one `aac::Session` from the movie request to the next movie request; a run before any
   movie is refused. The reply states its rate, channels and the count of units it concealed; the
   host checks whole frames of 1 to 128 channels, at most 2 048 samples a channel a unit, no more
   concealed than sent, and the same rate and channels as the first run that had samples.
4. **The ceiling is unchanged.** The worker's largest holding is the movie request: 8 MiB read once,
   the access-unit table at sixteen bytes a unit (16 MiB at `MAX_ACCESS_UNITS`), and the reply at
   twelve (12 MiB) — 36 MiB over the 4 MiB the worker begins with, under 64 MiB. A run holds 256 KiB
   in and at most `MAX_SAMPLES_PER_REQUEST` samples out, ADR 3297's figure. `pdf-sandbox` is not
   touched.
5. **Each run is under the processor budget.** Thirty-two units are 0.7 s of 48 kHz sound; the
   scalar decoder's figure in RFC 0012 (60 s of al18 in 0.048 s, release) makes a run a few
   milliseconds, and the debug-build tests answer whole clips in hundredths of a second.

## Consequences

- The host reads no byte of the file as structure: it cuts `file[at..at+32]`, the movie and each
  unit where the checked answers say.
- A clip is several requests where a WAVE file is a layout and runs: a three-minute song is about
  8 000 units, 250 runs.
- A worker killed mid-clip loses its session; the clip fails with the budget's sentence and the
  next clip starts a new worker, as before.
- A movie past 8 MiB is refused by its size, said.
