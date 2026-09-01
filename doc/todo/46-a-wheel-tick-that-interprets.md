# A wheel tick that interprets: §12.5.3's re-interpretation runs on the event thread

Status: **open — the shape is chosen and priced (ADR 0775); the first stage has landed and the
seam has not.**
Priority: 46 — performance a person feels directly, on a bounded population of documents.
Corpus: `doc/ISO_32000-2_sponsored_EC3.pdf` — the standard itself carries `NoZoom` annotations, on
341 of its 1023 pages by the instrument's own count. `examples/spec_annotation_census` is where the
share across corpora goes; §12.5.3's ledger row carries it.
Code: `crates/viewer-core/src/viewer.rs` (the `set_magnification` → `reinterpret` path),
`crates/viewer-core/src/open.rs` (`interpret` runs inside `Viewer::handle`),
`crates/pdf-model/src/content.rs` (`interpret_into`'s annotation pass, which is the seam).
Instrument: **`cargo run --profile gates -p viewer-core --example zoom_cost -- <file.pdf> [steps]
[page|all]`**, which times exactly what `viewer-ui`'s `--trace` line times — `Viewer::handle`, with
no host and no window in the number. It drives both gestures, both arms (a view-dependent page and
a plain one), and prints how many pages each step asks a render for.

**Three of its settings are load-bearing and each was wrong on the first attempt** (ADR 0775 §1):
the resize arm reaches this clause only under a *fit* mode, because the ISO specification opens at
§12.3.2.1's stated magnification where a window drag changes nothing; the arrangement has to be
scrolled *across a page boundary*, because the document's own catalog says `/PageLayout /OneColumn`
and one page on the screen understates a per-page cost by half; and the plain-page arm is what
attributes the cost, at 0.3 µs against milliseconds.

## The mechanism, and what ADR 0707 already took

§12.5.3 makes a `NoZoom` annotation's placement a function of the magnification, so a zoom of such
a page must re-interpret it. ADR 0707 fixed what that re-interpretation *broke* — it no longer
supersedes the ink, so stand-ins cover the render — but the re-interpretation itself still runs
synchronously inside the event that changed the magnification. With stand-ins restored it no
longer decides what the person sees; it still decides how fast the event loop turns during a
gesture. The wheel is not the only gesture on the path: `Viewer::settle` derives the magnification
from the *viewport*, so a resize under a fit mode is the same cost per drag step that a notch is
per notch (ADR 0766), and the instrument measures the two within 1% of each other.

## The shape, chosen

**Re-place rather than re-interpret**, and the measurement is what chose it (ADR 0775 §2). With
`draw_annotations` behind a switch, twelve runs a page, best of each: the annotation pass is
**1.6% of page 1001's 3.52 ms, 6.0% of page 10's 538 µs, and under the noise on page 504**. A
re-interpretation redoes a page's whole content stream to move an annotation that is a twentieth of
it at worst.

The other two shapes are settled and stay settled:

- **Interpretation off the event thread** — rejected as the *primary* answer. It is an async seam
  in `viewer-core`'s contract, it touches all three hosts and the headless tests, and after all of
  that the machine does the same work: it relocates 100% of the cost where the seam removes 94–98%
  of it. It stays available afterwards, over a much smaller residue, and would then be worth far
  less.
- **Debouncing to gesture-settle** — still unacceptable, unchanged. The frames rendered mid-gesture
  are real frames presented as correct; drawing them from the old magnification's interpretation
  would show the annotation at a size §12.5.3 says it never has. A stand-in may approximate; a
  rendered frame may not.

## What has landed (session 848, ADR 0775)

The re-interpretation is applied to **the pages the clause is about**. `Open::reinterpret` dropped
every on-screen page's display list and the whole readback cache; it asked the question of the
*arrangement* rather than of the page, so one `NoZoom` annotation anywhere on the screen cost the
interpretation of every page beside it. It asks per page now, and the readbacks go per page with
the lists.

Measured on the ISO specification, scrolled across a boundary so the column holds two pages:

| | before | after |
|---|---|---|
| worst notch over all 341 view-dependent pages | 13.99 ms (page 187) | **4.98 ms** (page 407) |
| page 187 | 13.99 ms | **1.11 ms** |
| page 10 | 1.19–1.58 ms | **0.568 ms** |
| page 1001 | 3.79 ms | 3.81 ms |

**Page 1001 is what is left to do.** It did not move, because it is itself a heavy page carrying
such an annotation — the case the seam is for.

## The remainder, with its price

`draw_annotations` runs **last** in `content::interpret_into`, so everything the clause's pass
contributes is a *tail* on every one of the interpreter's accumulators. Re-placing means keeping
the content half and re-running that tail. Two constructions, and the choice between them is the
first thing the round that takes this owes:

1. **Restore an `Interpreter` from an owned snapshot.** `Interpreter` borrows `&Document` and
   `&ViewState`, both owned by `Open`, so it cannot be kept across events and its state has to be
   extracted into an owned value and put back. That is about **forty fields** — the display list,
   the accumulators, `glyph_coverage` (the "this font drew nothing" report is computed over the
   whole page), and a dozen `Copy` flags §11.7.5.2 and §8.4.1 set. A field forgotten is a report
   silently lost, which is the failure mode this project cares most about, so it needs a test that
   compares a spliced interpretation against a whole one field by field, over a corpus of pages
   with annotations.
2. **Run the annotations into their own interpretation and merge.** Fourteen public fields rather
   than forty private ones, all on `Interpretation`, and a mismatch is visible at one call site.
   The cost is that the spans are relative: `describe_annotation` records offsets into `text`, and
   `marked`, `described` and `text_layer` all carry positions that have to be shifted by the
   content half's length. It also has to establish that the annotation pass depends on **no**
   leftover interpreter state — §12.5.5 says an appearance is a form `XObject` drawn from an
   initial graphics state, which is the clause that would make it true, and reading
   `draw_appearance` is what would settle it.

Either way, three things have to be answered before the code:

- **What happens to §11.4.7's subtractive pair.** `interpret_with_fonts` interprets the page
  *twice* for a page whose blending space is `DeviceCMYK` and merges the two lists, so the seam is
  two seams there. `examples/press_census` says how rare that is; a documented fallback to the
  whole re-interpretation for those pages is acceptable under principle 1 if its cost is written
  down, and is probably right.
- **Whether the full list is rebuilt by copying.** A render request carries one
  `Arc<DisplayList>`, so the re-placed page's list is the content prefix plus the new tail — which
  is a clone of the prefix per notch unless `Arc::make_mut` finds it unshared. Measure that clone
  on page 1001 before designing around it: if it is tens of microseconds it is the answer, and if
  it is not, the alternative is a *transform node* in `pdf_render::DisplayList` under which the
  annotation's commands sit, which makes the list magnification-independent again and is the
  architecturally right end state — at the price of all three rasterisers.
- **What the residue is worth after the stage above.** The table says 4.98 ms worst and about
  0.6 ms typical; a seam that removes 94–98% of it leaves roughly 0.1–0.3 ms. Re-derive it rather
  than believing this sentence (`doc/habits.md`, *a price is a claim*).

## What decides the priority

341 of the standard's own 1023 pages, and it is the document this project opens most. The worst
notch is now under 5 ms rather than 14, so this is no longer a gesture-breaking cost — but it is
still the whole of what a notch costs on those pages, and if the kernel-floor round lands and zoom
steps drop toward the 30s it becomes the visible fraction of the gesture again.
