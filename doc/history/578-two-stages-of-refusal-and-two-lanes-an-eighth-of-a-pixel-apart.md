# 578 — Two stages of refusal, and two lanes an eighth of a pixel apart

**The finding**: the quorra gate's refusal lists held two unrelated kinds of refusal in one array,
so a name leaving one meant either "the adapter grew a capability" or "this tree's translation
grew a construction" and the ratchet could not say which — and it made this tree's own two names
live in *two* copies, one per scale, which is how one of them spent twenty rounds missing from the
4× list. Split along the stage. Then, with an instrument built for it, the four pages where the two
coverage lanes' 23-page differing sets fail to overlap turned out to be one population: axis-aligned
rules about a device pixel wide, whose ink both lanes get right and whose *placement* neither
does — the default lane by up to ⅛ of a device pixel per drawing command, the gpu lane by
quantising some marks' y coverage. All four are quorra's, because the display list is the same one.

Date: 2026-08-18. Argued in
[ADR 0413](../adr/0413-two-stages-of-refusal-and-two-lanes-that-place-a-mark-differently.md).

Touched: `crates/render-quorra/tests/corpus.rs` (`REFUSED`/`REFUSED_AT_FOUR` →
`REFUSED_BEFORE_THE_SCENE`, `REFUSED_BY_THE_DEVICE`, `REFUSED_BY_THE_DEVICE_AT_FOUR`, plus
`refused_pages` and a failure message that names which half moved),
`crates/render-quorra/examples/lane_diff.rs` (new), `doc/QUORRA_FEEDBACK.md` (§31),
`doc/conformance/ledger.toml` (§14.8.5.4 `inapplicable` → `partial`, §14.8.5.4.1 read and kept),
`crates/pdf-model/src/image.rs` (a dead `doc/todo/47`), `doc/performance.md` (three pointers),
`doc/todo/54-…` (items 1 and 4 closed), `doc/todo/README.md`, `doc/adr/0413-…`, this file.

No pixel moves and no page moves.

## The split, and what it is a split along

`REFUSED_AT_FOUR` held three names. `issue1905.pdf` is refused inside quorra, at render time,
because the coverage sheet exceeds this adapter's texture limit. `bug1721218_reduced.pdf` and
`issue18032.pdf` are refused by this tree, before a scene exists, for §11.6.6/§11.7.2 and §11.4.6.
The first is a capability with a device behind it; the other two are constructions with a clause
behind them, and no upstream release can move one.

The array is three now — one scale-free, two per-scale-and-per-device — and each carries what a
departure from *it* would mean. The empty one is the one worth having: at a page's own scale this
adapter refuses **nothing**, and an empty array with that sentence on it is a statement, where the
absence of an array is an omission.

`REFUSED`'s own shape was checked before the shape was chosen, which the todo asked for: its two
names are both this tree's, so it was flat rather than flattened. It gets the same split with the
device half empty, so the two places have one shape.

## The four pages

`examples/lane_diff.rs` puts **one** display list through the oracle and both lanes and prints all
three comparisons. That is the whole reason the residue survived: the gate renders one lane per
invocation and writes an artefact only for the pages that lane differs on, so each of these four
had a picture of one lane and none of the other.

| page | oracle vs cpu lane | oracle vs gpu lane |
|---|---|---|
| `bug1743245.pdf` | mean **3.0919** ssim 0.98770 | mean 0.7339 ssim 0.99544 |
| `issue21068.pdf` | mean **2.5797** ssim 0.98413 | mean 1.0939 ssim 0.99208 |
| `bug1863910.pdf` | mean 1.1668 ssim 0.99252 | mean 1.3679 ssim **0.97911** |
| `issue16500.pdf` | mean 0.3863 worst **4.91** | mean 0.4092 worst **7.30** |

Then the pixels, which is where the answer is. `bug1743245.pdf` is graph paper, one `q … cm … S …
Q` per rule; the centroid of each rule's coverage along one raster row is 33.000, 49.500, 66.000,
82.500, 99.000, 115.500 in **both** the oracle and the gpu lane — a pitch of 16.500 against the
document's own `52.0277778 × 0.317180616` — and 33.122, 49.602, 66.083, 82.567, 99.047, 115.531 on
the default lane. `bug1863910.pdf`'s two identical widget borders take +0.103 and +0.078, each
constant within its own box: the offset is per drawing command, which is what makes a grid of
one-rule commands look like a scale error.

The gpu lane's is in y and is a different animal: `bug1863910.pdf`'s box rule splits 0.247/0.753
across two rows in the oracle and **0.500/0.500** on that lane, and `issue16500.pdf`'s table rule
is 0.439/0.439 in the oracle and **0.753/0.000** — the same mark 14% lighter and inside one row.
On the other axis of another page the same lane is exact to three decimals, so it is not "the
sampled lane is approximate"; it is per command.

**Which lane is right**: the gpu lane on the first two, demonstrably, because it reproduces the
document's own arithmetic; the default lane is merely closer on the other two and neither is right.
Every one of the four is quorra's, and none is a wrong picture — page ink agrees to under 1% on all
four. `doc/QUORRA_FEEDBACK.md` §31 carries it with two questions and no ask that costs a release.

## The sweeps

Seven run: `blockers`, `capabilities`, `pointers`, `owed`, `counts`, `inapplicable`, `quotations`.

`counts` printed four contradictions and all four are the known shape — a cardinal governing
something that is not a family of rows ("Errata Collection **3**", "**nine** rows" of a table,
"**49** rows" a ladder moved). §9.7's "sixteen rows below" was checked against the ledger and is
sixteen.

`pointers` paid, twice. `crates/pdf-model/src/image.rs`'s `RASTER_BUDGET` cited a `doc/todo/47`
that **the same commit which wrote the sentence had deleted** — the pointer and its subject closing
together, which is that sweep's own subject; it cites ADR 0374 now. And `doc/performance.md`'s two
`corpus.rs::REFUSED_AT_FOUR` symbols plus its `REFUSED` sentence went dead the moment this round
renamed them, which is the cheapest possible moment to find that out.

`inapplicable` paid on its stated discriminator — the **cousin**. §11.7.5.2 was read and is
correct. **§14.8.5.4 was not.** It was `inapplicable` on "[t]he appearance is already drawn;
nothing here can change it, and nothing in this program exports", which is true of §14.8.5.4.2's
eight attributes and of ten of §14.8.5.4.3's thirteen — and §14.8.5.4.3 has been `partial` since
ADR 0301, because `/BBox` is read and `viewer_accessibility::tree::place` puts it on AT-SPI for the
61 elements of 132 that state one and mark no text. An `inapplicable` parent, whose status the
ledger defines as *nothing is owed*, above a `partial` child naming three files and two tests. The
family's parent row is not maintained by the session that implements one of its members, because
the clauses do not cite each other — `doc/todo/01`'s fifth shape, and its seventh sweep's seventh
failure shape. `partial` now, with the exception named.

§14.8.5.4.1 was read in the same sitting and **kept** `inapplicable`, which is the other half of
doing this honestly: its one `shall` binds whoever *defines* an attribute object, and this program
defines none; the layout attribute it does read is reached through §14.8.5.3's priority rather than
through this clause's classification.

## Gates

Everything `doc/todo/02` §2 lists, all green after the last edit: fmt, clippy silent across the
workspace, 2 147 nextest tests, the doctests, the corpus gate at 974 documents with 66 incomplete,
the oracle at 1 794 pages — 906 agree, 67 contradicted, 786 ambiguous, reference-render cache at
99.8% — both text-extraction gates, dates, XMP, JPEG 2000, the quorra corpus gate at 957 pages
compared with 932 agreeing, 23 differing and 2 refused, and the conformance gate. The gpu lane was
run separately for §31's measurement and is not a ratcheted gate; it reports the same 23/2.

The conformance gate failed once, on my own edit, and the failure was the right one: a `partial`
row must name its `code` and its `test`, and the parent had inherited neither. Fixed and re-run
before anything was written down.
