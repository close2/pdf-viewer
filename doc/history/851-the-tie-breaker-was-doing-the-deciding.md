# 851 — The tie-breaker was doing the deciding

2026-09-01. A clause round on the `owed` reading list, with its demand-side half in the same
clause family.

## What was taken

`cargo run --release -p conformance --bin owed` prints the `partial` rows whose every stated term
this tree already names — the rows nobody has opened since their debt was written. Ranking that
list by when each note was last written (`git blame` on the `note` line, which is what the
four-hundred-and-forty-second session's ordering was) puts §8.7.4.5.7 and §8.7.4.5.8 — the Coons
and tensor-product patch meshes — among the three oldest. Both said the same short thing, and both
were `test = ["crates/pdf-model/tests/shadings.rs"]`: the whole file, which held no type 6 or type
7 fixture at all.

## Rows opened, and where each ended

| row | before | after |
|---|---|---|
| §8.7.4.5.7 Type 6 (Coons patch mesh) shadings | `partial` | `partial`, corrected — a defect fixed, a reading recorded as a choice, and a named test |
| §8.7.4.5.8 Type 7 (tensor-product patch mesh) shadings | `partial` | `partial`, corrected — Tables 84 and 85 checked index by index, the same defect, the same test |

Read and left as they stand, each checked against its clause and its code rather than only against
its own sentence: §8.7.3, §8.7.4.1, §7.7, §11.4.1, §11.4.3, §11.4.8, §12.8.2.1, §12.8.4.1, §12.9.1,
§12.10.1, §12.11.6, §14.7.4.2, §14.8, §14.8.2, §14.8.6, §14.8.6.2, §14.8.6.3, §14.13.8. Every one
of them is `partial` for a debt that is still owed, and several have already been re-read by an
earlier round and say so. The reading list is not exhausted; what it has left near the top is rows
whose debts are real.

## The defect

§8.7.4.5.7 decides a fold-over, and neither row had read the sentence: "[i]f more than one point
( u, v ) in parameter space is mapped to the same point in device space, the point selected shall
be the one with the largest value of v . If multiple points have the same v , the one with the
largest value of u shall be selected." Every rasteriser here paints a mesh's triangles in the order
`mesh::tessellate` returns them and each overwrites what is under it, so the emission order *is*
the precedence — and the loops nested `u` outside `v`, which ranks `(u, v)` where the clause ranks
`(v, u)`: its tie-breaker promoted over its rule. The clause's other overlap sentence, one patch
over another, already held.

One loop nesting. ADR 0778 has the argument, the witness construction — a patch whose control
points satisfy `p(i,j) = p(j,i)`, so `S(u,v) = S(v,u)` and every point off the diagonal is covered
by both orderings at once — and why the three reflex-quadrilateral fixtures tried first produced
zero differing pixels.

## The demand side

`examples/raster_digest` over `doc/pdf.js/test/pdfs`, both arms in one sitting with `touch` on the
changed crate (trap 10b) and the same release worker on disk: of 975 documents, **exactly one page's
pixels move** — `tensor-allflags-withfunction.pdf` page 1, which is one of
`AMBIGUOUS_BOUNDARY_PIXELS`' three. Its verdict stays `ambiguous`, and the group note's claim was
re-measured rather than assumed: rows 203 and 492 are still 0 ink and 0 non-white pixels on both
that file and its Coons sibling, because the split that note is about is over the mesh's top and
bottom *edge* rows and a precedence between two preimages of an interior point cannot reach an
edge. The note records that, with the ADR, so it stays off `--bin overtaken`.

Two clause numbers in the same note were wrong by one — the Coons mesh cited as §8.7.4.5.6 and the
tensor-product one as §8.7.4.5.7 — and are corrected.

## Also recorded

A patch's data is not padded to a byte boundary here, and that is now written as a *reading* with
the two sentences it rests on rather than as a bare assertion. Nothing on this disk decides it:
across `doc/pdf.js/test/pdfs` and the four `doc/corpora/` submodules, 1249 files, there are nine
distinct type 6 and type 7 shadings and every one states `/BitsPerFlag 8` over whole-byte
coordinate and component widths. A file with `/BitsPerFlag 2` is the witness that would.

## Gates

The whole of `doc/todo/02` §2, twice over the parts a later documentation edit could reach — the
change is in `pdf-model`, so it can move a pixel and the sequence is owed whole. All green, no
ratchet moved. `--bin quotations` and `--bin pointers` run after the sequence; neither names
anything this round added. §5's binaries rebuilt and installed, the directory derived from
`cargo metadata` rather than written down (trap 15).

The new test is calibrated the way trap 13 asks: the old nesting was put back and the test failed
naming what it saw, `246,0,9` where the clause asks for blue.
