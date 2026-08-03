# Hairline marks, measured against §10.7.4 — a proposal from the quorra side

Written 2026-08-03, after `QUORRA_FEEDBACK.md`'s findings were answered. This is the
reverse direction: something the quorra work found in *shared* code, brought back here
because the code it concerns — `pdf_render::split_collapsed_fill` — is deliberately this
tree's, and a change to it moves every backend at once, the oracle's included. Nothing
below is implemented; it is a diagnosis, a reading of the clause, and an offer.

## What the owner saw

`issue4260_reduced.pdf` — the page of ruling lines that QUORRA_FEEDBACK.md section 1
fixed — drawn by this viewer next to Okular, on a high-DPI screen. Ours looked fuzzy
and uneven; Okular's grid looked delicate and crisp. Both screenshots, measured
(vertical cut through the horizontal rules, rows of ink per line):

| | rows per line | total ink per line |
|---|---|---|
| this viewer | **2 rows** | **1.00** (split roughly half and half) |
| Okular | **1 row** | **0.13** |

And the same page rendered offscreen through both of this tree's backends, at three
scales (`crates/render-quorra/examples/mark_width.rs` is the instrument):

| render scale | rows per line | total ink per line (both backends) |
|---|---|---|
| 1× | 1–2 | 1.00 |
| 2× | 1–2 | 1.00 |
| 4× | 1–2 | 1.00 |

## The diagnosis

Two different renderers, two different halves of the right answer.

- **This tree's mark is the right amount of ink in the wrong place.** A §10.7.4 mark
  is one device pixel thick at any scale — the earlier finding's fix already made it
  screen-resolution on a high-DPI target — but it is an *antialiased band centred
  wherever the degenerate rectangle happens to sit*. At y = 1085.3 the band splits
  half-and-half across two pixel rows. A page ruled with such lines renders as a mix
  of crisp single rows (where the position lands on a boundary) and fuzzy grey double
  rows (where it does not), which is exactly the unevenness the screenshot shows.
- **Okular's mark is in the right place with the wrong amount of ink.** One crisp
  row — but at 13% coverage, consistent with a supersampled internal rendering
  averaged down. The line keeps its sharpness and loses seven-eighths of its colour.

## The clause prescribes better than either

ISO 32000-2 §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region
> intersects the shape, no matter how small the intersection is. This ensures that no
> shape ever disappears

Read literally: the *pixels* the degenerate shape touches are **painted** — full
current colour, pixel-aligned. A zero-height line at y = 1085.3 intersects exactly one
pixel row, and the clause's own rendering of it is that one row, fully painted: crisp
like Okular's, full-ink like ours. What `split_collapsed_fill` produces today — a
one-device-pixel band at the shape's fractional position, antialiased like any other
fill — is an approximation of the clause, not the clause.

## The proposal

Snap the marks to the device pixel grid, in `pdf_render::split_collapsed_fill`, where
every backend inherits the same answer (trap 2: a decision either backend takes alone
is a decision neither has made). Concretely:

- The helper gains access to the placement transform (it already takes `thinnest`,
  which the caller derives from that transform; the snap needs the transform itself).
- Under an **axis-preserving** transform — which these marks essentially always ride —
  each collapsed subpath's mark becomes the run of whole device pixels its zero-extent
  axis intersects, mapped back to path space, so the ordinary fill machinery paints it
  at exactly full coverage.
- Under a rotated or sheared transform, pixel alignment has no meaning along the
  mark's axis; today's band remains, as the stated fallback.

Estimated at roughly half a day including tests, on the quorra side's offer to
implement.

## Why it is an offer and not a commit

The change moves `render-cpu`'s pixels too, and `render-cpu` is the oracle: every
corpus and oracle ratchet touching a page with degenerate fills will shift. The
expectation is that they shift *toward* the references — Adobe and poppler both render
hairlines snapped — but expectation is not measurement, and re-ratcheting the oracle
is this tree's decision to take, not a backend's. Say the word and the quorra side
will implement it, re-run the corpus at 1× and 4×, and bring the before/after numbers
back to this document.

## Reproducing the numbers

```sh
cargo run --release -p render-quorra --example mark_width
```

renders `issue4260_reduced.pdf` through both backends at 1×, 2× and 4× and prints,
for a column crossing only horizontal rules, each line's starting row, rows touched,
and total ink. The screenshot measurements were the same cut applied to the two
window captures.
