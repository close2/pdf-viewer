# Q76 — Who builds §11.7.4.3's special overprinting mode inside `raster/`?

Source: round 1172, on ADR 1178's count and ADR 1182's reduction.

## The standing position, and what has changed under it

`render-raster` refuses a display list by name whenever it carries §11.7.4.3's special
overprinting blend mode, and the frame falls back to `render-cpu`. That was the conservative
answer while nobody had counted the population (ADR 1158 section 1). It is counted now: **1788
crawled documents and 9863 pages, 2.7% of the documents that open** (ADRs 1178, 1181) — the largest
by-name coverage loss that backend carries, on the one backend that otherwise *draws* a page
compositing in four components.

The ask has stood in `doc/QUORRA_FEEDBACK.md` section 49 since session 1160, and this round made it
smaller rather than larger: the mode is not a seventeenth blend function but **Porter-Duff
destination-over in the channels it keeps and source-over in the rest**, one `Compose` chosen per
channel, with the two uniform cases reachable from `Compose::DestOver` alone (ADR 1182, with the
identity held against `render-cpu`'s own compositing function).

**Nothing in `raster/doc` mentions overprinting at all**, so the ask exists only in this tree's
feedback file.

## The question

`raster/` is a sub-project in this repository with its own `CLAUDE.md`, its own ADRs and its own
brief. May a round of *this* project implement the mode there — `raster_scene::Compose` gaining
destination-over and a per-channel selection, and `raster-gpu`'s shaders the arithmetic — or does
that stay an ask this side makes and the other side answers?

It is not a question a round can settle from a clause. Two things pull against each other and both
are the project's own rules:

- **quorra's second principle is that this tree's CPU backend is its oracle**, and its own brief
  says an implementation shared between the two backends "would make the cross-backend comparison
  compare one implementation with itself". Writing quorra's arithmetic from here does not literally
  share code — `render-cpu` computes in Rust over eight-bit pixmaps, quorra in WGSL on a device —
  but it does put one author on both sides of the comparison that is this project's main
  correctness instrument.
- **The coverage is real and the other side has not acted on it.** A refusal this project measured
  at 2.8% of the web, whose fix is a compositing operator and three bits, is a poor thing to leave
  standing indefinitely because of where the code lives.

## Recommendation

**Build it in `raster/`, as a round of this project, and keep the oracle's independence by
construction rather than by authorship.** The two constructions are already different in kind — a
per-pixel Rust loop against a shader — and the thing that actually protects the comparison is that
neither side's output is derived from the other's, which is a property of the code and not of who
typed it. The safeguard to write into the brief of such a round is the one this project already
uses on itself: build it from Table 146 and §11.3.6 alone, and do not look at `render-cpu`'s
`Computed::Overprint` while doing so; then let `doc/verify.md`'s cross-backend run be the first
time the two meet.

If the answer is the other one, the consequence is a sentence rather than work: §49's ask stays as
it is, and the two refusals stay with it until the other side takes it.
