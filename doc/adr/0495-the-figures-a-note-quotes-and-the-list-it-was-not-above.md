# ADR 0495 — The figures a note quotes, and the list it was not above

Status: accepted, 2026-08-22. Session 670. Adds `tools/conformance/src/quoted.rs` and
`--bin quoted`, the twentieth sweep and the fifteenth to be a program; corrects the figures of
thirteen page-list notes in `crates/pdf-model/tests/oracle.rs`, moves one note back above its own
list, and amends two of them for a clause reading; extends §10.3.1's ledger row, trap 1,
`doc/todo/01`'s catalogue and `doc/todo/02` §4. **No pixel moves.**

## The question two rounds left between them

ADR 0491 (session 665) built `--bin overtaken` and, in choosing it, measured a rival candidate —
*a note whose measurement disagrees with the gate* — and rejected it:

> the gate's own vocabulary appears **12 times (`ssim`) and 34 (`worst tile`) in 7523 lines of
> note** … A group note rarely writes the gate's words beside its numbers, so there is nothing to
> anchor a number to.

ADR 0494 (session 668) worked one group by hand, found **two** stale figures in it, corrected them
by re-running the gate, and closed with the debt still open: *nothing links a group note to the
gate figures it quotes.*

Both are arguing from the same population and reaching opposite conclusions, so this round
counted it before building anything.

## The count, and why the earlier one was low

`ssim` and `worst tile` are two tokens of a five-token vocabulary. The gate prints **four**
measures and this tree writes each of them more than one way:

| measure | how notes write it |
|---|---|
| mean | `mean 1.38`, `mean of 1.11`, `mean at most 1.64`, `mean absolute difference — 5.4` |
| worst tile | `worst tile 6.73`, `worst tile of 10.10` |
| differing fraction | `differing 0.54%` **and** `0.54% of pixels differing` — either word order |
| similarity | `ssim 0.9445`, `similarity 0.9319`, `structural similarity 0.9906` |

Counted over all five spellings, across every page-list note in `crates/`, `tools/` and `fuzz/`:
**32 notes of 124 quote at least one, 137 figures in all.** Restricted to the file the oracle's
report is about, `crates/pdf-model/tests/oracle.rs`, the first run read **116 figures over 25 of
its 98 notes** and found the gate confirming 34 of them.

So the population is about a quarter of the notes and over a hundred figures — not a dozen — and
it is the quarter a round actually opens: the contradicted and ambiguous groups. **A gate, then,
and 0491's measurement was right about its two tokens and wrong about the population.** That is
worth stating plainly rather than quietly: a count taken to answer one question is not a count.

## What it is

`cargo run --release -p conformance --bin quoted -- <the oracle's log>`.

The discriminator is trap 1's by-hand tell mechanised:

> a figure quoted in the gate's own vocabulary, for a page the note itself holds, that **no page
> of that note carries** in the gate's current output.

**Nothing is rasterised and nothing is re-measured.** The oracle already prints all four measures
for every page it does not call agreement, and the change→gate map already makes a round that
touched a note run it — so the right-hand side is a log the round has, and the sweep is under a
second over a file. It is the first sweep here whose other side is another gate's *output* rather
than the tree's own sources, which is why it is the first that takes an argument.

### Precision is the discriminator's other half

The gate prints mean, worst tile and the differing percentage with `{:.2}` and the similarity with
`{:.4}`. A quotation is compared at the coarser of its own precision and the gate's, by formatting
both and comparing the text — no float is ever tested for equality — and a figure written *finer*
than the gate prints came off some other instrument in this tree. That is not a filter but a rung:
0491's own archetype, a note giving ours as `ssim 0.98591` where the gate prints `0.9879`, was
written at five decimals.

### Three rungs

1. contradicted, with a figure the gate *confirms* on the same line of the note or its neighbour —
   the note and the gate are demonstrably about one page's line and one number in it has moved;
2. contradicted, written to exactly the precision the gate prints;
3. contradicted only after rounding a finer figure to the gate's.

Under every hit it prints **what the gate says instead**, over that note's own pages, nearest
value first. That is the whole point: a note is corrected off the run rather than out of reasoning
about it, which is how 0494 took `hayro`'s page 12 from 16.54 to 12.47.

### The one constant bought with a measurement

`STRIDE`, how many words may stand between a measure's name and its figure, is **3**. A reach in
*characters* cannot separate `mean absolute difference — 5.4`, which is a figure, from `it does
not mean item 4 is paid by 0.75`, which is the verb: both put a decimal about twenty characters
past the word. Counted in words the first is three and the second five, and every quotation of a
gate line in this tree is nought to three. What three costs is written beside it: `worst tile the
only figure that moves, to 13.86` is seven words and this sweep cannot see it.

## Calibration, per trap 13 — and the plant found a hole first

**The first plant failed, and that is the finding.** `CONTRADICTED_UNEXPLAINED`'s `mean 0.17` was
changed to `mean 0.22` — 665's own defect, restored — and the sweep said nothing, because that
list is `[&str; 0]`: the page moved to `CONTRADICTED_TIGHT_CONSENSUS` in session 662 and the note
kept four paragraphs and six figures about a page it no longer holds. **A sweep anchored to list
membership alone is blind to exactly the notes nothing else is pointing at either.** So the anchor
was widened to the list's pages *and* every page of a document the note's prose argues, which is
the same widening `overtaken` makes for the same reason, and the plant was named.

The cost of the widening is stated rather than hidden: the values offered as the correction are
then drawn from every page of every document the prose names, so the nearest one may be another
document's. Every offered value is printed with its page beside it for exactly that reason.

The clean calibration is a figure the gate *confirms*, perturbed:
`worst tile at 6.73` → `7.73` in `CONTRADICTED_TIGHT_CONSENSUS`. Named on the first rung, with
`6.73` offered first, confirmed 34 → 33 and contradicted 69 → 70; restored with
`git checkout --`, both counts returned. The plant was made **before** any correction (session
645's rule) precisely because the restore would have taken the corrections with it.

## What the first run found

Figures corrected across **thirteen** notes, every one off the gate's own line — including the
first sentence of the note session 665 had already corrected, stale again within five sessions
because ADR 0492 changed what a group composites, and invisible to 665's own sweep for the reason
above. Over the same report, contradicted figures went **69 → 48** and confirmed **34 → 66**.

Four of the thirteen were *bands* over a population — the paper's 155 pages, `TAMReview.pdf`'s 22,
`standard_fonts.pdf`'s 14, `freeculture.pdf`'s 318 — re-derived from the gate's own per-page lines
rather than from a fresh measurement, which is the point of reading a log. Re-deriving the last of
them found a second thing the sweep could only point at: **`freeculture.pdf` page 255 stands
outside the band its note claims** — mean 16.63 where nothing else in the book reaches 9, a worst
tile of 51.93 against a next-highest 29.05 — and has never been opened. A band is a claim about
every member, and re-deriving one is how a member that stopped belonging is found.

**And one thing no figure could have said.** The sweep reported `AMBIGUOUS_DEVICE_N_ALTERNATE` —
a **one**-page group — quoting a band `mean 3.51 to 9.93, worst tile 20.94 to 48.31` that its one
page does not carry. Opened, the reason was not a stale number at all: forty lines diagnosing
*one paper under fifteen names* were sitting above that group's `const`, two groups away from the
list they belong to, whose own note therefore began mid-argument at *# And a second document*.
A doc comment attaches to whatever declaration follows it and both declarations are page lists, so
nothing said so and nothing could have. **That is a fourth way for a note to be wrong** — after
its name (ADR 0480), its reading, and its figures (ADR 0491): *a note attached to the wrong list.*
The lines were moved back and the band re-derived over the 155 pages it is actually about.

## The clause half: a recorded silence, in its third and fourth home

ADR 0494 found that `CONTRADICTED_CALRGB_TO_SCREEN` argued from §10.3.1's sentence putting the
*establishment* of a destination colour space beyond the document's scope, and stopped one
sentence short of the `shall` that says the conversion itself is not open. The same half-read was
still standing in two more notes, and `overtaken` had `AMBIGUOUS_ICC_MATRIX_PROFILE` at the head
of the reading list it left:

- **`AMBIGUOUS_CALRGB_TO_SCREEN`** — the same document's other eight pages, quoting the same
  sentence and concluding *the second half of the journey is each processor's*. It is not: the
  choice of destination is, the transform onto it is the referenced standard's.
- **`AMBIGUOUS_ICC_MATRIX_PROFILE`** — the same conclusion, *and a false quotation*. It attributed
  to §10.3.1 the words "[t]he characteristics of the output device", which §10.3.1 does not
  contain; they are §10.4.2.4's, about black generation on the way into `DeviceCMYK`. **A silence
  was asserted about one clause out of another clause's words**, and principle 5's rule — quotation
  marks mean verbatim — is the rule that would have caught it. Both the quotation and the reading
  are corrected, and the group's verdict stays `ambiguous` with the work it names changed: two
  evaluations of one matrix-shaper profile differing by 1.2 of 255, where the standard names an
  authority for how a profile is evaluated.

§10.3.1's ledger row carries both. `spec-errata emit` was run over the standard first, per
`doc/todo/02` §4: §10.3.1 has no erratum under its own heading — Issue #181's strike of the dated
*ISO 15076-1:2010 (ICC.1:2010)* files under §10.4.1's — which is why both corrections state the
`shall` in prose rather than as a blockquote, exactly as 0494 did.

## Why it is not a build failure

Three of the four kinds of noise it prints are correct prose: a note narrating its own correction
keeps the figure it supersedes; another instrument's table borrows the gate's words (one note in
`oracle.rs` quotes `render-quorra`'s gate, which this sweep has no right-hand side for); a range
is one endpoint to the sweep and a span to a reader. ADR 0249's ratio argument, and one of its own:
a build that failed here would teach rounds to delete measurements, and a note that loses a
measurement is worse than a note carrying a stale one.

## Owed

- **The other gates' notes.** `render-quorra/tests/corpus.rs` and
  `crates/pdf-model/tests/text_extraction.rs` keep notes in the same shape and print figures in
  their own words. The module is general; only the binary is scoped to `oracle.rs`, and pointing
  it at a second report is a second `GATE_NOTES` and a second parser.
- **Thirteen figures sit in notes the report names no page of**, and they cannot be judged at all.
- **The 42 notes `overtaken` still names**, and the 62 that cite no ADR.
- Nothing here establishes which evaluation of a matrix-shaper profile ICC.1 licenses. That needs
  the profile's tags worked through by hand, the way 0494 worked `ghostscript`'s synthesised
  `scnr` profile through.
