# 3384 — An MP4 clip of sound alone is played here

Session 2170. Status: accepted. Context: ADR 3350 (viability judged for the player a clip reaches),
ADRs 3381–3383; ISO 32000-2 §13.2.2, §13.2.4.2 and Table 285's `/CT`.

## Decision

1. **`container::is_played_here`** admits, beside a RIFF WAVE file, an ISO base media file whose
   `/CT` is one of the family's `audio/` types (`audio/mp4`, `audio/x-m4a`, `audio/3gpp`,
   `audio/3gpp2`), matched as RFC 2045 matches a type — without case and without parameters.
   `pdf_model::media`'s walk asks it (`Walk::played_here`), so §13.2.2's viability is judged for this player for such a
   clip exactly as for a WAVE file.
2. **`video/` and `application/` MP4 types stay with the hand-off.** A file so named may hold a
   picture, and playing its sound alone would present less than the clip while saying it was played;
   video is still excluded (`CLAUDE.md`), so the hand-off remains that clip's player.
3. **The clip crosses as its own recording**: `viewer_core::Recording::Mp4`, chosen by the clip's
   `/CT`, the confined protocol's sound kind 3, and `viewer_host::sound` decoding it with
   `MediaWorker::decode_mp4` (ADR 3381). The window's sentence names it "§13.2.4.2, an MP4 clip's
   AAC" and carries the worker's notes — concealed access units, an edit list not applied.

## Consequences

- An MP4 clip whose track is not AAC (`object type` other than 0x40), or that the reader declines,
  is refused in the window with the worker's sentence rather than handed on: it was judged for this
  player, as a WAVE file the worker declines is.
- The four windows need no change: each plays what `viewer_host::sound` gives its sink.
