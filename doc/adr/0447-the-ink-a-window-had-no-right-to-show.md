# 0447 — The ink a window had no right to show

Status: accepted.
Session: 612. Supersedes nothing; completes ADR 0446's open item.

## The requirement, and how it came to be met by an accident

ISO 32000-2 §14.11.2.1, of the five page boundaries, and it is a `shall`:

> The crop box defines the region to which the contents of the page shall be clipped (cropped)
> when displayed or printed. Unlike the other boxes, the crop box has no defined meaning in
> terms of physical page geometry or intended use; it merely imposes clipping on the page
> contents.

§12.2 says which of the five a screen uses. Table 147's `/ViewClip` is "the name of the page
boundary to which the contents of a page shall be clipped when viewing the document on the
screen", default `CropBox`, and `/ViewArea` is the separate question of which boundary is
*displayed*. So the region a reader's screen clips to is `Page::clip_box`, which is the crop box
in every document that states no preference and in all 974 of the corpus.

`pdf_model::interpret` deliberately keeps the marks a content stream made outside that box — a
display list is what the file says, and dropping them there would put the decision in the one
place no host could revisit. Nothing put the clip back.

**It went unseen for the whole life of this tree because the instrument met the requirement
instead of the code.** Every gate rasterises a *page-sized* target: `TargetSpec::for_page` builds
a raster of the boundary's own extent, so the raster's edge did the cutting and a page that drew
a metre beyond its crop box looked exactly like one that did not. A window is larger than its
page. `doc/traps/instruments-and-reports.md` is about exactly this shape, and this is its
sharpest instance so far — a `shall` satisfied by the shape of the measurement.

ADR 0446 made it visible rather than causing it: while the ground beside a page was page white
and one page filled the window, ink outside the boundary was invisible on both counts.

## The population, before anything was written

`crates/pdf-model/examples/crop_box_census` counts three nested things, because
`doc/traps/instruments-and-reports.md` trap 11 says to derive the condition from the clause and
print what it matched:

1. a `/CropBox` smaller than the `/MediaBox` — structural and free;
2. a command whose bounds leave the boundary, ignoring its own clip — an over-approximation, so
   a *candidate*;
3. ink actually outside it, by rasterising the candidates on a target three page-widths across
   and counting the pixels the page marked beyond its own boundary.

Over the pdf.js corpus, `doc/corpora` and the whole SafeDocs crawl — **66 887 first pages
interpreted of 67 193 files**:

| | pdf.js (958) | `doc/corpora` (270) | crawl (65 659) | total |
|---|---|---|---|---|
| crop box smaller than the medium | 52 | 4 | 1065 | **1121** |
| a command's bounds leave the boundary | 106 | 32 | 17 189 | 17 327 |
| **ink actually outside it** | **31** | **10** | **3649** | **3690** |
| …of those, cropping smaller than the medium | 7 | 1 | 796 | 804 |
| …of those, stating no smaller crop box at all | 24 | 9 | 2853 | 2886 |

**The two numbers are about different documents and that is the finding.** Seventy-eight percent
of the affected documents state no smaller crop box: they crop to the medium and draw
beyond it anyway. A structural condition — the obvious one, and the one the round was framed
around — would have named 1121 documents of which only 804 mark anything, and missed 2886 that
do. 202 of the 1121 place every command inside their boundary and the clip costs them nothing.

1729 crawled documents put such a command more than one page-width away, which the census counts
apart rather than dropping: a window scrolled or magnified can be anywhere, but "beside the page"
and "somewhere else entirely" are different pictures.

## Where the clip goes, and why not in the two obvious places

**Not as a `Clip` the interpreter emits.** A page-covering clipping path would be correct and
would cost every page a page-sized coverage mask in `render-cpu` plus a masked composite per
command — on the ninety-four percent of documents that never mark outside their own boundary.
Principle 2 is not a licence to skip a `shall`, but it does decide between two constructions that
state the same one.

**Not inside `impose_within` either**, which is what ADR 0446 refused in advance: a composite that
also erased ink would be a second, silent statement of a rule that belongs in one place.

So the boundary is stated once, as data, and applied once, per target:

- **`DisplayList::content_clip`** — a `Rect` in the list's own space, set by `interpret` from
  §12.2's `/ViewClip` boundary through the page's base transform. `None` means *this list is not
  a page*, which is what a host's chrome is: a sidebar and a find bar are display lists too, and
  §14.11.2.1 says nothing about them.
- **`pdf_render::crop_area`** — that region in a target's pixels, or `None` where no whole pixel
  of the target lies beyond it.
- **`pdf_render::crop_to_page`** — the pass, run by every rasteriser immediately before
  `impose_within`. The page's own ink first, the colours under it second.

`pdf-render` rather than a backend, for `doc/traps/pixels-and-rasterisers.md` trap 2's reason: a
decision either backend can make alone is a decision neither has made.

**`render-quorra`'s window path is the one exception and it is the same exception the medium
has.** A window frame is drawn straight onto the swapchain and there is no raster afterwards to
cut, so `Encoder::crop_to_page` builds one rectangle clip and every chain in the page's commands
hangs from it. A group's elements meet the rectangle twice, which changes nothing but the
coverage of a boundary pixel of the page's own edge.

**And `Interpreter::view_clip` is gone**, with the `Option<ClipId>` it threaded through three
annotation functions. It built a clip chain only where `/ViewClip` named a narrower box than
`/ViewArea` — which no corpus document does — and the region now covers both cases with one
rectangle. An annotation is not exempt from the boundary and no longer has to be told about it:
it belongs to the list rather than to a mark.

## The boundary is a set of whole pixels, and two other readings were written first

This is the part worth keeping. §10.7.4, of what a clip *is*:

> For clipping, the clipping region consists of the set of pixels that would be included by a
> fill operation.

and of which pixels a fill includes, with the reason attached:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules. The area covered by painted pixels shall always be at
> least as large as the area of the original shape.

The first construction written was the natural one — multiply the boundary pixel by the fraction
of it the box covers, exactly as `impose_within` composites 𝑊. It moved **37 of 957** corpus
first pages. The second was §10.7.4's intersection, `min(mark, box)`, which is what `render-cpu`
takes for a clipping *path* (ADR 0355) and which fixed most of it: **11**. The clause's own set
rule moves **none**.

The eleven were pages whose extent is not a whole number of pixels. `TargetSpec::for_page` rounds
a raster *up* so that it contains its page (ADR 0064), and `red_stamp.pdf`'s crop box is 315.001
units tall — its raster's last row is a thousandth of a pixel of page, and a fraction erased ink
there. The sentence quoted above forbids that outright: attenuating a partly covered pixel leaves
the painted area *smaller* than the shape.

**The intersection reading is still right where it came from** and the difference is worth
stating: a clipping path in `render-cpu` knows the mark's coverage and the clip's separately, so
it can take the smaller. A page boundary applied to a finished raster has only one alpha, which is
§11.3's shape and opacity conflated, and cannot.

**The oracle newly contradicted `red_stamp.pdf` under the fractional reading and does not under
this one.** That is corroboration and not the ground, and the order matters: the clause was read
first and the reference agreement noticed afterwards. Principle 5 forbids the other direction.

**This also settles §8.5 by consequence.** "At the beginning of each page, the clipping path shall
be initialised to the size of the MediaBox" is a clip this renderer still never builds — because
it builds a narrower one, which `build_page` has already intersected with the media box. A clip
contained in the media box satisfies a clip to it.

## What moved, measured rather than argued

- **`examples/raster_digest`: 957 corpus first pages, zero lines of difference.** Calibrated both
  ways in the same sitting: shrinking the boundary by five units changes **128** of them, so the
  instrument can fail; restoring it returns to zero. `doc/traps/instruments-and-reports.md` trap
  10b obeyed — `touch` on each changed crate's `src/lib.rs` before either arm.
- **`examples/display_list_digest`: every one of the 958 lists has the same command count and a
  longer `Debug`.** That is the new field and nothing else; the interpreter emits exactly the
  commands it emitted before.
- Every gate in `doc/todo/02` §2 green, including the oracle and the quorra cross-backend gate.

## What was seen on the screen

Xvfb :78 at 900×1100, `doc/pdf.js/test/pdfs/issue1350.pdf` — three pages, and page one draws a
whole second voucher above its crop box.

- `pdf-viewer` on quorra over lavapipe, `OneColumn`, three notches out: **before**, the voucher —
  logo, barcode, purchase details, a ruled frame — sits on the grey ground above the page, exactly
  where the previous page of a column would be. **After**, the ground is clean and the page's
  rectangle holds every mark there is.
- The same in `SinglePage`, reached with six `l`.
- `pdf-viewer-gtk` and `pdf-viewer-qt`, which draw the processor's raster: clean.

The before picture was taken from the same tree with `crop_area` forced to `None`, so the two
differ in this decision and nothing else.

## What is not decided here

The census's `beyond_neighbourhood` count — a mark more than one page-width outside the boundary —
is printed and not acted on. It cannot be: the clip is the same for a mark an inch out and a mark
a mile out, and the distinction exists only to say what the confirming raster could see.

No user interface. §14.11.2.2 lets an interactive processor "display guidelines on the screen for
the various page boundaries" and Table 396's `/BoxColorInfo` says what colour they should be; that
is a feature, it is a *may*, and it is deliberately not started here.
