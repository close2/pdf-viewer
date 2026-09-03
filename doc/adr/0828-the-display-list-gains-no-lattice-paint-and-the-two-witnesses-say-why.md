# ADR 0828 — The display list gains no lattice paint, and the two witnesses are what says so: one draws byte-identically whether the budget cuts it or not, the other is cut of 1.087 levels, and nine tenths of what drawing them whole costs is rasterisation

Status: accepted. Session 891.
Clauses: ISO 32000-2 §8.7.3.1 (the cell "replicated at fixed horizontal and vertical intervals",
painted "as many times as necessary to fill an area", Table 74's `/TilingType`); §11.6.7 (NOTE 1's
conditional permission, read in ADR 0827); §11.6.2 (a single object's portions).
Code: none. `crates/pdf-model/src/content/pattern.rs`'s `MAX_TILE_COPIES` doc comment carries the
measurement.
Tests: `doc/checks/fixed-documents.toml`'s two new rows —
`corpus-cache/safedocs/cc-main-2021-31/2760/2760154.pdf` and
`corpus-cache/tika-issue-tracker/batch5/PDFIUM/PDFIUM-1497-2.pdf` — run by
`cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored`.
Documents: `doc/todo/49` (the item, closed), `doc/conformance/ledger.toml` §8.7.3.1 and §11.6.7.

## The question

`doc/todo/49` had one open item under "two bounds count the wrong quantity": that
`MAX_TILE_COPIES` cuts two documents whose tilings want more copies than any budget in commands
can afford at a page turn, and that what they want is a display-list paint carrying a cell plus
its lattice, which the rasteriser replicates. ADR 0827 disposes of the premise — no note of the
standard suggests it, and §11.6.7's NOTE 1, which comes closest, is informative and conditional.
What is left is an engineering question, and it is answered here with a measurement.

## What the two witnesses cost, before and after

Both arms built in one sitting from the same tree, the second with `MAX_TILE_COPIES` and
`MAX_OPERATIONS` raised to 10⁹ in a scratch build (trap 29: at the constants themselves, which is
where the code reads them), run alternately on an idle machine through
`cargo run --release -p pdf-model --example open_one -- <file> 1.0`, four repetitions each, best
of four quoted; peak resident over the process tree from `tools/bounded.sh`; ink is
`tests/fixed_documents.rs`'s own scalar, the mean of 255 − Rec. 709 luma over page one at scale 1.

| | commands | interpret | **total** | peak | ink | reports |
|---|---|---|---|---|---|---|
| `2760154.pdf` cut | 67 676 | 43 ms | **0.33 s** | 0.02 GiB | 33.583 | `MAX_TILE_COPIES` |
| `2760154.pdf` whole | 765 191 | 378 ms | **2.08 s** | 0.42 GiB | 34.670 | — |
| `PDFIUM-1497-2.pdf` cut | 276 157 | 103 ms | **1.87 s** | 0.19 GiB | 11.9049 | `MAX_TILE_COPIES` |
| `PDFIUM-1497-2.pdf` whole | 276 157 | 601 ms | **10.53 s** | 0.93 GiB | 11.9049 | — |

Two things in that table decide the item.

**`PDFIUM-1497-2.pdf` draws a byte-identical raster either way.** The two PNGs have the same MD5;
`examples/compare_rasters` reports mean 0.0000, worst tile 0.00, differing 0.0000%, SSIM 1.00000.
Its two largest tilings are 448 632 and 389 205 sites of a four-command cell and the budget affords
16 384 sites of it, so 96% of the sites it states are not drawn — and the floor plan, its frame,
its dimension chains and its title block are the same page to the byte. Eight and a half seconds
and three quarters of a gibibyte buy nothing at all on this document. It is the *worse* of the two
witnesses by cost and the one with no defect to fix.

**`2760154.pdf` is cut of 1.087 of 255.** 33.583 of ink against 34.670, a mean difference of
0.8167 with a maximum of 11 levels over 7.73% of the pixels, all of it in the pale blue wash behind
the poster's title. That is a real departure from "as many times as necessary" and it is what the
report exists to say; it is also, on the page, invisible beside the same page's black text.

**And of the gap, nine tenths is rasterisation.** Interpretation — which is where a lattice paint
would save — goes from 43 ms to 378 ms and from 103 ms to 601 ms: 0.33 s and 0.50 s of a 1.75 s and
an 8.66 s gap, 19% and 6%. The rest is `render-cpu` drawing the marks, and a paint that hands the
backend a cell and a lattice to replicate *as geometry* still draws every one of them.

## The decision

**The display list does not gain a lattice paint.** Three reasons, in the order they bind.

1. **The version that fits this architecture buys almost nothing.** A `Command` carrying a cell
   and its lattice, replicated by each backend as geometry, removes the copies from the list and
   the copying from `interpret`. On the measurements above that is the peak — 0.42 → small, 0.93 →
   small — and 6% to 19% of the wall clock, leaving `PDFIUM-1497-2.pdf` at about ten seconds. Ten
   seconds at a page turn is inadmissible for the same reason eleven was, so the paint would not
   let either page off its budget: the budget would still have to cut, in a new unit, and the item
   would be where it is with a `Command` variant more.

2. **The version that would buy the rest costs correctness this tree has already paid for.** To
   get below about two microseconds a site a backend has to rasterise the cell once at device
   resolution and blit it. §8.7.3.1's `/TilingType` licenses the distortion that needs — value 1
   permits distorting the cell by up to one device pixel to keep spacing constant, value 3 permits
   "additional distortion … to enable a more efficient" implementation — `doc/md/`'s table cell
   truncates mid-sentence there, which is the conversion's doing rather than the standard's — and
   value 2 permits the *spacing* to vary by a device pixel while the cell does not — so the standard is not the obstacle. Two other things
   are. A blitted tile carries anti-aliased edges, and adjacent tiles' edge fractions then
   composite as `1 − (1−a)(1−b)`, which is precisely the 13% loss `pdf_render::repeat` exists to
   remove and which §11.6.2 forbids outright once §11.6.7 has made the whole tiling one object's
   paint: "Portions of an object shall not be composited with one another". And a tile raster has a
   resolution, so putting one in the display list bakes a flattening resolution into a structure
   that has none — `doc/todo/11` reached the same conclusion about the same construction from the
   other end, refusing a boolean intersection of path against box for that reason.

3. **Two of the three backends would have to refuse it by name.** `render-cpu` is the oracle and
   would have to draw the paint identically, which for a geometry replication it can. `quorra` and
   `vello` have no notion of a pattern and are other repositories: adding the variant means either
   an upstream ask in each (`doc/QUORRA_FEEDBACK.md`'s route) or a translation that expands the
   lattice into their scenes, which puts the display list's expansion back one layer down —
   quorra prices a scene at about 96 bytes a command against a 268 435 456-byte frame budget, so
   `PDFIUM-1497-2.pdf` whole is a refused frame there whichever way the paint is translated. Until
   an upstream lands, the cross-backend comparison would rest on two names in
   `REFUSED_BEFORE_THE_SCENE`, which is exactly the state the two witnesses are in today by way of
   a reported `MAX_TILE_COPIES` — a worse trade, because a refusal by name inside a backend is
   invisible to the interpreter's own report.

**So the item is closed by argument, not deferred.** It is not "we would like this and cannot
afford it": on the two documents that were its whole justification, the change would leave one page
byte-identical and lift the other by a mean of 1.087 levels, at a page turn ten times longer.

## What was done instead, so that the close is checkable

The figures above are a claim about this tree, and a claim in a document decays. Both witnesses now
have rows in `doc/checks/fixed-documents.toml`, which the merge round runs: each pins the page's
ink and that the report is still `MAX_TILE_COPIES`. `2760154.pdf`'s band discriminates — its whole
tiling is worth 1.087 levels against a band of ±1.0, so a paint that drew the wash whole would fail
the row and a round that added one would move the number deliberately. `PDFIUM-1497-2.pdf`'s cannot
and says so in its `why`: the ink is identical whole or cut, so what its row pins is that the page
draws and that the refusal is the per-tiling one rather than `MAX_OPERATIONS`, which is what starved
its frame and title block when the copies were charged to the page's budget alone (ADR 0810).

## What would reopen it

A document whose tiling the budget cuts *visibly* — a page where the sites withheld are worth more
than a wash. Neither witness is one, and neither is any of the 48 the crawl found: ADR 0810 admits
every one of them whole except these two. The instrument that would find such a page is the one
`doc/todo/00` step 7 already runs — our ink minus the lightest reference's over every ambiguous
page — and a tiling that matters would show there rather than in a count of sites. Until one does,
a bigger number and a new paint are the same arbitrary line.
