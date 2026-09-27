# 1341 — A path whose portions overlap is measured as the set its rule declares inside

Status: accepted. Session 1252.
Context: ISO 32000-2 §10.7.4, §10.7.1, §8.5.3.3.2, §8.5.3.3.3, §11.6.2; ADRs 0476, 0583, 0590,
1082, 1102.
Code: `crates/render-cpu/src/area.rs`, `crates/render-cpu/src/scan.rs` (`Exact::Outline`,
`fill_outline`), `crates/render-cpu/src/lib.rs` (`draw_stroked_outline`).
Tests: `crates/render-cpu/tests/overlapping_portions.rs`, `area.rs`'s own.
Documents: §10.7.4's ledger row, `doc/todo/11` item 7, `doc/todo/65`.

## 1. What the clause says about a pixel two portions share

§10.7.4 has no sentence about a *shared* pixel. What it has is the order of operations: "At this
level, curves have been flattened to sequences of straight lines, and all "insideness" computations
have been performed." Its rules apply to a shape whose inside §8.5.3.3 has already decided, so under
this tree's anti-aliasing (§10.7.1's NOTE) a pixel is covered by the area of **the set** the fill
rule declares inside — the union of two same-wound squares under the non-zero rule, their union less
the overlap under the even-odd rule. §11.6.2 forbids the other reading outright: portions of one
object "shall not be composited with one another".

`render_cpu::area` (ADR 1082) sums the integral of the winding **number** over each pixel. That is
the set's area only while the winding stays inside `0..=1` or `-1..=0`; where two portions overlap
at a boundary pixel it reads up to twice the area, and where two opposed portions share a pixel their
integrals cancel. ADR 1082 declined such a mark — a cell past one whole winding — to `tiny-skia`'s
supersampled converter, which applies the rule per sample and measures to a sixteenth.

## 2. The construction

A row that may part from the set is cut into sub-strips at every height where an edge begins, ends
or crosses another. In a sub-strip no two edges cross, so they stand in one left-to-right order;
walking it with the running winding finds where the rule's answer changes, and only those edges are
deposited, with unit weight, through the same closed form the accumulator is made of. The row's sum
is then the indicator's integral: the set's area, exactly, under either rule. A row whose winding
never left the two ranges keeps the accumulator's bits, so the decision is per row and a strip's
answer is the whole page's.

**Which marks are walked.** A mark any of whose rows shows a cell or a pixel past one whole winding,
or integrals of both signs, is walked whole — an overlap confined to partly covered pixels shows no
sign in its own row. And a stroker's outline with a join or a second subpath is walked whole
whatever it shows: `render-raster/examples/ink_ladder` found the thin strokes of `issue20232.pdf`
reading 20 039 at 1× against 17 866 at 8× with the first rule alone, and 17 932 with this one. A
single straight segment's outline covers no point twice and is not walked. What stays unseen is an
overlap confined to pixels a path that is not an outline only partly covers, with no whole winding
of two in its row; the error is bounded by the overlap's own area, and `area.rs` names it.

`SET_WORK` bounds the walk at 2^24 pair tests and placements per mark; past it the library draws, as
before. It is a cost guard, not a condition.

## 3. What it moved and what it costs

`raster_golden`: 267 of 974 first pages, every one `raster only`, median 0.03% of a page's ink.
Each page looked at lands on its own 2× to 8× ink at 1× — `issue6081.pdf` 68.85 → 53.98 against
53.73, `multiline.pdf` 1059.86 → 1103.56 against 1103.56, `issue20232.pdf` 19 324 → 17 932 against
17 866 — and on those crops the 1× raster is within a third of a level of the 8× raster averaged
down, where it was 1.4 and 6.2 levels off. `render-raster --test corpus`: `issue21068.pdf` left the
differing list. `callgrind_rasterise`, one thread, five draws: ISO 32000-2 page 101 −2.7%,
`issue12295.pdf` −0.03%, `issue20232.pdf` +9.3%, `issue14415.pdf` +47%, `issue19802.pdf` +50% —
the last two dense with joins that overlap, where every row of more than two edges is walked.

## 4. What the row is

§10.7.4 stays `partial`, and not for this: §10.7.4's own mark for a shape its *transform* collapsed
(`doc/todo/11` item 8) and the clipping paragraph's product at an image's edge and inside a group
whose opacity is below 1.0 (item 4) are requirements not yet executed. `departed` is one decided
departure inside an otherwise executed clause; this row has residues that are not decisions, and
its four documented choices under §10.7.1's NOTE are not what keeps it open.
