# 2316 — A window raster's declines are handed back, a worker's pixel page is named, and a stand-in says none

Status: implemented. Session 1741, 2026-10-24, the host slot of batch one hundred and thirteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §10.7.4 and `CLAUDE.md`'s first principle; the row stays `departed`. Context: ADRs 2060,
2280, 2290 (its section 4 is the list this closes); `doc/ui-boundary.md`'s freeze.

## 1. A C caller's window raster

ADR 2290 made `quorra_raster` the pixels and the draw's declines, and `quorra_render_ready_raster`
tells the viewer of them with the page. A caller under `QUORRA_OVERSIZE_LISTED` never hands a
window back — it shows it and frees it — so what a window's draw declined stopped in the handle.
**`quorra_raster_declined(viewer, request, raster, events)`** is that half: it consumes neither
handle, sends `Command::Declined` for the request's page with the raster's declines, and answers
the core's `QUORRA_EVENT_REPORTED`, once a condition for the page's reading, or no event where the
draw declined nothing. It is the toolkit windows' `declined_in` for a window, through the ABI; no
new message crosses the boundary, since `Command::Declined` already passed the freeze's test.
`Session::window_declined` is the Rust half, and its test draws `inks.pdf` at sixteen times over
the 19 743-piece mark: one sentence for the first draw, none for the second or for a window clear
of the mark. The C driver calls it on its page past the bound — a missing raster refused, the
window's events counted — and the .NET declaration is added beside its siblings. The entry-point
counts move by one (`unsafe_position.rs`, `header_and_library_agree.rs`).

## 2. A page the confined worker drew as pixels

`quorra-confined` named only the pages its own thread drew from crossed marks; a page its worker
drew as pixels — a scan, whose samples outweigh its pixels, a page whose producer the wire defers,
a page past the bound whose marks cannot cross — was said on the terminal by the worker's viewer
and nowhere in the title, because what crossed was an `Event::Reported`, the same event a
signature or a refusal is. Recognising a decline by its words would make the title depend on a
sentence's spelling. Instead the wire's
frame carries one more byte per page after its origin: **`Framed::declined`**, whether a draw of
the page in the worker declined a mark. `protocol::Marks` keeps the pages a draw there declined
(`declined_here`, from the whole-page draw and from a window's) and forgets one with the page when
it leaves the arrangement, as it forgets everything it holds; a page that crossed as marks writes
`false`, since the host draws it. The window adds a page whose frame says so to the set its title
already names. The drive's `89-union-declined` gives the confined window its folding strokes over
a page-wide image of twelve million samples, so the worker draws the page, and no longer accepts a
silent title: this tree's title names page 1, HEAD's binary's does not.

## 3. Stand-ins say nothing, deliberately

`viewer_device`'s retained proxies and `quorra`'s composed whole page under the base are pictures
nobody asked for, drawn small while the window is idle so a zoom or a scroll has something to move
until the view's own frame lands (`stale.rs`'s rule 1: never the last word). Their draws go through
`compose_pages`, which collects no declines, and that stays: **what the reader is shown is the
view's own frame, and it says its declines at the size the reader sees.** Saying a stand-in's
would be worse than redundant — the core says each condition once while the page's reading stands,
so a proxy's sentence told first would spend that one sentence on a picture at a size nobody
looked at, and the frame that matters would then be silent about its own. Both sites' comments
say so. This is a reading of the first principle, which forbids a silence about what is shown, and
not an exception to it.

## 4. Cost

One entry point, which changes no existing signature and so moves no `QUORRA_ABI_VERSION`; one
byte per page per frame on the confined wire, and a `BTreeSet` bounded by the arrangement in the
worker. No pixel moves.
