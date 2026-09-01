# Session 848 — the annotation pass is a twentieth of the page it forces a re-read of

2026-09-01. ADR 0775. A loop round on `doc/todo/46`, §12.5.3's re-interpretation on the event
thread.

**Finding**: the item offered three shapes and asked for one to be chosen. The measurement chooses
it — `draw_annotations` is **1.6% of page 1001's 3.52 ms and 6.0% of page 10's 538 µs** — so
*re-place rather than re-interpret* removes 94–98% of the work where *interpretation off the event
thread* would relocate 100% of it into an async seam across three hosts. Building the seam is more
than a round; its first stage turned out to be a defect rather than an optimisation, and landed:
`Open::reinterpret` was asking "does this depend on the magnification?" of Table 29's
**arrangement** rather than of the page, so one `NoZoom` annotation on the screen cost the
interpretation of every page beside it. The worst wheel notch on ISO 32000-2, scrolled across a
page boundary, falls from **13.99 ms to 4.98 ms**.

## Files

- `crates/viewer-core/src/open.rs` — `Open::reinterpret` asks per page, and forgets one readback
  rather than the cache.
- `crates/viewer-core/src/readback.rs` — `Readbacks::forget`, beside `clear` with the difference
  between their two reasons written down.
- `crates/viewer-core/src/viewer.rs` — `settle`'s arrangement-wide guard is gone; `reinterpret`
  decides.
- `crates/viewer-core/tests/headless.rs` —
  `a_zoom_re_interprets_the_page_with_the_annotation_and_not_the_page_beside_it` and its two-page
  fixture, whose annotations differ in one bit of Table 167.
- `crates/viewer-core/examples/zoom_cost.rs` — new, the instrument.
- `doc/adr/0775-the-page-beside-the-one-the-clause-is-about.md` — new.
- `doc/todo/46-a-wheel-tick-that-interprets.md`, `doc/todo/README.md` — the shape chosen, the stage
  landed, the remainder priced with its two constructions.
- `doc/conformance/ledger.toml` — §12.5.3, whose note had asserted the thing that was false.

## What the round did

The item's headline number could not be reproduced at first: the first honest instrument said
**567 µs**, not 10–19 ms, and three settings were each wrong before it agreed with the owner's
trace. The resize arm read 190 ns because the ISO specification states §12.3.2.1's `/OpenAction`
with a magnification and therefore opens at a *fixed* scale, where dragging a window edge changes
nothing — a fit mode has to be pressed first, and then the two gestures track each other within
1%, which is ADR 0766's claim measured rather than inferred. And the arrangement has to be scrolled
across a page boundary: the document's own catalog says `/PageLayout /OneColumn`, so a reader spends
most of their time with two pages on the screen and the item's "two ISO-spec pages" is the ordinary
case rather than a corner. With all three right, the worst of the 341 view-dependent pages read
**13.99 ms**.

That third setting is what exposed the defect, because the two pages were not paying for the same
reason. Page 187's 13.99 ms was almost entirely page 188 — a heavy page with no `NoZoom` annotation
anywhere on it, re-interpreted every notch because its *neighbour* had one. After the change page
187 is **1.11 ms** and page 1001, which is itself the heavy view-dependent page, is unchanged at
3.81 ms. Both halves of that are the point: the fix does what it claims and does not touch what it
does not claim.

The ledger's §12.5.3 row had carried the false sentence in writing since the clause was
implemented — "`Interpretation::view_dependent` says whether a page has an annotation that would
notice, so a page with none never re-interprets at all" — and so had the test named
`a_no_zoom_annotation_is_the_one_thing_a_zoom_re_interprets`, whose fixture is a one-page document
and which therefore asserted the first half of its own title. The new test states
`/PageLayout /OneColumn` over two pages whose annotations differ only in Table 167's bit 4 and
compares `Arc::ptr_eq` on their display lists across a zoom; restored to the old `reinterpret` it
fails, which is trap 13 satisfied before the claim was believed.

The readback half is the same correction rather than a second one. `reinterpret` cleared the whole
cache on the argument that lists and readbacks "are invalidated together everywhere else":
*together* is right and *whole* was not, so a wheel notch was dropping up to 1023 pages of readback
— the cache that takes a repeated document-wide search from 5.45 s to 7.27 ms. It now forgets
exactly the pages whose lists it dropped. The stronger claim is available and deliberately not
taken: `ViewState::magnification()` has one consumer in the tree and it produces only a transform,
so no character of a readback can move with the magnification at all — an argument about a path
rather than a property the types enforce, worth one page in an arrangement of two, and written into
the ADR so the next round need not re-derive it.

## What is left, and it is priced rather than described

The seam. `draw_annotations` runs last, so the clause's output is a tail on every accumulator, and
re-placing means keeping the content half and re-running the tail. Two constructions — restoring an
`Interpreter` from an owned snapshot of about forty fields, or merging two `Interpretation`s across
fourteen public ones with the span offsets shifted — plus three questions to answer before writing
code: what happens to §11.4.7's subtractive pair, whether the rebuilt list is a clone of the
content prefix per notch or a transform node in `pdf_render::DisplayList`, and what the residue is
actually worth once measured again. `doc/todo/46` carries all of it.

## Gates

`fmt` (both workspaces), `clippy --workspace --all-targets` and `clippy` over `fuzz/`, both under
`RUSTFLAGS="-D warnings"`; `nextest --workspace`; `--doc`; `selection_census` and
`accessibility_census`, which are what `doc/todo/02` §2's map assigns a `viewer-core` change; and
`cargo test -p conformance` for the ledger and the citations. `--bin pointers` and `--bin
quotations` were run over the moved documents and this round's four new pointers are all live.
