# 3122 — A printed sheet's type 1 grid is asked for its faults after the sheet is drawn

Session 2105. Status: **implemented** in `viewer-core` (`query.rs`: `PrintPage::faults_after_drawing`;
`viewer/answers.rs`: `print_page` keeps the reading's `AfterDrawing`) and the three hosts that print
(`viewer-gtk`, `viewer-qt`, `viewer-ffi`, each `printing.rs`). Closes ADR 2848's item "The type 1 grid's caller";
follows ADR 2946. Clauses: ISO 32000-2 §7.10.5.3, §8.7.4.5.2. Row §7.10.5.3 stays `implemented`.

## What the item asked, read against today's code

ADR 2848 left the question `Interpretation::function_faults_after_drawing` with no host asking it, and the 2063
record restated that. ADR 2946 (round 2066) answered it for the screen: `viewer-core` asks after every
`Command::RenderReady` and after the empty `Command::Declined` a host sends when it draws after answering. It named one
route it did not cover: **a page printed** — `print_page` words its reports before the host draws the sheet and kept
no watch, so a fault met only on the sheet's grid, at the printer's resolution, was said nowhere. That is the host
path the item still asks for, and it is what this builds.

## Decision

`PrintPage` carries the print reading's `AfterDrawing` (a private field) and answers
`faults_after_drawing()`: the faults the sheet's draw met that `reports` does not already say, worded by
`report::describe` as the screen words them. Each host that rasterises a `PrintPage` asks it after the draw, a refused
draw included (it may have evaluated part of a grid), and says what it answers beside the page's reports: GTK's
`say`, Qt's and the C session's kept reports. A method rather than a command because the sheet is not the screen's
page: nothing about it is kept in the core, and its reading is the print operation's own.

Not covered, stated: the confined process's `Answer::PrintPage`, because a type 1 shading's list is `Uncodable`
(ADR 2946) and such a page refuses to cross as a print page at all, which the answer's encoder already says.

Test: `viewer-core` `function_faults_after_drawing::a_printed_sheets_grid_fault_is_asked_after_the_sheet_is_drawn`:
no fault in the answer's reports nor asked before the draw, one fault after it. Planted back — `print_page` handing an
`AfterDrawing::default()` — the test fails.
