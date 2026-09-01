# ADR 0775 — The page beside the one §12.5.3 is about

Status: accepted, 2026-09-01. Session 848, a loop round on `doc/todo/46`.

`doc/todo/46` names three shapes for getting §12.5.3's re-interpretation off the per-tick critical
path and says none of them is chosen. This round chose, and the choice is settled by a measurement
neither the item nor ADR 0707 had: **the annotation pass the clause forces is 1–6% of the
interpretation it forces.** So the shape to build is *re-place rather than re-interpret* — remove
the work — and not *interpretation off the event thread*, which moves all of it to another thread
and still burns it.

That build is more than a round. What landed is its first stage, and the first stage turned out to
be a defect rather than an optimisation: the re-interpretation was being applied to pages the
clause is not about.

## 1. The instrument, and why the item's number could not be reproduced without it

`crates/viewer-core/examples/zoom_cost.rs` drives `Viewer::handle` with no host at all and times
exactly what `viewer-ui`'s `--trace` line times — the item's own witness is
`zoom In … -> 3 event(s) in 19.3ms`, and `dispatch.rs` measures `self.viewer.handle(command)`
around nothing else. Three of its properties were each necessary to see the number the item
quotes, and each was wrong on the first attempt:

- **The suspect is removed by choosing a page, not by editing code.** The example drives a page
  whose `/Annots` set Table 167's `NoZoom` and a page whose do not. On the ISO specification the
  plain page's notch is **0.2–0.4 µs** and the view-dependent page's is milliseconds, so the
  attribution needs no profile: essentially the whole of a notch on such a page is this clause's.
- **A resize only reaches the clause through a fit mode.** ADR 0766 established that
  `Viewer::settle` derives the magnification from the viewport, so a window resize is the same
  gesture; but the ISO specification states §12.3.2.1's `/OpenAction` with a magnification, so it
  opens at a fixed scale where dragging the window edge changes nothing. Driven that way the
  resize arm read **190 ns** and would have been reported as a gesture that costs nothing. With
  `Zoom::FitPage` pressed first — what a reader does, and what ADR 0766's round did — the two
  gestures are within 1% of each other at every page measured.
- **The arrangement has to hold two pages, and the document's own catalog says so.** ISO 32000-2's
  catalog states `/PageLayout /OneColumn`. Scrolled to a page top, that shows one page; scrolled
  across a boundary — where a reader of a continuous document spends most of their time — it shows
  two. One page on the screen gives a worst notch of **3.8 ms**; two gives **13.99 ms**, which is
  the item's "10–19 ms per wheel tick on two ISO-spec pages", reproduced.

The example prints how many pages each step asks a render for, because a per-notch cost that is a
*page's* multiplies by what the arrangement is showing and a number taken with one page on the
screen cannot say so.

## 2. What the measurement decided

Timed on ISO 32000-2's pages 1001, 504 and 10, twelve runs apiece, best of each, with
`Interpreter::draw_annotations` behind an environment switch so that the two arms are one binary
and one sitting:

| page | interpretation | without the annotation pass | the pass |
|---|---|---|---|
| 1001 | 3.523 ms | 3.465 ms | 1.6% |
| 504 | 3.465 ms | 3.473 ms | under the noise |
| 10 | 538 µs | 506 µs | 6.0% |

**A re-interpretation redoes a page's content stream to move an annotation, and the annotation is a
twentieth of the page at worst.** That is the whole argument for shape 2 over shape 1, and it is
the argument the item asked for: shape 1 is "an async seam in `viewer-core`'s contract … touches
all three hosts and the headless tests", it leaves `interpret` no longer synchronous with the
event that caused it, and after all of that the machine does the same work. Shape 2 leaves
`Viewer::handle` synchronous — so the contract, the three hosts, the headless tests and
`interpret`-as-a-pure-function all stand — and removes 94–98% of the work instead of relocating
100% of it.

`doc/todo/46` carries the remainder with its price. The short version is that the seam is a
*checkpoint*: `draw_annotations` runs last, so everything it contributes is a tail on every one of
`Interpretation`'s accumulators, and re-placing means keeping the content half and re-running the
tail. Two constructions are available and neither is a round — restoring an `Interpreter` from an
owned snapshot (about forty fields, and a field forgotten is a report silently lost), or running
the annotations into their own interpretation and merging fourteen public fields with the span
offsets shifted. The item names both and what each would cost.

## 3. The first stage, and it is a defect

`Open::reinterpret` dropped **every** on-screen page's display list:

```rust
for on_screen in &mut self.on_screen {
    on_screen.interpreted = None;
}
self.readbacks.clear();
```

and `Viewer::settle` guarded it with `on_screen.iter().any(|…| view_dependent)`. So the question
"does this page's display list depend on the magnification?" was asked of the *arrangement*: one
`NoZoom` annotation anywhere on the screen cost the interpretation of every page beside it. The
ledger's §12.5.3 row had asserted the opposite in writing — "a page with none never re-interprets
at all" — since the round that implemented the clause.

It is asked per page now. `Interpretation::view_dependent` is exactly the predicate, so the guard
in `settle` is gone as well: `reinterpret` decides.

**The readbacks go per page with it, and that is the same correction rather than a second one.**
The old comment argued for clearing everything because "the two are invalidated together
everywhere else"; together is right and *everything* was not, so a wheel notch on ISO 32000-2 was
dropping up to 1023 pages of cached readback — the cache `find_cost` measures at 5.45 s → 7.27 ms
for a repeated document-wide search (ADR 0250's successor). A page whose display list is kept
cannot have a readback that moved.

The stronger claim is available and is deliberately not taken: `ViewState::magnification()` has
exactly one consumer in the tree — `content/annotations.rs`'s `ViewGeometry` — and it produces
only `Adjustment::transform`, so no character of `Interpretation::text` can move with the
magnification and even the view-dependent page's readback could be kept. That is an argument about
a path rather than a property the types enforce, it buys one page in an arrangement of two, and it
is not needed for the number; it is written here so the next round does not have to re-derive it.

## 4. What it bought

`zoom_cost` on `doc/ISO_32000-2_sponsored_EC3.pdf`, viewport 1100×1200, fit to the page and
scrolled across a boundary, medians of 8–32 steps, before and after in one sitting:

| | before | after |
|---|---|---|
| worst notch over all 341 view-dependent pages | **13.99 ms** (page 187) | **4.98 ms** (page 407) |
| page 187 | 13.99 ms | **1.11 ms** |
| page 10 (the first view-dependent page) | 1.19–1.58 ms | **0.568 ms** |
| page 1001 | 3.79 ms | 3.81 ms |
| a page with no such annotation | 0.30 µs | 0.29 µs |

The resize arm tracks the zoom arm within 1% throughout, which is ADR 0766's claim measured rather
than inferred.

**Page 1001 is the point of the table.** It did not move, because it is itself a heavy page
carrying a `NoZoom` annotation — the case this stage does not touch and shape 2 is for. Page 187's
13.99 ms was almost entirely its *neighbour*, a page the standard's clause has nothing to say
about.

## 5. The test, and why the existing one could not see it

`headless.rs::a_no_zoom_annotation_is_the_one_thing_a_zoom_re_interprets` is titled "and no other
page is" and its fixture is a one-page document, so it asserted the first half of its own sentence
and nothing about the second. The new test
`a_zoom_re_interprets_the_page_with_the_annotation_and_not_the_page_beside_it` states
`/PageLayout /OneColumn` over two 100×100 pages whose annotations differ **in one bit** — `/F 12`
against `/F 4`, Table 167's `NoZoom` set and clear with Print on both — and asserts `Arc::ptr_eq`
on the two pages' display lists across a zoom: not equal for the first, equal for the second.

It was run against the defect before it was believed (trap 13): restored to the old `reinterpret`,
it fails on the second assertion.

## 6. What this does not change

- **No frame is drawn from a stale magnification.** Nothing was deferred and nothing was
  debounced, which `doc/todo/46` names unacceptable and which this round did not go near: the
  re-interpretation still happens inside the command that changed the magnification, for the pages
  it is owed for.
- **The ink does not move**, so ADR 0707's stand-in still covers the render. `reinterpret` is
  still not `stale`.
- **A page arriving on the screen mid-gesture** has no interpretation to judge `view_dependent`
  from, so `reinterpret` leaves it alone and `arrange` interprets it once, at the magnification
  now in force. That is the same answer as before and it is the right one.
