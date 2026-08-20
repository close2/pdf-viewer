# 0445 — The pages a question could not see

Status: accepted.
Session: the six-hundred-and-tenth.
Supersedes nothing; amends ADR 0441's shape decision by using it three more times, and extends
ADRs 0134, 0214, 0325 and 0422.

## The defect, and which direction it points

Sessions 606 to 609 gave all three hosts Table 29's six page layouts, so a window now routinely
shows several pages at once. **Three questions did not follow the arrangement**, and all three read
`Open::interpreted()`, which is `on_screen`'s entry for `page_index`:

- `Query::Reports` — what the interpreter could not draw. Under a column it answered about one page
  while the reader was looking at several, so a report about a page *on* the screen was silently
  absent and a report about a page *off* it was present. Both directions are wrong and **the first
  is the one nobody would notice**, because a silence looks exactly like a page with nothing to say.
- `Query::Readback` — session 587's per-code shortfall counts (ADR 0422), the same shape.
- `Query::AccessibilityTree` — and this one is the sharpest, for the reason `doc/traps/instruments-and-reports.md`
  and ADR 0214 already state: **the person who cannot see the page is the one for whom a count in the
  title bar is no answer.** A screen reader walking a continuous document and handed one page's tree
  is being told the document is one page long.

## The shape, and why it is not a new message

`doc/ui-boundary.md`'s test asks whether a host could answer the question for itself. It could not:
which pages are on the screen is the *viewer's* arrangement — which is exactly why 606 had to add
`Command::Layout` for the input direction — and a host holds no page extents, no display lists and
none of ADR 0118's arithmetic. But it does hold **half** the answer here, which is the other
mechanism: it knows which page it is drawing where, from `Answer::Frame` and `Query::PageGeometry`.
So this is the fourth precedent in fifteen rounds for the same move — 596's `Event::Extracted`,
606's `Answer::Frame`, 609's `Answer::Selected` — and it is taken again rather than adding a message:

| answer | was | is |
|---|---|---|
| `Answer::Reports` | `&'a [String]` | `Vec<PageReports<'a>>` — `{ page, notes: &'a [String] }` |
| `Answer::Readback` | `Shortfall` | `Vec<PageReadback>` — `{ page, shortfall }` |
| `Answer::Accessibility` | `Vec<AccessibilityNode>` | `Vec<PageStructure>` — `{ page, nodes }` |

One entry per page the arrangement shows **and this crate has read**, in page order — the same
population `Answer::Frame` and `Query::PageGeometry` answer over. The notes stay *borrowed*: a
page's sentences are worded once during interpretation and kept, so answering for four pages is
four slices and one allocation rather than four copies of the prose.

`Answer::None` now means one thing for all three — no document is focused. The distinction
`Query::Readback` used it for is kept by the *list*: an empty list is "no page on the screen has
been read yet" and an entry whose `Shortfall::is_whole` is true is "this page was read and nothing
was lost". That is `Answer::Frame`'s own division between a tier-2 host (`None`) and a tier-1 host
with no frame yet (empty), and it costs a host nothing to tell apart.

## Why the structure tree is a list of pages and not one tree

This is the question the todo entry left open — "§14.7's structure is a *document's*, so what a
column should publish is a question about AT-SPI rather than about this crate" — and the standard
answers the first half of it outright.

§14.7.5.2 makes a marked-content sequence carry

> an integer marked-content identifier that uniquely identifies the marked-content sequence within
> its content stream

— *within its content stream*, which is one page's — and Errata Collection 3's Issue #308
(`Review`/`Completed`, found with `spec-errata emit` rather than `check`) adds the consequence as a
NOTE: "MCIDs are scoped by content stream and must start at zero, so the same MCID may reappear
across pages or XObjects." §14.7.5.4 then states the route in: "[b]ecause a marked-content sequence
is not an object in its own right, its parent tree key shall be found in the StructParents entry of
the page object or other content stream in which the sequence resides."

So the numbering is a page's, the route is a page's, and two pages' answers meet only in whatever
ancestors they share. A single flat list across the screen would have had to renumber one page's
tree against another's — and §14.8.2.5 states an order *within* a structure tree and none between
two pages to renumber in. Hence `PageStructure`, whose `parent` and `headers` indices are that
entry's own, said in the type rather than in a comment.

**What a column should *publish* is then AT-SPI's question, and `viewer-accessibility` answers it**:
one `accesskit::Role::Document` node per page under the `PdfRoot`, each named by §12.4.2's label,
each with its own `Status` group and its own extents — the page's rectangle rather than the
window's, because two page nodes claiming the viewport would be two nodes claiming one place.
Identifiers are banded: `Band::of(slot)` offsets the four existing ranges by `slot × 2^32`, so the
first page on the screen keeps exactly the identifiers a `SinglePage` tree has always had and no
page can reach the next band. The bound is a clamp rather than an assertion, because this tree is
handed to an assistive technology and a panic there takes the window with it.

**An untagged page in a mixed column keeps its own sentence** saying the document states no logical
structure. That is trap 5 in the place a column could have broken it quietly: a merged tree would
have swallowed the empty entry, and the page would have read as absent rather than as untagged.

## What it costs, measured rather than assumed

`viewer-core --example accessibility_cost` gained a fourth argument, `column`, which puts
`OneColumn` at half magnification so the question is asked of a screen rather than of a page.
ISO 32000-2's page 700, the document whose size makes this a question:

| | pages on screen | nodes | wall clock, best of 7 | instructions (callgrind) |
|---|---|---|---|---|
| `SinglePage` | 1 | 40 | **11.70 ms** | **178 M** |
| `OneColumn` | 3 | 174 | **21.03 ms** | **257 M** |

Three pages cost **1.44×** one page, not 3×, and 4.35× the nodes. Two reasons, and the second was
taken deliberately: the expensive part of §14.7.5.4's route is the *ancestry* between a page and the
root, which neighbouring pages share; and `Viewer::structure` takes the page out of
`OnScreen::object` — ADR 0124's cache — instead of asking `Pages::get`, which is a page-tree walk
costing 3.8 ms on this document's thousandth page and would have been paid once per page on screen.
The single-page figure is unchanged from ADR 0394's 11.7 ms, which is what says the cache swap paid
for the extra structure around it. **And the worst case is bounded rather than open**:
`viewer_core::layout::MOST` is 8, so no arrangement puts more than eight pages on a screen and the
question's cost is bounded by the first page plus seven marginal ones.

The launch path is untouched — all three questions are asked on demand and nothing about them runs
at open. Measured anyway, because `CLAUDE.md` makes it a gate: `target/pdf-viewer --trace=launch` on
the same 1023-page document, three runs, **148.1 / 141.4 / 162.6 ms** to first present, inside the
band sessions 607 and 608 measured.

## All three hosts, and the C ABI

- **`viewer-ui`** publishes one page node per page on the screen and titles the window from
  `Query::Reports` over all of them — it counted from the last `Event::Reported` before, so a column
  of four showed one page's count.
- **`viewer-gtk` and `viewer-qt`** name the page in the status bar — `Event::Reported` has always
  carried which page it is about and both hosts dropped it — and re-ask `Query::Reports` on a page
  turn and on a layout change, which is what "a host that cleared its status bar can ask again"
  means once several pages are showing. The wording is one function in `viewer_host::status`,
  because the third copy of a sentence is where two hosts stop agreeing about what they are saying.
- **`viewer-confined`** carries all three shapes across the pipe, `Reply::Reports`,
  `Reply::Readback` and `Reply::Accessibility` each a list with its page.
- **The C ABI gains two entry points, 114 → 116**, and it is `pdfv_frame_count`'s argument said
  again: a C consumer cannot fail to compile, so a `/PageLayout` putting four pages on the screen
  has to be something a caller can *ask* about. `pdfv_reported_pages` counts the entries and
  `pdfv_reported_page` says which page one is; `pdfv_reports_len` and `pdfv_report` both take the
  entry index, so a caller written against the old shape fails to compile rather than reading page
  one's sentences for four pages.

Five consumers failed to compile, `PDFV_EVENT_KIND_COUNT` stayed 16, and no `Command`, `Event` or
`Query` was added.

## How it was verified, and it is the bus

`doc/verify.md`'s recipe — `dbus-run-session`, `at-spi-bus-launcher`, `IsEnabled` set on the session
bus, `at-spi2-registryd` with a `DISPLAY` of its own, `Xvfb`, and a client walking
`org.a11y.atspi.Accessible` from the registry root through `busctl`. `doc/PDF20_AN001-BPC.pdf`,
which states `/OneColumn` itself:

```text
[frame] 'PDF 2.0 Application Note 001… — Cover — page 1 of 5…'  at (0, 0, 800, 1000)
  [document_frame] 'PDF 2.0 Application Note 001: Usage of Black Point Compensation'  at (0, 0, 800, 1000)
    [document_frame] 'page Cover (1 of 5)'       at (0, 0, 500, 708)
      [document_frame] ''                        at (35, 90, 428, 578)
    [document_frame] 'page Copyright (2 of 5)'   at (0, 716, 500, 708)
      [document_frame] ''                        at (113, 1145, 275, 174)
```

Two page nodes where there was one, each with its own extents and its own subtree — the second
page's element at y = 1145, which is inside the second page's own place in the viewport and a
number the old tree could not have produced. `--trace=access` beside it prints
`2 page(s) on screen, 32 element(s)`.

## The censuses, which are ratchets

`accessibility_census`: **102 853 elements, 0 defects, 876 of 876 untagged pages honest**, and
0 answers at `viewer-core`'s node bound — unmoved. `selection_census`: **98.91%**, 0 panics —
unmoved. Both were run after the final edit.

## What this leaves

`doc/todo/30`'s column list is down to its third item, the gap colour between two pages, which is a
change to both backends and belongs in a round of its own. `doc/todo/31` gains one entry: the
*cost* is now a screen's rather than a page's, so the marginal page is what to watch if a future
arrangement puts many more on the screen than three.
