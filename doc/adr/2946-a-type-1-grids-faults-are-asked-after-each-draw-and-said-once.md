# 2946 — A type 1 grid's faults are asked after each draw of the page, and said once

Session 2066, a pixels round in the gwg worktree. Status: **accepted and built** in `pdf-model`
(`content/report.rs`: `AfterDrawing`, `Interpretation::after_drawing`), `viewer-core` (`viewer/opening.rs`:
`said_after_drawing`, called from `rendered` and `declined`; `open.rs`: `Interpreted::after_drawing`) and
`viewer-confined` (`worker.rs`: `draw_windows` tells the viewer of every window it draws). Builds ADR 2848's item
"The type 1 grid's caller"; follows ADR 2940.
Clauses: ISO 32000-2 §7.10.5.3, §8.7.4.5.2. Row §7.10.5.3 stays `implemented`.

## The clause

§7.10.5.3: "It shall be an error for the number of remaining operands to differ from the number of output variables
specified by Range or for any of them to be objects other than numbers." and "it shall be an error to overflow the
stack." §8.7.4.5.2's type 1 shading is evaluated where its grid is drawn, after the interpretation's reports were
worded; since ADR 2940 every program that may fault is drawn by the processor's grid, whose evaluator records the
fault on cells the interpretation still watches (ADR 2848). Nobody asked them.

## Where it is asked

The core learns a draw is over in two places, and asks in both:

1. **`Command::RenderReady`**, every arm. A tier 1 host (`viewer-host`'s windows, `viewer-device`'s read-back lane,
   the confined worker's whole page) draws before it answers `Rendered::Raster`; `quorra` presents before it answers
   `Rendered::Presented`. A refusal is asked too: a draw that stopped may have evaluated part of a grid.
2. **`Command::Declined`**, now sent with an empty list by a host that draws after it answered — the confined
   worker's window, answered `Rendered::Listed` and drawn after the command. A new command would have crossed the
   confined protocol for one caller; `Declined` already means "a draw of this page is over, and here is what it left
   coarser", and an empty list is the true answer for a draw that left nothing.

The marks arm needs nothing: a type 1 shading's `DeferredColours` is `Uncodable`, so its page crosses as pixels or a
window, both drawn inside the worker on the cells the worker's viewer watches, and what the viewer says crosses the
pipe as the `Event::Reported` it already carries.

## Said once

`AfterDrawing` keeps the faults it has answered, seeded with those the interpretation said, and answers only new
ones; a page is drawn at every magnification a person stops at, and `Command::Declined` already says each of its
conditions once for that reason. A sentence said is kept with the page's notes, so `Query::Reports` says it again.

## Not covered, stated rather than hidden

- **A page printed.** `print_page` words its reports before the host draws the sheet and keeps no watch; a fault met
  only on a sheet's grid is not said.
- **A host that answers `Rendered::Listed` and draws later in its own process** without sending `Declined`. None does
  for a page with a type 1 shading today (the list crosses only where the marks are codable).

## Held by

`crates/viewer-core/tests/function_faults_after_drawing.rs`: a program erring for x above one half says nothing before
a draw, its fault once with the draw's answer and not at a second magnification; answered `Listed`, it says nothing
until the empty `Declined`; a program whose arms agree says nothing. Planted back — the call in `rendered` dropped —
the first test fails.
