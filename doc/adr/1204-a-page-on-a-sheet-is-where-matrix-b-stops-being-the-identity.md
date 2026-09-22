# 1204 — A page on a sheet is where matrix B stops being the identity

Session 1183. Status: **accepted**. Carries out the expired premise `doc/todo/65` recorded against
[ADR 1057](1057-a-cloud-is-a-shall-behind-a-silence-and-a-watermark-has-no-window.md) section 4, and
discharges the debt [ADR 1179](1179-table-167-states-two-answers-and-the-output-decides-which.md)
section 4 left at `annotation::target_media`'s own doc comment.

## 1. The debt was written at the function, and this is it paid

ADR 1179 §4 read §12.5.6.22's B sentence and found that for every caller this program then had, B
scaled nothing and rotated nothing — a page went onto the sheet at its own size — so the whole of
the cancellation was the translation between two origins, which a media *rectangle in the page's
own space* already carries. It then named the condition under which that stops holding:

> The two bullets after the EXAMPLE are the cases where B stops being the identity — page tiling
> and n-up printing, each of which the clause gives its own placement rule — and neither is a thing
> this program offers.

A scale mode and an n-up composition are things this program offers now, so the rest of the
sentence is owed and is here.

## 2. What the two bullets actually require

§12.5.6.22, after the EXAMPLE:

> When page tiling is selected in a PDF processor (that is, a single PDF page is printed on
> multiple pages), watermark annotations shall be printed at the specified size and position on
> each page to ensure that the content of the watermark annotation is present and legible on each
> printed page.

> When n -up printing is selected (that is, multiple PDF pages are printed on a single page), the
> annotations shall be printed at the specified size and shall be positioned as if the dimensions
> of the printed page were limited to a single portion of the page. This ensures that any content
> of the watermark annotation does not overlap content from other pages, thus rendering it
> illegible.

Two requirements, and the second bullet states both of them:

- **At the specified size.** The size is measured on the *media*, so a page the printer scales must
  not scale the mark with it. That is what "cancels out B" means once B has a scale in it, and it is
  a division: the transformed annotation rectangle is divided by the factor the page's own placement
  will multiply it by.
- **Against a single portion of the page.** The media for an n-up page is its *cell*, not the sheet
  — the clause says "as if the dimensions of the printed page were limited to" it — so Table 194's
  percentages are measured against the cell's rectangle.

## 3. The shape, and why the cell's position on the paper does not enter it

`pdf_model::view::TargetMedia` is the pair a caller states: the media rectangle **in the page's own
default user space**, and `page_scale`, what B scales the page by. The first carries the
translation by being stated in the page's space; the second is what `fixed_print` divides by.
`ViewState::set_paper` takes it and `viewer_core::Sheet` carries it across the boundary, the confined
wire and the C ABI — where it is `quorra_print_placed` rather than a fifth argument on
`quorra_print`, because a changed signature is a call an old caller has already compiled and no
diagnostic would catch.

**One `Sheet` serves a whole n-up job**, and that is arithmetic rather than a simplification: the
media in page space is the cell's corner measured *from the page's origin*, and every cell of a
uniform grid places its page the same way inside itself, so the cell's absolute position on the
paper cancels. What differs per cell is only where a host paints, and that is
`viewer_host::printing::cell` — the same placement said the other way round, in points on the paper.

`viewer_host::printing::placed` composes the pair from a paper size, a page size, RFC 0004 §6's
scale mode and a pages-per-sheet count. The raster scale is multiplied by the page's own, so a page
drawn at half size costs a quarter of the pixels and still lands on the paper at the printer's
resolution.

## 4. Two conventions, named as conventions

The clause states what happens to a watermark when n-up is selected. It does not state what n-up
*is*, and neither does any other clause, so two choices are written down rather than implied:

- **The grid.** Two pages split the sheet's longer axis; four split both. This keeps each cell's
  proportions as near the sheet's as the count allows.
- **The page in its cell.** Centred, with a page larger than its cell inset negatively — which is
  the honest arithmetic for *actual size* on paper too small for the page, rather than a clamp that
  would move the mark.

RFC 0004 §6's three scale modes are the field's and are named there: actual size, shrink to fit
(the default, and the one that leaves a page already fitting the paper alone, so B stays the
identity for the common job), and fit to page.

## 5. What tiling gets, and why it is not less than the clause asks

The first bullet is about one page on several sheets, and no host composes one. The *model*
expresses it — a tile is a media rectangle and a factor like any other, and a host that composed
tiling would state one `Sheet` per tile — and nothing in this tree selects it. The clause's
requirement is conditioned on tiling being "selected in a PDF processor", so it is met wherever this
program can reach the condition, which is the same shape §12.5.6.22's screen sentence already had.

## 6. What was exercised

- `pdf-model/tests/print_intent.rs`: the same media with `page_scale` 1.0 and 2.0, which differ
  **only** in the mark's size and not in its position — a reader ignoring the factor passes the
  first row and fails the second, and one applying it to the translation as well moves the mark off
  25% of the media's width. A factor that is not a finite positive number leaves the page's own
  scale standing.
- `viewer-host/src/printing.rs`: a two-up A4 sheet, where the cells tile the longer axis, are one
  size, do not overlap, and agree with the media the core measures percentages against; and the
  three scale modes against a page larger than the paper and one smaller.
- The fixtures are hand-built and say so (trap 8). `/FixedPrint` is essentially absent from the
  corpora — one document states one — so a corpus cannot hold the pair of files that differ in one
  placement and in nothing else, which is the only shape that discriminates here.
