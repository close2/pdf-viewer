# ADR 0738 — The closed form that counted a corner twice

Status: accepted, 2026-08-28. Session 806. Reads the three page-list notes ADR 0735 overtook,
corrects the caller population `pdf-render`'s `outline` module states about itself, and replaces
the closed form `AMBIGUOUS_TILING_CELL_CLIP` has measured `issue16038.pdf` against since the
three-hundred-and-seventy-fourth session. **No pixel moves**; nothing under `crates/*/src` changes
except one module comment.

## What was owed, and by whom

The eight-hundred-and-third session's merge sweep found ADR 0735 overtaking three page-list notes
— `AMBIGUOUS_TILING_CELL_CLIP`, `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` and quorra's
`DIFFERS_IN_SHAPE` — and filed the reading as owed rather than doing it in a merge. The
nineteenth sweep's question is exact and is not "is this note wrong": it asks whether a decision
was taken *after* a note's newest citation, about a page that note explains. Answering it means
reading the decision against the note and citing it either way.

## What the three notes said, and what they now say

ADR 0735 gave a stroke whose colour is a tiling pattern the region §11.5.2 states as a group's
alpha, so `Interpreter::tile` takes a `Tiled::Fill` or a `Tiled::Stroke`. **All three notes are
about pages that take the first**, and that is a fact about the documents rather than a judgement
about the change:

- `issue12295.pdf` states **no `/Pattern` and no `SCN` at all** — checked on the file expanded by
  `qpdf --qdf --object-streams=disable`, because the tokens live inside object streams and a grep
  over the original file is a claim about compression. Neither arm of `tile` is reached on it.
- `issue16038.pdf` installs its two cells with `scn`, under a `B` whose stroking colour is the
  `0 G` set above them. It contains no `SCN`. Its patterns reach the fill arm, which ADR 0735
  left unchanged in every particular: the span from `bounds_of`, the path as a clip,
  `state.fill_alpha`, and §11.7.5.2's `inside` test without the shape term.

So no verdict moves and none was expected to. What each note gains is the sentence saying *why*,
with the file's own evidence, and a citation of this decision — which is what takes it off the
sweep, and which is the cure `doc/todo/02` §4 already prescribes.

**ADR 0735 names both pages exactly once each**, in its `doc/todo/00` step-7 line recording that
neither moved. That is the benign shape the sweep was designed to surface: a decision that
*measured* a note's page is a decision about that page, and the sweep cannot tell a measurement
from a mechanism. Reading it is a round's work and the answer is worth having either way.

## The finding: the area the document asks for is 313.12, not 316.29

`AMBIGUOUS_TILING_CELL_CLIP`'s whole argument rests on a closed form — the ink `issue16038.pdf`
states, computed from the file with no renderer in it — and every judgement in the note is a
percentage of it. It has been **316.29 square points** since the three-hundred-and-seventy-fourth
session: twenty rules of `28.3468 × 0.3985`, plus a `0.3985` border around each of two `28.3468`
squares.

Both terms are right and their sum is not, because the two overlap. Each rule is a pattern mark
clipped to its square's fill path, so it runs to `x = 0` and `x = 28.3468`; the border is a
**stroke of that same path**, and §8.4.3.2 puts half its width on each side of it. The region a
rule shares with the border it ends under is `2 × (w/2) × w = w²`, which is 0.15881, and there are
twenty rules. **3.18 square points are counted twice**, and the area the document asks for is

    20 × 28.3468 × 0.3985  +  2 × 4 × 28.3468 × 0.3985  −  20 × 0.3985²  =  313.117

Checked against a 1/1024-point sampling of the same geometry, which converges on 313.12 from
below and agrees at 313.122.

**Why it matters is not the 1%.** The note's load-bearing sentence is *every reference is above
the area and we are the only one below it*, and the interesting question underneath it is how far
below. Against 316.29 this tree reads 98.4% at 24× and the shortfall looks like a residue nobody
had explained. Against 313.12 it reads **100.0%** — 313.02 measured — so at the limit there is no
shortfall at all, and what remains at the page's own scale is 4.2%, which is the anti-aliasing
departure §10.7.4's row already documents, on a rule half a device pixel wide. The `−5.642` this
page has held at the head of `doc/todo/00` step 7 for many rounds is therefore **entirely the
references' excess**: the nearest of them is 17% over the area and the furthest 204% over.

That is what that file's own paragraph on this page has argued since the
three-hundred-and-seventy-fourth — "a page can sit at the head of this ranking because the
references are heavy rather than because we are light" — and it had never been held to a limit of
ours, because the limit it would have been held to was 1% wrong.

## And it is a test now, because that is what stopped it being one before

The closed form stood wrong for four hundred sessions for a reason the ledger states about
itself: *a note is a claim; only a test makes it a fact.* Every percentage in
`AMBIGUOUS_TILING_CELL_CLIP` was against a number nobody re-derived, and nothing in the tree could
have said so.

`tiling.rs::the_page_that_is_a_closed_form_weighs_what_the_closed_form_says` opens
`issue16038.pdf` — the real document, trap 4 — renders it at 24× under `Medium::NONE` so that
alpha *is* coverage and no greyscale weighting enters, and asserts the ink is within half a
percent of `rules + borders − shared`, with the three terms computed in the test rather than
written down as a total. It skips, saying so, without `doc/pdf.js`.

**Calibrated twice, both runs, and the second is the more interesting** (trap 13):

- With the third term dropped — the note's own 316.294 — the test fails, 1.03% out. That is the
  defect it exists for and it is the arithmetic this decision corrects.
- With ADR 0155 undone in the source (`unclip_redundant_cell` made to return `false`, the
  redundant per-cell box back on) the test **passes**: the page deposits 312.975 against 313.016,
  a movement of 0.013%. So this test does not hold that defect, and saying so is worth more than
  a tolerance tightened until it did. A clip's cost is the anti-aliased seam at a cell boundary,
  which is 15% of the ink at the page's own scale and a fortieth of that at 24×;
  `a_rule_spanning_its_whole_cell_deposits_the_ink_its_geometry_states` is what fails under that
  mutation, at 1×, and it did. **Two tests, two scales, and neither is the other's spare** — which
  is the same shape as the four-scale table below being a new row rather than an old one
  continued.

## ADR 0226's owed column, and why it is a new row rather than that one continued

ADR 0226 closed with the four-scale interior-coverage table in this note carrying a 1× column
older than itself, "marked as such rather than guessed at". Taking it needs a band whose ink is
known exactly, and the construction that gives one needs no whole number of pixels: a band from
2.5 to 10.5 periods has both edges in the middle of a white gap and holds exactly the eight rules
`k = 3..10`, so snapping it to whole device rows changes its **area** and not its **ink**. The
quantity is then that ink over the `8 × 0.3985 × width` the geometry puts in it.

```text
               1×      2×      4×      8×     24×
left        0.971   0.986   0.990   0.994   0.998
right       0.980   0.970   0.993   0.994   0.998
```

The two squares still weigh the same to within 1.6% at 2× and 0.9% everywhere else, and both
approach the geometry from below without crossing it — ADR 0213's result holding under everything
since. **It is recorded as its own instrument rather than as the old table's row completed**: the
older band was placed differently and read above 1.0 at 8×, and two measurements of the same shape
are not one measurement. Writing today's numbers into yesterday's row would have manufactured a
movement out of a change of instrument, which is the failure the paragraph below is about.

## A lesson about the instrument, paid for in this round

`doc/todo/00` step 7's number is *our ink minus the lightest live reference's*, and the file
prescribes the recipe: `magick <png> -alpha off -colorspace Gray`. Run instead with a greyscale of
one's own — Rec601 luma over the three channels — the same artefacts put `issue16038.pdf` at
**−5.394** where the prescribed recipe puts it at **−5.642**, while `issue12295.pdf` reads −2.364
against −2.362.

The difference is the page: `issue16038.pdf`'s rules are pure **blue** and every greyscale weights
blue differently, `issue12295.pdf` is near-black and weights the same in all of them. This file
already warned that an *absolute* ink differs between recipes on a coloured page. What it did not
say is that the **difference between two renderers** does too, and a quarter of a level is the
size of the movement the sweep is watched for — so a round that changes the recipe manufactures a
finding, and a round that inherits a number taken under another recipe compares two instruments.
The correction is in `doc/todo/00` beside the run.

## The third caller, which a sentence did not know about

`pdf-render`'s `outline` module says why it answers a question `Command::device_bounds` cannot,
and then enumerates who asks: "once per pattern for ADR 0155's containment test, and once per
command per *cell* for `repeat`'s fold". ADR 0735 added a **third** caller — `pdf_model`'s `tile`,
once per patterned stroke, to say which tile sites the outline reaches — and the sentence had no
instrument on it, because `--bin parts` reads cardinals about this tree's own crates and backends
and not a prose enumeration inside one.

It is worth more than a bibliographic fix, because the third caller asks a *third* question. The
first two ask **containment** — does this command mark outside this rectangle — and the new one
asks **reach**: which cells can this mark touch. They want the same tight bound for opposite
reasons, and the module's own two-question framing did not have a place for the second.

## What was measured

Every gate in `doc/todo/02` §2, whole, before the round's first edit and again after its last. No
verdict, count, list or ratchet moves in either direction: this round writes doc comments, one
ledger sentence and documents. `doc/todo/00` step 7 over all 835 measurable ambiguous pages, and
the §4 sweeps before and after, with `overtaken` falling by the three notes this decision reads.
The two closed forms above are arithmetic on the file and a sampling of the same geometry; the
five-scale ladder is `examples/render_at` and the red channel, which is coverage here because the
fill is pure blue and the border is black.

## What this does not close

The page stays `ambiguous` and should. Nothing here moves it toward any reference — the argument
runs the other way, and the four references disagree with each other about this page's weight by
far more than any of them disagrees with us. §10.7.4's row keeps its departure (1) and gains a
page where its size is measured against a closed form instead of argued.
