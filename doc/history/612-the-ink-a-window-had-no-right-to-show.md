# 612 — The ink a window had no right to show

The seventh round of the block on the owner's decision that **the UI is now work**, and the one
that closes what 611 found and deliberately left: §14.11.2.1's crop is a `shall`, and a window did
not apply it.

Date: 2026-08-20.
ADR: [0447](../adr/0447-the-ink-a-window-had-no-right-to-show.md).

Touched: `crates/pdf-render/src/{medium.rs, display_list.rs, lib.rs}`; `crates/pdf-model/src/content.rs`
and `crates/pdf-model/src/content/annotations.rs`; `crates/render-cpu/src/lib.rs`;
`crates/render-gpu/src/lib.rs`; `crates/render-quorra/src/{scene.rs, present.rs, lib.rs}`; a new
`crates/pdf-model/examples/crop_box_census.rs`; `doc/conformance/ledger.toml` (§14.11.2.1, §12.2,
§10.7.4, §8.5), `doc/traps/pixels-and-rasterisers.md`, `doc/HANDOVER.md`, `doc/todo/30`, the ADR and
this file.

## The population first, and it was not the population the round was framed around

`examples/crop_box_census` is new and counts three nested things rather than one, because trap 11
says to derive the condition from the clause: a crop box smaller than the medium (structural), a
command whose bounds leave the boundary (a candidate, since it ignores the command's own clip), and
ink actually outside it (the candidates rasterised on a target three page-widths across).

Over 67 193 files — the pdf.js corpus, `doc/corpora` and all 65 944 of the SafeDocs crawl — 66 887
first pages interpreted: **1121 state a crop box smaller than their media box, and 3690 actually
mark outside the boundary.** Only 804 documents are in both sets. **2886 of the 3690 state no
smaller crop box at all** and simply draw beyond the medium, and 202 of the 1121 draw nothing out
there and cost nothing. The structural question, which is the one that reads like the right one,
would have named the wrong documents in both directions. Fifteen minutes over the crawl, three
seconds over the corpus.

## Where it went

`DisplayList::content_clip` is the boundary, in the list's own space, set by `interpret` from
§12.2's `/ViewClip` — `Page::clip_box` rather than the crop box by name, because Table 147 lets a
document display one of §14.11.2's boxes and clip its ink to another. `pdf_render::crop_area` maps
it into a target's pixels and answers `None` where no whole pixel lies outside; `crop_to_page` is
the pass, run by all three rasterisers immediately before `impose_within`. `render-quorra`'s window
path is the exception the medium already had — a frame drawn onto a swapchain has no raster
afterwards to cut — so `Encoder::crop_to_page` hangs every clip chain from one rectangle.

A `Clip` the interpreter emits would have said the same thing and cost every page a page-sized
coverage mask in `render-cpu`. `Interpreter::view_clip` and the `Option<ClipId>` it threaded through
three annotation functions are gone: the region covers what the chain did, for every document rather
than for the zero that state the preference.

`None` also means *this list is not a page*, which is what a host's chrome is — a sidebar is a
display list drawn into a window-sized target and §14.11.2.1 says nothing about it.

## The clause decided the arithmetic, and it took three tries

**Two constructions were written before the right one and the corpus rejected both.** Multiplying
the boundary pixel by the box's coverage moved 37 of 957 corpus first pages. §10.7.4's intersection,
`min(mark, box)` — `render-cpu`'s own reading for a clipping path (ADR 0355) — moved 11. The clause's
own rule moves none:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects the
> shape, no matter how small the intersection is. … The area covered by painted pixels shall always
> be at least as large as the area of the original shape.

A clipping region is "the set of pixels that would be included by a fill operation", so a pixel the
boundary touches at all is inside it and keeps its ink whole. The eleven were pages whose extent is
not a whole number of pixels: `red_stamp.pdf`'s crop box is 315.001 units tall, so the last row of
its raster is a thousandth of a pixel of page, and any fraction erased ink there — which the last
sentence quoted forbids outright.

**The oracle newly contradicted `red_stamp.pdf` under the fractional reading and does not under this
one, and the order the two arrived in matters**: the clause was read first and the reference
agreement noticed after. Principle 5 forbids the other direction, and the ADR says so where a later
round can check it.

## Nothing moved, and the instrument was made to fail on purpose

- `examples/raster_digest`, 957 corpus first pages: **zero lines of difference**. Calibrated in the
  same sitting — shrinking the boundary by five units changes **128** of them, restoring it returns
  to zero. Trap 10b obeyed: `touch` on each changed crate's `src/lib.rs` before either arm, and the
  before-arm built in its own worktree with its own target directory so the shared build scripts
  were left alone.
- `examples/display_list_digest`: all 958 lists have the **same command count** and a longer `Debug`.
  That is the new field and nothing else — the interpreter emits exactly the commands it emitted.
- Every gate of `doc/todo/02` §2, run whole.

## `doc/todo/00`'s step 7, run whole because the habit says so

Our ink minus the lightest reference's, over all **786** ambiguous pages of the oracle's own output,
from artefacts already on disk. **19 at or past −1 and 16 of them documents this tree calls
incomplete**; head `issue12418_reduced.pdf` −19.447, `issue4722.pdf` −13.810,
`issue15977_reduced.pdf` −12.927, `bug1050040.pdf` −11.272, `issue5801.pdf` −8.991. On the complete
documents `issue16038.pdf` −5.737, `issue12295.pdf` −2.364, `issue14297.pdf` −1.130, then
`issue7821.pdf` −0.957 and `jpx_smaskindata.pdf` −0.840 — three past −1 and all three diagnosed.
Positive head `recursiveCompositGlyf.pdf` +198.653, `bug1743245.pdf` +23.277, `bug920426.pdf`
+21.073, `issue4260_reduced.pdf` +17.607.

**Every figure reproduces the five-hundred-and-ninety-eighth session's whole run to the
thousandth**, which is what the raster digest predicted and what the habit exists to check rather
than to assume.

## What was seen on the screen

Xvfb :78 at 900×1100, `doc/pdf.js/test/pdfs/issue1350.pdf`, whose first page draws a whole second
voucher above its crop box.

- `pdf-viewer` on quorra over lavapipe, `OneColumn`, three notches out: **before**, the voucher —
  logo, barcode, "Purchased by", a ruled frame — sits on the grey ground above the page, exactly
  where a column's previous page would be. **After**, the ground is clean.
- The same in `SinglePage`, six `l` away.
- `pdf-viewer-gtk` and `pdf-viewer-qt`: clean.

The before picture came from the same tree with `crop_area` forced to `None`, so the two differ in
this decision and in nothing else.

## Two things a later round should know

**Trap 14 is new** and is the general form of what this round fixed: a requirement can be met by the
shape of the instrument rather than by the code, and then no gate can tell. Every gate here
rasterises a page-sized target, which *is* the crop box, so a clause about clipping to the crop box
was satisfied by the raster's own edge for the whole life of the tree.

**Table 30 has no `/CropBox` row.** The round was pointed at "Table 30's `/CropBox` row" and the
entry is Table 31's — Table 30 is the *required* entries in a page tree node. The code cites 31
correctly and always did.
