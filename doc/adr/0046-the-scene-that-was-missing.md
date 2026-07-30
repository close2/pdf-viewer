# ADR 0046 — The scene that was missing

Status: accepted, 2026-07-30.

## Context

Clause 11 was ten `unreviewed` rows from complete — §11.1, §11.2, §11.3's arithmetic, and
§11.3.8's summary, all of which the handover described as "the model rather than its PDF
representation" and therefore as the cheapest rows left. The demand track had no obvious item:
what the corpus still names is transparency-group departures, which need a second raster format
or a backdrop, and substitution quality, which is not a clause gap.

The review supplied the demand item.

## What reading §11.3.5 found

**Not one cross-backend scene selects a blend mode.** Every `Command` in every other scene in
`headless_gpu.rs` carries `BlendMode::Normal` — geometry, shadings, images, soft masks and a
transparency group are all compared under it — so the two backends' sixteen blend functions had
never been held to each other at all.

That is trap 2 in its stated form: **a decision either backend can make alone is a decision
neither has made.** And §11.3.5.3's four modes are exactly where it bites. Hue, Saturation,
Color and Luminosity are *non-separable*: each is defined over all three components at once
through the clause's `Lum`, `ClipColor`, `SetLum` and `SetSat` functions, so no per-component
formula produces them, and one of them being wrong still draws a plausible picture. Nothing else
in this project could see it — the corpus gate reads `unsupported: []` either way, and the
reference oracle judges a whole page against a tolerance built for glyph hinting.

## What the scene found

Twelve separable modes and `Saturation` agree **to the channel** — not within a tolerance,
exactly. `Hue`, `Color` and `Luminosity` differ by 113 of 255.

**The clause settles which is right, so this is not a tie.** Red painted over blue in `Hue` is
`SetLum(SetSat(Cs, Sat(Cb)), Lum(Cb))`. `Sat(blue)` is 1 and `SetSat(red, 1)` is red;
`Lum(blue)` is 0.11 against `Lum(red)`'s 0.30, so `SetLum` adds −0.19 to each component and
gives (0.81, −0.19, −0.19). A component fell below zero, so `ClipColor` applies, and the red
component becomes `L + (C − L) × L ÷ (L − n)` = 0.11 + 0.70 × 0.11 ÷ 0.30 = **0.367**, which is
94 in eight bits.

Vello produces 94. `tiny-skia` produces 207, which is 0.81 — the value *before* the clip.

**And a debug build says why.** Under Rust's overflow checks, `tiny-skia`'s own
`wide/u16x16_t.rs` panics with "attempt to multiply with overflow" inside these three modes, so
the release-build answer is an intermediate exceeding sixteen bits and wrapping. That is a
defect in the dependency's arithmetic rather than a difference of reading, which is why the
scene is a release-build test and says so.

The uncomfortable part is worth stating plainly: **the CPU backend is this project's correctness
oracle for everything else, and here it is the wrong one.**

## Decision

- **`test_scenes::blend_modes`** is a 480-by-480 page of sixteen tiles, each a three-band
  backdrop under a three-band source. Two details are load-bearing and both are about keeping
  an edge rule out of a colour measurement: every coordinate lands on a whole pixel at scale 1,
  and every band is inset by two units so that **no two rectangles share an edge**. The first
  draft had neither, and every one of the sixteen modes "differed" — by the seam between two
  colours, which two rasterisers antialias differently.
- **The test holds the disagreement as a named list rather than under a tolerance**, ratcheted
  in both directions: fixing one of the three fails it, and a fourth joining fails it too.
- **§11.3.5.3's ledger row is `partial`**, with the closed form on it.

The fix is not taken here, and the reason is that it is a different piece of work: the four
non-separable modes would have to be composited by `render-cpu` itself rather than handed to
`tiny-skia`, which means a paint path that reads the destination. That is a change to the
backend's structure, and it should be made with its own benchmark rather than at the end of a
clause review.

## Consequences

Clause 11 is complete as a review — 58 rows, none `unreviewed`. Two of its rows record something
this project did not know an hour earlier: §11.3.4's blending colour space is the device's and
four corpus documents ask for another, and §11.3.5.3's three modes are wrong on the backend that
is usually right.

No gate number moved. The scene is the thing that moved: **a cross-backend comparison is only
worth what its scenes can express**, and this one could not express a blend mode at all. The
sixteenth session learned the magnitude version of that lesson — a scene must be able to fail at
the defect's *size* — and this is the axis version, three sessions after the handover recorded
that fourteen scenes existed without anyone asking what they did not cover.
