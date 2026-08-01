# ADR 0131 — The glyph cache, and the sixteenth of a pixel it costs

Status: accepted, 2026-08-01. Session 146. **Not built, and the number is why.** This is step 2 of
ADR 0128's plan, executed as a measurement, and it changes step 4.

## What ADR 0128 asked for

> **A glyph coverage cache in `render-cpu`.** The same insight, in the backend that is both the
> correctness oracle and the startup path. It prices the atlas before anyone writes a shader, and
> it speeds up the path page one already takes.

The insight was a count: page 6 of ISO 32000-2 is **5933 fills of 107 distinct outlines**, and
every backend re-derives all 5933 from scratch. Rasterise 107 coverage bitmaps instead and blit
them, and most of the page's work disappears.

## The number ADR 0128 did not take

A coverage bitmap can be reused only where the outline, the linear part of the device transform,
**and the sub-pixel phase of the translation** all match. A glyph a third of a pixel to the right
is antialiased differently; blitting a bitmap rasterised for a different phase moves ink.

`examples/glyph_reuse.rs` counts exactly that, and it is committed so the next session can re-price
this in one command. Distinct cache entries against 5933 and 4282 fills:

| phase | ISO 32000-2 p. 6 | `tracemonkey.pdf` p. 1 | ISO 32000-2 p. 101 |
|---|---|---|---|
| **exact** | **5817** | **4282** | **3002** |
| 1/32 pixel | 1634 | 3866 | 2787 |
| 1/16 pixel | 1182 | 3219 | 2480 |
| 1/8 pixel | 858 | 2109 | 1862 |
| 1/4 pixel | 576 | 1261 | 1186 |
| 1/2 pixel | 350 | 787 | 789 |

**The exact row is the finding.** A cache that moves no pixel at all would hit 116 times out of
5933 on the page ADR 0128 counted, and **not once** on `tracemonkey.pdf`. Glyph origins are
arbitrary floats; two occurrences of the same letter almost never land on the same sub-pixel
phase. The 107 outlines are real and the sharing is real, but it is sharing of *outlines*, not of
coverage — and the coverage is what costs.

So the cache pays only with a quantised phase, which is a positional departure, which
`CLAUDE.md` requires to be argued and costed rather than taken.

## What the departure costs, measured

The cost was measured the only way this tree measures a departure: apply it and run the gates. A
patch snapping the device translation of every fill whose device extent is at most 64 pixels to a
grid of *n* phases, then the oracle over 1794 pages and the text gate over 974 documents.

| phase | oracle | text gate |
|---|---|---|
| 1/4 pixel | **1 page newly contradicted** — `freeculture.pdf` page 339 | unchanged |
| 1/8 pixel | **1 page newly contradicted** — `issue11403_reduced.pdf` page 1 | — |
| 1/16 pixel | clean | — |
| 1/32 pixel | clean | — |

A *page a page*, and a different page each time, which is what a threshold effect looks like: at
1/8 pixel some glyph on some page crosses the tolerance its class is held to. **The finest
quantisation the oracle tolerates is 1/16 of a pixel**, and at 1/16 the cache is 5.0× on page 6,
1.3× on `tracemonkey.pdf` and 1.2× on page 101.

## What the win would have been

`callgrind_rasterise` on page 6, twenty renders, 16 771 M instructions. The coverage half of that
is `fill_path_impl` 24.5%, `SuperBlitter::blit_h` 17.0%, `AlphaRuns::break_run` 5.7% and
`SuperBlitter::flush` 1.9% — **49.2%**. That is the part a cache removes, in proportion to the
reuse; the blit of the cached coverage stays, and is new code.

So at 1/16 pixel the upper bound on page 6 is about **39%** of the page, and on `tracemonkey.pdf`
about **11%** — before the cost of a blitter this crate does not have. `tiny-skia` has no API for
compositing a small coverage bitmap at an offset, so taking this means writing one: a paint, a
clip multiply, a blend mode and the `Compose` distinction §11.4.6 needs, in the backend whose
first sentence is that correctness outranks speed here.

## The decision

**Not built.** Two reasons, in order:

1. **The reuse is not general.** 5.0× on a page set in one face at one size, 1.3× on a page of
   ordinary book text. A cache whose hit rate is a property of the document is not an optimisation
   this project can quote a number for, and `CLAUDE.md` requires a benchmark beside every
   optimisation.
2. **It is a compositor.** Writing a blitter into `render-cpu` puts new correctness risk into the
   backend that *is* the correctness oracle — the one place this tree has no second opinion.

**And it changes ADR 0128 step 4.** The atlas was named there as "the single largest optimisation
available to this program". It is not free of this measurement: a GPU glyph atlas quantises the
sub-pixel phase in exactly the same way and buys exactly the same reuse. What a backend of our own
would gain over Vello on this page is therefore **the 1/16-pixel reuse and nothing more than
that** — 39% of one page's rasterisation, 11% of another's. The other four items on that list
(damage rendering, persistent geometry, progressive rendering, clause 11) are untouched by this
and are now, by elimination, the whole of the case.

## The item this found instead

The same profile, same page, same twenty renders:

| | share of page 6 |
|---|---|
| `calloc`, **all of it under `tiny_skia::Mask::new`** | **18.1%** |
| `fill_path_impl` | 24.5% |
| `SuperBlitter::blit_h` | 17.0% |

Page 6 uses **303 distinct clips** and the mask cache builds each exactly once — 303 callocs per
render, no thrashing, all of them zeroing a page-wide band. Nearly a fifth of a dense text page is
spent zeroing clip masks, and unlike the glyph cache **nothing about attacking it moves a pixel**.
It is the next thing to measure, and `glyph_reuse` prints the clip count beside the fill count for
that reason.

## The lesson

**A count of what is shared is not a count of what can be reused, and the difference is the key.**
ADR 0128 counted 5933 fills of 107 outlines and drew the obvious conclusion; the outlines are
shared through an `Arc` and the *coverage* is not shared at all, because the thing that varies is
the one the count left out. **Ask what the cache's key would have to be before believing the
count** — this tree already knows that a cache is a claim and its key is the currency of the claim
(ADR 0115), and here the key decides whether the optimisation exists.
