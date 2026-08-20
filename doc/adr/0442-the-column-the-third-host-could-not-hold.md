# 0442 — The column the third host could not hold

Status: accepted.
Session: the six-hundred-and-seventh.
Subject: ISO 32000-2 Table 29's `/PageLayout` on the tier-2 host; the route chosen between the two
ADR 0441 named; what a reprojection means when the frame is several pages; and the §11.4.7 reading
that looking at the screen produced.

## What was owed

ADR 0441 built Table 29's six arrangements in `viewer_core::layout`, added `Command::Layout`, gave
`Answer::Frame` one entry per page, and landed all six in `viewer-gtk` and `viewer-qt`. It did
**not** land them in `viewer-ui`, and said so out loud rather than drawing one page of a column: a
tier-2 surface drew exactly one `Arc<DisplayList>` per frame and `crate::stale`'s reprojection was
keyed on that list's identity. The host asked the viewer *back* for `SinglePage`, through the
boundary, in one message.

That is honest and it was a violation of the project owner's decision of 2026-08-20 — **all three
hosts stay level** — which `doc/todo/30` records with its own reason: this tree's proudest claim
about the boundary is that six consumers have never asked for a new message, and that claim is
evidence only while the consumers are actually made to carry what is added.

## The two routes, and why the second one

ADR 0441 named two and chose neither: **a display-list merge in `pdf-render`**, remapping clip and
soft-mask identifiers through nested groups so that several pages become one list; or **a second
path in `surface.rs`**, the host drawing several lists per frame. The second is what this round
built, and the argument is four things rather than a preference.

**A merged display list is not expressible for two pages that state different page groups.**
`DisplayList` carries §11.4.7's blending colour space and, with it, a whole companion list holding
the black component — `set_blending`, `blending()`, `black()` — and there is one of each per list
because §11.4.7 makes the space a property of *the page*: "[a]ll page-level compositing shall be
done in the default blending colour space of the page". Two pages in one column may state two
different ones, and one page may state one where its neighbour states none. A merge would have to
refuse the pair or flatten it, and neither is a picture the standard describes. This is a
specification obstruction rather than an engineering one, which is the strongest kind of argument
this project has.

**`Command::Group` carries no transform.** There is no way to place a sub-list by composing an
affine onto it: a page's commands would have to be *rewritten*, transform by transform, along with
every clip's transform and every soft mask's, once per magnification. On the 58 009-command page
that raised `doc/todo/44` that is a deep copy of the page at every zoom step.

**A merged list is a new allocation, and the address is the identity everything downstream is keyed
on.** `render-quorra`'s `FrameSlot` reuses a retained encode by `Arc::as_ptr` (ADR 0351), and
`crate::cache` pins its resources the same way. A list rebuilt whenever the visible set changes
tells both of them the page has been replaced, on every scroll across a row boundary — which is the
233.8 ms per frame ADR 0351 removed, put back for a reason nobody would be able to see.

**And trap 2 does not point at the merge.** The rule is that *a device decision either backend can
make alone is a decision neither has made* — a zero-width stroke, a degenerate contour, a knockout.
Where the pages of an arrangement go is not a decision a backend makes: `viewer_core::layout` has
already made it and states it as a `TargetSpec` per page, so both backends execute one arrangement
and neither chooses one. A merge would have to be *given* those placements anyway, and would then
hold a second statement of the arrangement in `pdf-render` beside the first in `viewer-core` — which
is the shape trap 2 exists to prevent rather than an instance of obeying it.

**The route taken is the one the frame already had.** `PresentFrame::overlays` has been a slice of
placed display lists since the window frame existed, and `build` walks it with one `Encoder` apiece.
`PresentFrame::page` becomes `PresentFrame::pages`, a slice of `(&Arc<DisplayList>, TargetSpec)`, and
`build`'s single `if let` becomes the same loop the overlays already use. `SceneKey` carries one
placement per page and `Retained` one `Arc` per page, so a scroll that brings a further row onto the
screen rebuilds and a still column replays.

## It needed no message, and that is the part worth keeping

A tier-2 host hands back no pixels, so `Query::Frame` answers `Answer::None` for it and cannot say
which pages are on the screen. The obvious move was a new query. It is not needed:
**`Query::PageGeometry` answers `Answer::None` for a page the arrangement does not show** — its own
documentation has said "a page that is not the one showing has no place on the screen" since long
before Table 29 was obeyed — so a host that keeps one `RenderRequest` per page and asks that
question per frame learns exactly which of them have scrolled off. `App::arrangement` is those six
lines; the requests kept are the pages placed.

So the third host cost the boundary nothing, which is what `doc/ui-boundary.md`'s claim is worth
only when a host is actually made to carry the feature.

## What a reprojection means when the frame is several pages

`crate::stale` kept a page and a placement; it keeps a **list** of pages and placements now, and the
question it answers changed shape with it. The window presents its pages as one texture under one
placement, so a stand-in is defensible only where **one affine is true of every page in the
picture**. `one_placement` composes each page's own `settled⁻¹ ∘ asked`, matched by the `Arc`'s
address, and answers only where they all agree. Three outcomes, and each is a different fact:

- **They agree.** A scroll moves every page of a column by the same distance, so a column reprojects
  exactly as a single page always did. This is the gesture a continuous arrangement is *for*.
- **No page is in both.** A page turn, `Refusal::AnotherPage`, unchanged.
- **They disagree**, which is new: `Refusal::Rearranged`. A **zoom** in a column produces it, and
  the cause is `viewer_core::layout`'s own documented choice — the gap between rows is stated in
  logical pixels and does not scale with the magnification while the pages either side of it do, so
  a placement read off the first page would move the second by `GAP × (1 − k)` to somewhere it is
  not.

**Exact rather than tolerant, deliberately.** A tolerance here would be a constant nobody measured a
purpose for, which is the mistake `doc/todo/37` already records twice at two scales. What would
remove the refusal rather than tune it is one textured quad per page in the presenter, and nobody
has asked for it: a zoom in a column shows the previous frame unmoved for one render, which is what
every view change did before ADR 0378. Under `SinglePage` — Table 29's default and what every
document that states nothing opens in — every structure above holds one entry and nothing changed at
all.

## What looking at the screen found, and what was deliberately not done about it

`CLAUDE.md`'s trap 1: every page a change makes drawable is a page nobody has looked at. The column
draws, all six arrangements draw, the wheel crosses a page boundary and the title bar follows it —
**and the 8-pixel gap between neighbouring pages is invisible**, because it is white and so are the
pages.

The reading behind that is worth more than the picture. §11.4.7:

> Ordinarily, the page shall be imposed directly on an output medium, such as paper or a display
> screen. The page group shall be treated as an isolated group, whose results shall then be
> composited with a backdrop colour appropriate for the medium. The backdrop is nominally white (in
> a colour space chosen by the PDF processor), although varying according to the actual properties
> of the medium.

and Table 141 names that colour 𝑊, "[i]nitial colour of the page (nominally white but may vary
depending on the properties of the medium or the needs of the application)". So **𝑊 is a property of
the page**, and what a window shows where there is *no* page is not §11.4.7's subject at all — the
standard says nothing about it, because there is no page there.

This tree has one colour for both. `render-quorra`'s `build` paints the medium over the whole target
and then draws the pages onto it; `render-cpu`'s `impose_on_medium` imposes it over the whole
raster. That was invisible for four hundred sessions because one page filled or was centred in the
window, and it is visible the instant a column puts white paper beside white surround. The two
native hosts never had it: their surround is the toolkit's own window background, drawn by the
toolkit.

**The colour was left where it is.** Splitting 𝑊 from the surround is a change to both backends —
the page's own rectangle painted with 𝑊 under each page, the window's background under everything —
and it touches the correctness oracle. Doing it at the end of a round that had already changed the
frame's shape is how a plausible picture gets shipped, which is the thing principle 1 forbids. It is
named in `doc/todo/30` with the clause under it, and it is a round of its own.

## What it cost

`viewer_host::arrangement::next_layout` is new and is where the *third* copy of that function would
have gone: `viewer-gtk` and `viewer-qt` each wrote it in the six-hundred-and-sixth, one of them with
a comment saying the other had it "deliberately". Which key cycles is a toolkit's; what the next
arrangement is is not, which is that crate's whole test for what belongs in it.

`viewer_ui::software::compose_pages` is the processor's half — `--cpu`, and the window whose device
would not come up — and it is the same construction as `compose`: the first page onto the medium,
every page after it onto transparency and composited, at most `viewer_core::layout::MOST` of them on
the path that is already the slow one. `SoftwareError::Page` is named apart from
`SoftwareError::Overlay` although both are one rasteriser's refusal, because a page that will not
draw is a fact about the document and an overlay that will not draw is a defect in this host's own
chrome.

`App::requests` replaced `App::request`, and `App::unacknowledged` replaced `App::acknowledged`: the
core holds one outstanding request per page on the screen, and a page never answered for is a page
it goes on believing is not yet showing.

Time to first present, three runs apiece under `Xvfb`, both documents in the `OneColumn` each of
them states:

| | pages | `viewer-ui` | `viewer-gtk` |
|---|---|---|---|
| `doc/PDF20_AN001-BPC.pdf` | 5 | 120.5, 120.0, 121.1 ms | 123.6, 119.9, 109.5 ms |
| `doc/ISO_32000-2_sponsored_EC3.pdf` | 1023 | 120.8, 135.9, 134.7 ms | 163.7, 160.2, 163.2 ms |

A document two hundred times longer opens in about the same time on the tier-2 host as well, which
is `CLAUDE.md`'s startup rule holding on the path that now interprets several pages rather than one:
`layout::place` measures a row only when it is about to place it and `layout::MOST` bounds the walk,
so what the arrangement costs is bounded by the window rather than by the document.
