# 0446 — The colour the page was not

Status: accepted.
Session: the six-hundred-and-eleventh.
Supersedes nothing; closes the finding ADR 0442 recorded and deliberately left, and amends
`doc/todo/30`'s claim that the two native hosts never had it.

## The defect

Sessions 606 to 610 gave all three hosts Table 29's six page layouts, so a window now routinely
shows several pages at once with a gap between them. **The gap was invisible.** The
six-hundred-and-seventh session found it by looking at the screen, tried a grey medium to see the
gap, watched the *pages* turn grey, and wrote the finding down rather than half-fixing it at the
end of a round that had already changed the frame's shape.

That experiment is the whole diagnosis: **one colour was doing two jobs.** A page paints no white
of its own — §11.4.7 makes the page group isolated, so an unmarked pixel is transparent and the
medium is composited with the *result* — so the single `Color` every backend imposed was serving
both as the page's own colour and as the ground the window shows where there is no page. Change it
and both move.

## What the standard states, and what it does not

**§11.4.7 states the first of the two, and states whose it is.** The page group "shall be treated
as an isolated group, whose results shall then be composited with a backdrop colour appropriate for
the medium. The backdrop is nominally white (in a colour space chosen by the PDF processor)", and
Table 141 names that colour 𝑊: "[i]nitial colour of the page (nominally white but may vary
depending on the properties of the medium or the needs of the application)". A property of **the
page**. The clause's own sentence about an interactive processor stays inside that boundary — "some
interactive PDF processors may choose to provide a different backdrop, such as a checker board or
grid to aid in visualizing the effects of transparency in the artwork" is a different backdrop *for
the page*, so that the page's transparency can be seen through it, not a statement about what
surrounds one.

**Where the page stops is §14.11.2.1's, and it is a `shall`**: "[t]he crop box defines the region to
which the contents of the page shall be clipped (cropped) when displayed or printed", and three
sentences later "the crop box determines how the page's contents shall be positioned on the output
medium". `DisplayList::page_bounds` is that box with its corner at the origin, so mapping it
through a target's transform says where 𝑊 applies in that target's own pixels.

**What lies outside every page is stated nowhere, and that claim was searched rather than
assumed.** `CLAUDE.md`'s rule is explicit that "the specification defines nothing here" is itself a
claim about the specification and that it decays — `DeviceCMYK` survived thirty-two sessions in
that file being wrong. So: §11.3's compositing formulas and §11.4.5's isolated groups name only a
*group's* backdrop; §11.4.7 and Table 141 name the page's; §14.11.2's five boundaries are all
regions *of* a page; and Table 147's twenty-two viewer preferences say what to hide, which boundary
to display, which to clip to, how to scale for printing — and nothing about what a window shows
around the result. `cargo run --release -p spec-errata -- emit doc/*.pdf` reports no erratum
anywhere in clause 11. The standard describes one page imposed on one medium; that a window may
show two at once, with something between them, is outside its subject.

So the surround is **this program's choice**, and it is written down as one.

## The decision

`pdf_render::medium` is a new module and owns the whole subject:

| | is | whose |
|---|---|---|
| `Medium::page` | §11.4.7's 𝑊 | the standard's |
| `Medium::surround` | what a target shows outside every page | this program's |
| `page_area(list, target)` | §14.11.2.1's crop box in the target's pixels | the standard's |
| `SURROUND` | a quarter of full scale, neutral | a documented choice |

**It is in `pdf-render` because `doc/traps/pixels-and-rasterisers.md` trap 2 says so**: *a decision
either backend can make alone is a decision neither has made*. Three rasterisers draw this
program's pages and the boundary has to fall in the same place in all three, so the boundary, the
composite and the colour are stated once. `render-cpu` and `render-gpu` composite it per pixel
after the page is drawn (`impose_within`, which is `impose_on_medium` with the boundary in it);
`render-quorra` draws it as rectangles at the bottom of a scene — the surround over the frame, 𝑊
per placed page at that page's own `page_area`. `crates/viewer-ui/tests/page_and_surround.rs`
compares the two constructions on a two-page arrangement and **each arm was made to fail** by
removing its own half.

`with_background(Color)` became `with_medium(Medium)` on all three backends, so every caller had to
say what it meant rather than inherit a habit — the mechanism `doc/todo/30` calls *a variant
carrying too little*. Twenty call sites, all of them `Color::TRANSPARENT`, all of them now
`Medium::NONE`.

### The boundary is a coverage, not a pixel edge

Each pixel takes the fraction of itself the page covers — exact box coverage of an upright
rectangle, the product of the two axes' overlaps — and 𝑊 is composited at that fraction, over the
surround. Snapping to whole pixels would be cheaper and wrong in the way that matters: two pages
whose gap is under a device pixel would each round outward, the gap would close, and the separation
would vanish at exactly the magnification a reader is most likely to notice it at.

### What is *not* decided here, and is now visible

§14.11.2.1's clip is the interpreter's, and `pdf_model::interpret` deliberately keeps the marks a
stream made outside the crop box. On a page-sized target the raster's own edge cuts them; on a
**window-sized** one it does not, so a mark outside the box draws over the ground beside the page.
That was invisible while the ground was page white and one page filled the window, and it is
visible now on both counts. It is a second decision — where to apply the clip, and in a scene
builder rather than in a per-pixel pass — and it belongs in a round of its own. `doc/todo/30` and
§14.11.2.1's ledger row carry it with the clause; the population is not measured.

**No chrome was invented.** A drop shadow, a page border or a configurable theme is a larger
question and is deliberately not answered: `SURROUND` is one constant with no user interface.

## Not moving a pixel is the claim, and it is measured

The corpus and oracle gates rasterise a **single page**, whose target is the page's own extent, so
`Medium::PAGE_ONLY` asks for the same colour on both sides of a boundary and `Medium::is_uniform`
then takes exactly the pass this clause always had, byte for byte.

`crates/pdf-model/examples/raster_digest.rs` is the instrument that says so rather than argues it —
new, because `display_list_digest` proves the *interpreter* unchanged and nothing proved the
rasteriser. Over `doc/pdf.js/test/pdfs/*.pdf` at `HEAD` and at this revision: **974 documents, 957
first pages rasterised, zero lines of difference.**

**`is_uniform` is a correctness decision and not a shortcut**, and the calibration is what shows
it: with `PAGE_ONLY`'s surround moved off white, **193 of the 957 pages change** — the ones whose
extent is not a whole number of pixels at 72 dpi, where `TargetSpec::for_page` rounds the raster up
past the crop box and a sliver of it is genuinely outside the page. That population is what a
careless separation would have moved on every gate in this tree.

**Cargo will hand a stale binary to whoever runs that comparison.** Adding a *new module file* to
`pdf-render` left the release-profile fingerprint of every crate above it unaware of it, so
`cargo build --release` after editing the file recompiled nothing and the example printed the
previous revision's hashes — twice, before it was noticed, and the first "byte-identical" result
of this session was worthless. `touch` the changed crates' `src/lib.rs` before believing either
arm. The example's own header says so.

## The native hosts were not exempt, and `doc/todo/30` said they were

ADR 0442 recorded that "[t]he two native hosts never had it — their surround is the toolkit's own
window background". **Looked at, on the screen: they had it.** GTK's default background under
Adwaita is within a few levels of paper white, so the gap between two pages of a column was a
hairline nobody would read as a boundary; Qt's was the same. The sentence was a plausible inference
and not an observation, which is trap 1 in the form it takes about *other people's* defaults.

Both hosts take `pdf_render::SURROUND` now — GTK through one `CssProvider` rule on the widget
holding the pages, Qt through the palette of `PageArea`, with the value crossing the `cxx` bridge
from `Host::surround` rather than being restated in C++. The alternative — inherit the platform —
sounds like the native answer and is not one: a toolkit has no notion of *the surface a document is
laid on*, so there is no platform value to inherit, and picking the same one in all three hosts is
`doc/todo/30`'s standing decision that the hosts stay level.

## What was seen on the screen

Xvfb :78 at 900×1100, `doc/PDF20_AN001-BPC.pdf`, all three hosts. `viewer-ui` in `OneColumn` shows
a dark band between pages one and two; `l` to `TwoColumnLeft` and six `-` gives 1|2, 3|4, 5 with
every page's four edges legible against the ground, which is the picture this round exists for and
which was one white field before it. At the magnification where a page fills the window there is no
surround at all, correctly. Sampling a row across a page's left edge gives 64, 64, …, 64, 255,
255 — `SURROUND` is exactly a quarter of full scale and the transition is one pixel wide. `pdf-viewer-gtk`
and `pdf-viewer-qt` show the same band in `OneColumn`.
