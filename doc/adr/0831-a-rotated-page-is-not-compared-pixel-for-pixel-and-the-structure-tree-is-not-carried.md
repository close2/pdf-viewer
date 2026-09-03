# 0831 — A rotated page is not compared pixel for pixel, and the structure tree is not carried

Session 893. Status: **accepted**. The eleventh decision record of RFC 0002's implementation, and
two honest statements the `pages` verb forced: one about what its gate can assert, and one about
what no verb of this suite carries.

## Context

RFC 0002 §9 gives a transform four layers of correctness and names the third the load-bearing one:

> render page k of the output and its source page with the same backend at the same scale —
> `render-cpu`, the correctness oracle — and require **bit-identical** rasters for lossless
> transforms (split, merge, pages without rotate), a stated `raster-compare` tolerance for lossy
> optimize, and the rotation-transformed comparison for rotate.

`split_corpus.rs` and `merge_corpus.rs` are the first clause of that sentence over the corpus.
`pages_corpus.rs` is the first walk to reach the last clause, and the RFC's phrase "the
rotation-transformed comparison" turns out to name a question rather than an instrument.

The second subject is older. Session 888 named it as the suite's largest debt: **no verb carries
§14.7's structure tree.** `pages` is the verb where a reader is most likely to expect it to
survive, because nothing about deleting one page of a tagged document suggests the other pages
should lose their tagging. So it is stated here rather than left implicit.

## Decision

### 1. The rotated page's raster is measured, printed, and not asserted on

The obvious construction is: draw the source page, turn the *raster* a quarter turn clockwise —
which is what §7.7.3.3 says the page now is — and require the rotated page's raster to equal it.
Measured over the corpus, that comparison fails on 905 of 905 rotated pages, for two reasons that
are both the renderer's and neither the writer's.

**The grid turns with the page.** A page turned a quarter turn is scan-converted on a grid at
right angles to the one it was on, so a glyph edge that covered a pixel 6 % on one grid covers it
8 % on the other. `issue15150.pdf` is the smallest witness in the corpus: a 7 × 7 raster whose one
non-white pixel reads (255, 239, 239) before the rotation and (255, 234, 234) after — five levels
of 255 on two channels, with the pixel in exactly the place the rotation puts it, so the geometry
is right and only the coverage moved.

**The leftover sliver changes edges, and it is worth a whole pixel.** A page `W` units wide at
scale `s` is `ceil(W × s)` pixels wide, and the strip between `W × s` and that ceiling is raster
the page does not reach. Turn the *page* and the strip is on the right of the new raster; turn the
*raster* and the same strip is at the top. So the two disagree by up to one whole pixel of
placement — which the corpus confirms exactly: on `issue2761.pdf` the turned source and the
rotated page agree to a mean absolute difference of **0.000** once one column is allowed for, and
to 19.4 levels without; `issue4398.pdf` reads 0.019 against 0.132 and `bug1146106.pdf` 0.008
against 0.938, each with the same one-column offset and no other.

`CLAUDE.md` names this case as one the standard leaves open — "how a fractional page becomes a
whole number of pixels" — so what shows through here is the renderer's documented choice, not a
defect in what `pages` wrote. A gate that asserted on it would be asserting on that choice, and
would fail the day the choice changed for a better reason.

So the walk **measures** it and prints the distribution — worst tile error 43.40, least similar
tile −0.4325 over 905 pages, on ADR 0755's two figures — and asserts three other things instead,
each of which is exact:

- **The round trip.** `+90` and then `−90` over the same page writes the value the page had, and
  its raster is bit-identical to the source's, on the *same* grid, with no rotation of a raster
  anywhere in the comparison. Zero failures over the corpus. This is the statement about the
  *writer*: the integer arithmetic is reversible and the renderer honours what was written.
- **The dimension swap**, which is §7.7.3.3's own claim about a quarter turn: the rotated page's
  raster is as wide as the source's was tall. The rounding above moves content by a pixel; it does
  not change the count of them, because the same ceiling is taken of the same two numbers in the
  other order. Zero failures.
- **Bit identity for every page the plan did not rotate**, which is RFC §9's "pages without
  rotate" class in as many words. Together with the content-stream check — every carried page's
  `/Contents` byte-identical to its source page's — that is the whole of the appearance claim for
  a delete and a reorder.

What is left owed is the aligned comparison: turn the raster, allow the sliver's whole-pixel
offset that the *renderer's own* rounding implies rather than one searched for, and then assert
the tolerance. That is a change to how `render` reports what it drew, not to this walk, and
`doc/todo/57` carries it.

### 2. §14.7's structure tree is not carried, and the honest statement is that the key names nothing

§14.7.1 states the shape of the thing:

> A PDF document's logical structure shall be stored separately from its visible content, with
> pointers from each to the other.

Two halves, and this suite carries neither. The catalog's `/StructTreeRoot` is in `merge.rs`'s
`NOT_CARRIED` list, so a source that states one is named in a warning and the output states none.
The other half is Table 31's entry on the page:

> ( Required if the page contains structural content items; PDF 1.3 ) The integer key of the
> page's entry in the structural parent tree ( see 14.7.5.4, "Finding structure elements from
> content items").

and a carried page still states the integer its producer wrote.

**With no structure tree in the output, that integer names nothing at all.** §14.7.5.4's parent
tree does not exist, so there is no entry for the key to be wrong about. This is the distinction
worth keeping, because the two outcomes are not equally bad: a key into an absent tree is a
dangling reference an assistive processor finds nothing for, while a key into a *partial* tree
built by a verb that carried some fragments would name **another page's** structure element, and
a reader would be told the wrong thing with no way to know it. Half a structure tree is worse than
none.

So the integer is left as the producer stated it — dropping it would be a second edit to the page
dictionary in service of a construct this verb does not write, and a page that does contain
structural content items is required to have one — and the warning names the tree as not carried,
every time, on every verb. The corpus walk asserts that the output states no `/StructTreeRoot` at
all: 0 of 966 edited documents carried one, which is the check that the suite has not started
half-carrying it by accident.

A tagged document therefore loses its tagging to any verb of this suite. `doc/todo/57` has carried
that as the largest single thing the suite owes since session 888, and this record is the reason
it is not being paid down a fragment at a time.

## Consequences

- `crates/pdf-transform/tests/pages_corpus.rs` is the walk, under `tools/bounded.sh --data 4
  --tree 12`. It asserts contents, labels, the absent structure tree, the round trip, the
  dimension swap, determinism and no panic; it measures the rotated comparison.
- `raster-compare` becomes a dev-dependency of `pdf-transform`, for the measurement alone. The
  tree's own instrument rather than a second one written in a test.
- The one-pixel sliver offset is a *finding about `render`*, not about the writer, and it is
  written down here so that a round reaching it starts from the measurement rather than from the
  surprise. It is also a reason the oracle's own cross-backend comparisons should be read twice on
  a page with a non-zero `/Rotate`.
- §14.7's ledger row records the two-sided reading — the tree absent, the key dangling and
  deliberately not dropped — so that the debt is legible from the ledger and not only from a todo.
