# Two things break at high zoom: the chrome, and the glyphs

Status: **reproduced and measured, not diagnosed.** Found in the three-hundred-and-thirty-sixth
session, by zooming a page in steps until something broke — which is what the project owner asked
for.
Priority: 12
Corpus: none — this is the *window*, and no gate opens one
Clauses: none. Every number here is this host's own chrome.
Code: `crates/render-quorra/src/present.rs` (`build`), `crates/viewer-ui/src/bin/pdf-viewer.rs`
(`present`), `crates/render-quorra/examples/zoom_ladder.rs`

## What happens

Open `doc/PDF20_AN001-BPC.pdf` at page 3 in a 900 × 1100 window, press `+` repeatedly, and
photograph the window at each step (ADR 0126's recipe). The page is fit to the window at **131%**
and every press is ×1.25.

| presses | magnification | the page's target | the sidebar |
|---|---|---|---|
| 10 | 1 220% | 7 258 × 10 268 | drawn, rows and tab strip |
| 12 | 1 900% | 11 341 × 16 044 | drawn, rows and tab strip |
| **14** | **2 970%** | 13 500 × 19 100 | **background only, shifted ~35 px down** |
| 16 | 4 640% | 22 205 × 31 405 | background only, shifted ~40 px |
| 20 | 6 400% (the clamp) | 38 093 × 53 876 | background only, shifted ~85 px |

**The page's glyphs break too, and separately** — that is the section below, and it is quorra's
rather than this host's.

**And the CPU backend does not break.** The same document at the same 16 presses under `--cpu`
draws the sidebar whole: rows, tab strip, disclosure triangles. So this is the graphics path, not
the panel's own display list and not the host's arithmetic.

## And the characters, which is the second defect and is **quorra's** — `doc/QUORRA_FEEDBACK.md` §11

The owner reported it as "zoom in and it looks fine, then it starts being wrong, and zooming out
again keeps having broken fonts", with a screenshot where `extensive` reads `extens:ve`. That is
now **reproduced offscreen in two frames** and written up for the rendering library's team:

- it is the **GPU coverage lane**, which `viewer-ui` switches to above 10× magnification
  (`GPU_COVERAGE_MAGNIFICATION`), and the same ladder on the CPU lane is clean at every rung;
- it is **state, not magnification**: a fresh device whose first frame is 6400% is clean
  (mean 0.0134), and the same frame after one 3200% frame on the same device is wrong (7.63);
- the damage **reaches backwards**: 1600% is wrong afterwards and was right on the way up.

What is left on *this* side is the mitigation, and it is deliberately not taken yet: not switching
lanes would cost the ten-fold frame time the switch was measured to buy
(`doc/quorra-gpu-coverage.md`). It waits on their answer.

## What is measured, so that "the characters go wrong" has a number

`cargo run --release -p render-quorra --example zoom_ladder` walks the *window's* transform up a
ladder of magnifications — which is what a viewer at 4 000% actually rasterises, a 900 × 1100
window at a scaling transform rather than a whole page — and compares the two backends on each
rung. On this machine (`lavapipe` under `Xvfb`), page 3 of the note:

```text
    zoom        target       mean     worst      ssim
     100%    595 × 841       0.5574     20.31   0.99272
     800%   4761 × 6734      0.1171      1.73   0.99950
    3200%  19046 × 26937     0.0403      1.00   0.99982
    6400%  38092 × 53875     0.0301      1.71   0.99985
```

**Those numbers are the *CPU* coverage lane**, which is what a ladder gets by default — and that
is exactly why the first run of this instrument said the characters were fine. Switching the lane
at 10× the way the viewer does is what produced §11's reproduction above. **A ladder that does not
switch lanes is not measuring what a person sees past 1000%**, and this file said the opposite for
one round.

## The hypothesis to check first

The overlays are drawn into the *same* scene as the page, immediately after it, each with its own
`TargetSpec { width, height, transform: IDENTITY }`. What changes between 1 900% and 2 970% is not
the overlay and not the window: it is the **page's** target, which crosses 16 384 in one dimension
between those two rungs.

That number is quorra's scratch-sheet bound, and this tree already knows it as the reason a page
can be refused (`doc/HANDOVER.md`'s "a page the device refuses for the other reason" —
`bug1721218_reduced.pdf`, whose coverage outgrows a 16 384 × 16 384 scratch image). The frame here
is **not** refused: it presents in 78 to 92 ms and reports nothing. So the first question is
whether a scratch sheet sized from the *page's* extent is being shared with, or is silently
truncating, the commands encoded after it — and whether a refusal is being lost where the handover
says one would be reported.

Two cheaper checks before that one:

- the shift grows with the zoom (~35 px at 2 970%, ~85 px at 6 400%), which a *clamp* would explain
  and a precision loss would not;
- the panel's background rectangle survives and its glyph fills do not, which is the same split
  ADR 0154's collapsed fills have — a rectangle is one command and a row of text is hundreds.

## Why it is a defect and not a curiosity

A person who zooms to read something small loses the panel that tells them where they are, and
nothing says so — the trap-5 shape in the window rather than on the page. And **no gate can see
it**: the corpus interprets page one, the oracle rasterises a page at its own scale,
`render-quorra/tests/corpus.rs` compares backends at 1×, 2× and 4×, and none of them opens a
window or magnifies past 4×. The ladder above is the first instrument that looks.
