# ADR 0116 — The host boundary is a vocabulary, not a trait

Status: accepted, 2026-08-01.

## What this decides

`viewer-core` is real. It is `Command` in, `Event` out, `Query` → `Answer` beside them, and
`Viewer` is the state machine between: the open-document set, which page is showing, how large
it is drawn, where it is scrolled, and what has to be rasterised next. The handover's §0
specified the shape; this is the first code under it, with a headless consumer that drives it
and asserts what comes back.

Five owed items were one missing interface — a password prompt, a layer panel, presentation
mode, an editable field, an accessibility tree — and none of them is a clause question. This
does not implement any of the five. It implements the thing all five were waiting for, and two
of them are now one host away rather than one architecture away.

## The decisions inside it, each with its reason

**Messages rather than a callback trait.** A trait would have been fewer lines. It would also
have been untestable without a display, unable to cross an FFI boundary without a shim per
method, unable to cross a *process* boundary at all, and the wrong shape for the confined
renderer principle 3 wants. `Command`/`Event` cost one `Vec` per call and survive all four.

**Two channels, not one.** `handle` mutates and returns events; `query` takes `&self` and
returns an answer. A selection cannot wait for a render round trip: hit-testing a point and
asking where the page sits on the screen happen between two frames, and a host that had to post
a command and wait would feel every one. The `&self` is the promise, stated in the type system
rather than in a comment.

**This crate interprets; the host rasterises.** `Event::NeedsRender` carries a finished
`Arc<DisplayList>` and a `TargetSpec`. Three reasons in the order they decided it: interpretation
is a pure function of an immutable document, so duplicating it per host would be a second reading
of the same file; a display list is resolution-independent, so **zoom and scroll re-rasterise
without re-interpreting**, which is what `TargetSpec`'s own doc comment says it exists for; and
it leaves the process boundary in one piece, because everything that touches PDF bytes stays on
this side of it. The cost is that interpretation runs on whichever thread calls `handle`, and the
answer to that is that a host wanting it elsewhere moves the whole `Viewer` — which a message
vocabulary allows and a callback trait would not.

`zooming_rasterises_again_without_interpreting_again` proves the middle reason by pointer
equality of the shared list, and fails if a zoom re-interprets.

**A render is answered with a token, and a stale token is dropped.** A page turned while a render
is in flight must not be overwritten by the frame the previous page produced. Confirmed by
deleting the check: `a_render_answered_after_the_page_turned_is_dropped` fails.

**Three outcomes, not a `Result`.** `Rendered::Raster` is tier 1 — the viewer holds the pixels and
hands them back on `Damage`. `Rendered::Presented` is tier 2 — the host drew onto its own surface,
and there is nothing here to hold. `Rendered::Failed` names why. A host that draws its own pixels
has not failed, and a two-variant `Result` would have had to call it one or the other.

## The one measurement

**A page fitted to a window came out one pixel taller than the window, about half the time.**
`TargetSpec::for_page` rounds a raster *up* so that it contains the page — its own comment
explains why — and the nearest `f32` to the exact ratio `viewport / extent` is above the exact
value as often as below. The product then crosses the next integer and the raster is 1001 rows in
a 1000-row viewport: a page fitted to the window, with a scrollbar down the side.

The fix is not an epsilon. `fitted` steps to the next smaller representable `f32` until the
rounding lands, which is exact, needs no constant anybody chose, and costs a comparison once per
render. `FIT_STEPS` is 4 as a bound rather than an expectation — one step has always been enough,
and the loop's condition is what decides.

Confirmed by setting the bound to zero: `the_page_geometry_maps_the_page_onto_the_screen` fails
with 1001 against 1000.

This is the class of defect no gate in this project could have found. It is not a clause, the
oracle never renders into a window, and the corpus has no viewport. It was found by asserting a
number in a test that had no reason to be interesting.

## What is not here

The winit binary still has its own copy of everything — links, actions, imports, the document's
own notes — and is not yet a consumer. That is deliberate: moving it in the same commit would
have meant either two copies of the open-document notes for a session or a change too large to
review as one thing. It is the next session's whole content.

Nothing edits, selects or prompts. The vocabulary has room for all three and no code behind any
of them; `CLAUDE.md`'s exclusion list was amended for the first two in the session before this.

## Consequences

Tests 895 → 907. The twelve are the headless consumer, which is the gate this project would want
anyway: selection semantics, undo, edit application and hit-testing are all things to test
without a display, and the harness for them now exists. Citations 2833 → 2874, quotations 305 →
307. The four gates are unmoved, which is the expected result of a session that added a crate and
changed no rendering path: corpus 91 incomplete, oracle 65 contradicted, text 42 below the floor.

`pdf_model::content::rotated_size` became `displayed_size` and public. A viewer needs a page's
extent *before* there is a display list to read it from, because fitting the page to the window is
what decides the scale to interpret it at; asking the other way round would interpret every page
twice.

## The lesson

**A boundary that is being discovered rather than invented shows it by fitting.** Five of the
`Event` variants are `pdf_model::view::Request` variants that already existed, arrived at one
clause at a time over sixty sessions by asking "what does performing this action need from a
viewer?". None of them was designed as part of an interface. That they are the interface is the
strongest evidence available that the line is in the right place.
