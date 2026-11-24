# 3611 — A poster crosses the confined wire and the C ABI as an event

Session 2227. Status: accepted. Context: ADRs 1961, 3433, 3609.

## Decision

`Event::Poster { document, annotation, movie, fit }` is a variant of its own rather than a
`Media` without a play: a hand-over is behind the reader's level and a poster is not, and a window
that acts on `Media` would start a player. It crosses the confined wire as kind 32 (the window
holds the worker, `MEDIA`'s reason), and the C ABI as `QUORRA_EVENT_POSTER` (33,
`QUORRA_EVENT_KIND_COUNT` 34), its movie file read through `quorra_events_bytes`.
`quorra-confined` sends no `ShowsVideoHere`, so it is never raised one.
