# 2297 — The knockout residual is two opaque images, and nothing is built for it

Status: deferred — the count says the residual ADR 2286 named is two elements of one kind on the
corpus's first pages, and what would reopen it is below. Session 1731, 2026-10-24, the pixels slot
of batch one hundred and twelve. Clause: ISO 32000-2 §11.4.6 (the row stays `implemented`).
Context: ADRs 1320, 2166 §3, 2286 §4. Code read: `render-cpu/src/encode.rs`'s `encode_command`.

## Context

ADR 2286 drew every knockout `Fill` and `Stroke` with a coverage twin in ADR 1320's two stages and
left three kinds on the bare Porter-Duff `Source` draw, whose edge coverage is the library's lattice
of sixteenths rather than the area `crate::area` measures: an image, a nested group, and an element
with no twin. The brief asked for the residual to be counted before the smallest of the three is
built, and trap 232 says what to count: what the backend draws, not what the display list states.

## The count

A probe at the head of `encode_command` in a scratch copy of the tree — never in this one — printed
the route of every command reaching it under `Compose::Knockout`, with its list and command
addresses, over the `cpu` 1x arm's 968 first pages. Distinct commands by route:

| route | commands | what it draws with |
|---|---|---|
| a `Fill` or `Stroke` with a twin | 340 | ADR 1320's two stages (ADR 2286) |
| `Command::Shaped` | 8 | the shape the interpreter states (§11.4.6's own) |
| an opaque image | 2 | the bare draw unless a clip cuts it |
| a translucent image, a twinless path, a bare nested group | 0 | — |

The addresses are reused across pages, so the counts are a floor; the zeroes are exact, since a
route that never printed was never taken.

## Decision

Nothing is built. Of the three kinds, two never occur on the corpus's first pages, and the third is
two opaque images, whose two stages would differ from the bare draw only along an edge the image's
placement leaves fractional. The smallest build is one condition — `Command::Image` beside `Fill`
and `Stroke` in the knockout branch, the twin already written — and it is not taken on two
elements without a page that shows it.

## Revisit when

- a page is found where a knockout image's edge is fractional and its pixels differ between the
  oracle and the device lane: the one-condition build above, with a test red before;
- a corpus addition holds a twinless knockout path (a pattern or a translucent shading) or a bare
  nested group, which the probe above counts in one walk.
