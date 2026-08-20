# 611 — The colour the page was not

The sixth round of the block on the owner's decision that **the UI is now work**, and the one that
closes what 607 found by looking at the screen and deliberately left: the page's colour and the
window's surround were one value, and they are two different things.

Date: 2026-08-20.
ADR: [0446](../adr/0446-the-colour-the-page-was-not.md).

Touched: `crates/pdf-render/src/{medium.rs (new), backend.rs, lib.rs}`; `crates/render-cpu/src/lib.rs`;
`crates/render-gpu/src/lib.rs`; `crates/render-quorra/src/{lib.rs, present.rs}`;
`crates/viewer-ui/src/software.rs` and a new `crates/viewer-ui/tests/page_and_surround.rs`;
`crates/viewer-gtk/src/host.rs`; `crates/viewer-qt/src/{bridge.rs, host.rs}` and
`crates/viewer-qt/cpp/window.cpp`; a new `crates/pdf-model/examples/raster_digest.rs`; nineteen test
files that named `with_background`; `doc/conformance/ledger.toml` (§11.4.7, §14.11.2.1),
`doc/todo/30`, the ADR and this file.

## The clauses, and the line between them

- **§11.4.7** — "[t]he page group shall be treated as an isolated group, whose results shall then be
  composited with a backdrop colour appropriate for the medium. The backdrop is nominally white (in
  a colour space chosen by the PDF processor)". **Table 141** names it: 𝑊, "[i]nitial colour of the
  page". A property of *the page*.
- **§14.11.2.1** — "[t]he crop box defines the region to which the contents of the page shall be
  clipped (cropped) when displayed or printed", and "the crop box determines how the page's contents
  shall be positioned on the output medium". That is where the page — and therefore 𝑊 — stops.
- **Nothing** states what a window shows where there is no page. Searched rather than assumed,
  because `CLAUDE.md` says that claim decays and has been wrong here before: §11.3's compositing
  formulas and §11.4.5's isolated groups name a *group's* backdrop, §14.11.2's five boundaries are
  all regions of a page, Table 147's twenty-two viewer preferences say what to hide and which
  boundary to clip to and nothing about the result's surroundings, and `spec-errata`'s `emit`
  reports no erratum anywhere in clause 11.

§11.4.7's own permission — "some interactive PDF processors may choose to provide a different
backdrop, such as a checker board or grid" — is a different backdrop **for the page**, so that the
page's transparency can be seen through it. It is not about what surrounds one, and reading it as
such would have been the wrong way to close this.

## The separation

`pdf_render::medium` is a new module and owns the subject: `Medium { page, surround }`,
`page_area(list, target)` mapping §14.11.2.1's box into the target's pixels, `impose_within` as
`impose_on_medium` with that boundary in it, and `SURROUND` — a quarter of full scale, neutral —
written down **as a choice**. In `pdf-render` because trap 2 says a decision either backend can make
alone is a decision neither has made.

`render-cpu` and `render-gpu` composite it per pixel after the page is drawn; `render-quorra` draws
it as rectangles at the bottom of the scene, the surround over the frame and 𝑊 per placed page at
that page's own `page_area`. `with_background(Color)` became `with_medium(Medium)` everywhere, so
twenty call sites had to say what they meant.

**The boundary is a coverage rather than a pixel edge** — exact box coverage, the product of the two
axes' overlaps — because snapping would close a gap narrower than a device pixel, which is the one
magnification where the separation matters most.

## Not moving a pixel, measured rather than argued

`examples/raster_digest` is new: `display_list_digest` one layer down, hashing the *bytes* of every
corpus first page. Nothing in this tree could see a rasteriser change directly before it.

Over `doc/pdf.js/test/pdfs/*.pdf` at `HEAD` and at this revision: **974 documents, 957 first pages,
zero lines of difference.**

Calibrated both ways. Moving 𝑊 off white changes every hash — the instrument can fail. Moving the
*surround* off white on a page-sized target changes **193 of the 957**: the pages whose extent is
not a whole number of pixels at 72 dpi, where `TargetSpec::for_page` rounds the raster up past the
crop box. That is the population a careless separation would have moved on every gate, and it is why
`Medium::is_uniform` is a correctness decision rather than an optimisation.

**And the first two byte-identical results of this session were worthless.** Adding a *new module
file* to `pdf-render` left the release-profile fingerprint of every crate above it unaware of it, so
`cargo build --release` recompiled nothing and the example printed the previous revision's hashes.
`touch` the changed crates' `src/lib.rs` before believing either arm; the example's header and this
paragraph exist because it took three runs to notice.

## The native hosts were not exempt, and 607 said they were

ADR 0442 recorded that "[t]he two native hosts never had it — their surround is the toolkit's own
window background". Looked at, on the screen: **they had it.** GTK's Adwaita background is within a
few levels of paper white and Qt's palette is the same, so the gap between two pages was a hairline
in both. A plausible inference about somebody else's defaults, never observed — trap 1 pointed
outwards.

Both take `pdf_render::SURROUND` now: GTK through one `CssProvider` rule on the widget holding the
pages, Qt through `PageArea`'s palette with the value crossing the `cxx` bridge from
`Host::surround`. Inheriting the platform sounds like the native answer and is not one — a toolkit
has no notion of *the surface a document is laid on*, so there is no platform value to inherit.

## What was seen on the screen

Xvfb :78 at 900×1100, `doc/PDF20_AN001-BPC.pdf`.

- `viewer-ui`, `OneColumn`: a dark band between pages one and two where there was one continuous
  white field.
- `l` to `TwoColumnLeft`, then six `-`: 1|2 over 3|4 over 5, every page's four edges legible against
  the ground. This is the picture the round exists for.
- At the magnification where a page fills the window, no surround at all — correct.
- Sampling a row across a page's left edge: 64, 64, …, 64, 255, 255. `SURROUND` is exactly a quarter
  of full scale and the transition is one pixel wide.
- `pdf-viewer-gtk` and `pdf-viewer-qt`: the same band, in `OneColumn`, where both had a hairline.

## What is owed, named rather than left implicit

**§14.11.2.1's clip is not applied on a window-sized target.** `pdf_model::interpret` deliberately
keeps the marks a stream made outside the crop box; a page-sized raster cuts them at its own edge
and a window-sized one does not, so such a mark draws over the ground beside the page and over a
neighbouring page of a column. Invisible while the ground was page white and one page filled the
window; visible now on both counts. It is not `impose_within`'s to do — a composite that erased ink
would be a second, silent statement of a rule that belongs in one place. `doc/todo/30` carries it
with the clause, and **the population is not measured**: nothing counts how many documents mark
outside their crop box.

No chrome was invented. A drop shadow, a page border and a configurable theme are a larger question
and are deliberately not answered; `SURROUND` is one constant with no user interface.
