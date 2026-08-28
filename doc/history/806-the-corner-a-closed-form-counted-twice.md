# 806 — The corner a closed form counted twice

Branch `round-806` off `main` at `5efead7b`, in `.claude/worktrees/r806`. Siblings 804, 805 and
807 ran beside it; diffs disjoint. ADR 0738 has the argument.

## What the round was handed

The eight-hundred-and-third session's merge sweep found ADR 0735 overtaking three page-list notes
and filed the reading as owed. Two things, in order, the second only if the first earned it:
read `AMBIGUOUS_TILING_CELL_CLIP`, `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` and `DIFFERS_IN_SHAPE`
against what the tree now does; then re-measure the ambiguous pool's ink tail, whose head
`issue16038.pdf` has been at −5.642 for many rounds.

## What the notes said and now say

None of the three had a wrong sentence about ADR 0735, and the reason is a fact about the
documents rather than a judgement about the change. `issue12295.pdf` states no `/Pattern` and no
`SCN` at all — checked on the file expanded by `qpdf --qdf --object-streams=disable`, because a
grep over the original is a claim about compression. `issue16038.pdf` installs its cells with
`scn` under a `B` whose stroking colour is a flat `0 G`, so it takes the fill arm, which ADR 0735
left unchanged in every particular. Each note gains the sentence saying so with its evidence, and
cites 0735 and 0738.

**One sentence elsewhere in the tree *was* wrong, and no instrument could see it.**
`pdf-render`'s `outline` module enumerates its own callers — "once per pattern for ADR 0155's
containment test, and once per command per *cell* for `repeat`'s fold" — and ADR 0735 added a
third, `pdf_model`'s `tile`, once per patterned stroke. `--bin parts` reads cardinals about
crates, backends and workers, not a prose enumeration inside one. The third caller also asks a
third question: the first two ask containment, this one asks reach.

## What the tail's measurement shows

`doc/todo/00` step 7 over all 835 measurable ambiguous pages, on the file's own recipe: 19 at or
past −1, 16 of them incomplete, and on the complete documents `issue16038.pdf` −5.642,
`issue12295.pdf` −2.362, `issue14297.pdf` −1.135, then nothing past −0.957. The head is the
eight-hundred-and-second session's to the thousandth, and the mechanism is unchanged.

**The finding is underneath it.** That note's closed form — the ink the document asks for, with no
renderer in it — has been 316.29 square points since the three-hundred-and-seventy-fourth session,
and it counts 3.18 of them twice: each rule runs to the square's edge and the border is a stroke
of that same path, so the two share `w²` per rule. The area is **313.117**, and this tree deposits
**313.016** at 24× — a thirtieth of a percent. So the −5.642 is the references' excess entirely
(the nearest is 17% over the area, the furthest 204%) and not our shortfall, which is what
`doc/todo/00`'s own paragraph on this page has argued for four hundred sessions without a limit of
ours to hold it to.

ADR 0226's owed 1× column was taken at the same time, by a band whose ink is exact under any
pixel snapping, and recorded as its own instrument rather than as the old row continued.

**And a lesson about the instrument, paid for in this round.** Run with a greyscale of one's own
instead of the recipe `doc/todo/00` prescribes, the same artefacts put `issue16038.pdf` at −5.394
and `issue12295.pdf` at −2.364: the first page's rules are pure blue and every greyscale weights
blue differently. A quarter of a level is the size of the movement this sweep is watched for.

## What was touched

- `crates/pdf-model/tests/oracle.rs` — two notes.
- `crates/render-quorra/tests/corpus.rs` — `DIFFERS_IN_SHAPE`.
- `crates/pdf-render/src/outline.rs` — the caller enumeration. The round's only change under a
  `src/`, and a module comment.
- `crates/pdf-model/tests/tiling.rs` — one new test,
  `the_page_that_is_a_closed_form_weighs_what_the_closed_form_says`, calibrated twice.
- `doc/conformance/ledger.toml` — §10.7.4 gains the page where its departure (1) is measured
  against a closed form rather than argued. The row was `partial` and stays `partial`; §8.7.3 and
  §8.7.3.1 are `partial` and untouched.
- `doc/todo/00-ambiguous-bucket.md`, `doc/adr/0738-…`, this file.

## Gates and sweeps

The whole `doc/todo/02` §2 sequence, before the first edit and after the last, in this worktree.
The §4 sweeps before and after against the same baseline. `doc/todo/00` step 7 both times. The
numbers are in the round's report; nothing here repeats them, because a document that carries a
gate's figure is how a round writes "unchanged" without running it.

## Left standing

- `doc/rfc/` still awaits the owner's review, untouched.
- CI on `origin/main` was already red before this branch existed.
- `--bin quoted` still reports `AMBIGUOUS_TILING_CELL_CLIP` quoting four figures the oracle does
  not print. All four are `render-quorra`'s gate or a superseded movement, and the note already
  says which; it is the sweep's own documented noise and not a correction owed.
- The interior-coverage table's older four-scale row is left where it is, beside the new one, with
  a sentence saying they are two instruments. Merging them would manufacture a movement.
