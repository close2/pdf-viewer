# 679 — The number that was in a report and not in the tree

First merge round of the block. Four branches, **no conflicts**, and the round spent most of its
time on a movement that turned out not to have happened — which is worth writing down, because the
way it was settled is the point.

## The sequence, whole, on a quiet machine (load 1.28)

`fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check ·
`nextest` **2441 passed, 17 skipped** · doctests · conformance 182 + 5 + 1 · corpus **974 documents,
68 incomplete** · oracle **1794 pages — 908 agrees, 65 contradicted, 786 ambiguous, 2 reference
geometry, 13 not comparable, 18 no render** · `render-quorra` **957 pages at glyph quantum 1/16 —
933 agree, 22 differ, 2 refused** · `fixed_documents` **40 checked, 0 absent** · text, both censuses,
dates, XMP, JPEG 2000 · `cargo deny` all four ok. §5's binaries rebuilt and installed.

**The ledger moved, and in the direction a round should have to argue for:**

| | before | after |
|---|---|---|
| implemented | 436 | **443** |
| inapplicable | 76 | **69** |
| partial | 224 | 224 |

875 rows, 0 unreviewed, no `silent` row. The seven are 677's §10.6.5 rows, re-decided on the
permission §10.6.5.1 *states* rather than left on an assertion — and 676's §12.7.5.5 went the other
way, `implemented` → `partial`, which is why `partial` did not move.

## The movement that was not one

`--bin quoted` printed **50 contradicted figures** on the merged tree where the report closing the
last block said **48**, and the note population read **98** where that report said **124**. Two
numbers apparently moved, and the round that changed `oracle.rs` had reported the sweep clean.

Both figures were wrong in the *report*, and the tree had never held either. 670's own history file
says **25 of 98**; the 124 came from a different round's different denominator, and the 48 was that
round's figure against a log that no longer exists.

**Settled by measurement rather than by argument**, and the measurement is the reusable part: the
same oracle log, run against the block base's `oracle.rs` and against the merged one.

| | notes quoting | figures read | confirmed | contradicted |
|---|---|---|---|---|
| base `oracle.rs` | 25 | 137 | 74 | **50** |
| merged `oracle.rs` | 27 | 142 | 79 | **50** |

675 added five figures and **all five are confirmed**, which is precisely what it reported. Nothing
regressed.

**The rule this is an instance of is already `CLAUDE.md`'s** — *a fact that can be counted is not
written down; what is written down is the command that counts it* — and this is what the rule is
*for*, from an angle the file does not state outright: **a number quoted in a report is not a
before-measurement, and comparing against one manufactures regressions that then cost a round.** A
before is the base tree measured with the after's own instrument and the after's own input. Two of
this block's four rounds independently corrected a figure their briefing had carried; this is the
third instance in one batch, and the first where the wrong number was in the *caller's* report rather
than in the tree.

It is also, in miniature, the shape the previous block was entirely about — an instrument that
cannot see the population it claims to measure. Here the instrument was fine and the *comparison*
could not see it.

## Trap 15, and the same defect made impossible

676 found that `tools/conformance`'s `root()` is `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")`
— **compiled in** — so a sweep binary can measure a tree its round did not edit and print thirteen
plausible lines while doing it. The tell is *nothing moved*. That is now `doc/traps/`'s fifteenth,
with the recipe for a real before.

It bears directly on the paragraph above and the two make one lesson: **a sweep's "before" has two
ways of being about the wrong tree — the binary's compiled-in root, and the caller's memory — and
both fail silently, in the reassuring direction.**

Separately, 677 hit the hazard `doc/environment.md` has warned about twice: `git add doc` in a
worktree whose corpora are symlinks rewrites six gitlinks from mode 160000 to 120000. It caught and
repaired that correctly. **`tools/worktree.sh` now makes it impossible** rather than documented —
`--skip-worktree` on each linked corpus, so git never compares that path against the working tree
and `add` leaves the gitlink alone. Measured both ways before the line was written: without it,
`git add doc` writes 120000; with it, 160000 survives. *A hazard a document warns about is a hazard
every future round has to remember.*

## The four rounds

**675 — the eight groups are three buckets, not one.** 672's sixth criterion, pointed at the eight
groups it named. Six taken, fifteen pages, landing in three places: three account for their failing
bound once their figures are converted into the gate's units, two need an ablation to show it, one
accounts for **none** of it. That one is `CONTRADICTED_NEGATIVE_LINE_WIDTH`, whose note's ink ladder
predicts the gate's mean to four decimals (0.6366 against 0.6366) — **and the mean is a bound the
page passes**. Restating `-0.1 w` as the `0` §8.4.1 mandates leaves `mupdf`, the reference the verdict
is taken from, byte for byte unmoved. §8.4.3.2's one-device-pixel `shall` owns both failing bounds,
and **ours is the only one of four renderers obeying it**. Third round running where the deciding
clause was not the one the group cited. Its arithmetic finding travels furthest: `raster_compare`
divides by width × height × **four** channels and two notes had that factor wrong, while two more
called `Distance::of`'s output "levels of 255" when it prints a ratio against the page's own bounds —
**a ranking whose unit is misread cannot be checked against anything**, which is why no sweep had
caught it.

**676 — the entry a row declared unread.** §12.8.2.2.1 is a false negative: 1 curated document binds
a `/DocMDP`, **143 crawled ones do, 122 at `/P` 1** — the level withholding both operations,
previously exercised only by a fixture. Nothing was owed beyond the count, because the `shall`s fall
on a producer and the one reaching a reader is already in `CLAUDE.md` §3's shape. One negative
*survived* and was probed rather than believed: no document anywhere states a `/DocMDP` whose level
we cannot read — which matters because Table 255 makes that `/Reference` an array while Table 263
describes it in the singular, and our reader accepts only the array. A planted file of that shape
proved the blindness before the zero was trusted. **§12.7.5.5 is the round's judgement call**: the row
disposed of Table 236's `/P` in one sentence of the entry's seven, three of the others address a
processor that *changes* the file, 28 crawled documents state it — and the round measured it, argued
it, and **deliberately did not implement it**, because the entry states permissions with no binding
route where §12.8.2.2.1 gives one explicitly, so the weaker reading would add a default refusal on 28
real documents. Row moved to `partial` naming what is not executed. And **errata issue #131 amends
that entry** and had never been recorded, because a row saying an entry is unread is a reason not to
open its page.

**677 — the entry a halftone carries that is not a halftone.** The question was whether §10.6's
inapplicability carries §10.5's sentence about a halftone dictionary's `TransferFunction`. It does
not, and the standard separates the two in the place this project had joined them: §10.1's steps make
only one conditional on the device — "*If* the raster output device supports PDF-defined halftoning,
apply halftoning according to 10.6" against an unqualified "For any object for which transfer
functions are in effect, apply those transfer functions" — and §10.6.1's sentence *excusing* a screen
from halftoning is the same sentence that charges it: "Halftoning is not required for such devices;
**after gamma correction by the transfer functions**, the colour components shall be transmitted
directly to the device". So §10.6's inapplicability is a screen's, the dictionary is the carrier, and
the entry is §10.5's. Built for all three shapes of `/HT`, each derived: `/Default` **removes** an
override, a Type 5 dictionary is read per colourant, anything else governs all three, and
`TransferFunction /Identity` is an override rather than a silence. **The census counts `/HT` by two
instruments and they disagree over the crawl** — the resource walk misses documents whose
`/ExtGState` no page or form reaches, which is the false zero probed for rather than assumed. Three
documents in the whole crawl carry a `TransferFunction` and every occurrence is `Identity`;
`raster_digest` is byte-identical with the reader switched off; all four fixtures fail against the
tree without it. And **a struck erratum nearly became the argument**: the obvious authority for "a
non-Type-5 halftone governs all components" is a §10.6.5.6 sentence erratum #311 deletes entirely,
which `spec-errata check` cannot see because the extracted words run together (`graphicsstate`).

**678 — the first UI round.** Both premises it was given were wrong about the file: the gap colour
has been closed since ADR 0446, documented *as a choice* with the search establishing the standard
says nothing, and the `/TI` binding floor was not the blocker. Raising `gtk4` to `v4_12` and calling
`GtkListView::scroll_to` moved nothing at any `/TI` — GTK documents that call as putting an item
*into view*, and `GtkScrollInfo` carries two booleans about which axes may move and no alignment;
Qt's `PositionAtTop` has no counterpart. The floor went back to `v4_10` (a floor costs a runtime
requirement) and the entry drives the scrolled window's own adjustment from an idle, because a
`GtkListBase` recomputes that adjustment from its anchor at every allocation — found by tracing. The
corpus witness the file said did not exist is `annotation-choice-widget.pdf` object 62, missed
because the census behind the claim counted list boxes and not `/TI`. **And the ordering the owner
asked for**: seven ranked items with a stated criterion, resting on three findings — "all three hosts
stay level" is false in both directions and there are **four** consumers; **no host can copy a
selection out of the program**, ranked first and needing no new message; and "the ABI's entry points
are the whole vocabulary" has decayed, so `tools/state.sh hosts` counts it — every `Command` reaches
the ABI, **19 of 31 `Query` variants** do, and a C caller can run Annex O's search and cannot draw a
match.

## Owed

- **`SUBSTITUTED_FONT` and `DEVICE_CMYK_CONVERSION`**, 13 pages, the two of the eight 675 did not take.
- **A voting reference whose raster is constant contributes nothing to a verdict, and the gate lets
  it vote.** The JBIG2 group's whole verdict line on four pages reproduces digit-for-digit against a
  synthetic *white sheet* — no renderer that drew that image could meet any bound. Refusing constant
  voters moves pages between four lists at once, so trap 11 makes it its own decision.
- **The negatives queue: 17 done, 28 owed** — the instrument's count, not 671's.
- **`freeculture.pdf` page 255**, still never opened, outside its note's band at mean 16.63.
- **The owner's `git stash drop`** — the one entry is verified dead (`doc/environment.md`), and this
  account cannot drop it.
