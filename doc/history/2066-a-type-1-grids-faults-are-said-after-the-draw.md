# 2066 — A type 1 grid's faults are said after the draw

Session 2066, a pixels round in the gwg worktree. Row §7.10.5.3 (and §8.7.4.5.2, read); ADR 2946; no question,
so ADR 2947 and Q894 are unused.

**The slice.** ADR 2848's open item "The type 1 grid's caller": `Interpretation::function_faults_after_drawing`
answered a §7.10.5.3 fault met where §8.7.4.5.2's grid is drawn, and no host asked it.

**The premise.** Held: the only callers were `pdf-model/tests/function_faults.rs`. Narrowed: the confined pipe needs
no change, since a type 1 shading's `DeferredColours` is `Uncodable` on the marks arm, so its page is always drawn
inside the worker, whose viewer says the fault in the `Event::Reported` the pipe already carries.

**Built.** `pdf-model`: `AfterDrawing` (the page's watched cells and the faults said, answering only new ones) from
`Interpretation::after_drawing`; `function_faults_after_drawing` is its first answer. `viewer-core`: each
`Interpreted` keeps one; `said_after_drawing` asks it on every `RenderReady` arm and on `Command::Declined`, words
each fault as the page's other §7.10.5.3 faults are worded, keeps it with the notes and sends `Event::Reported`.
`viewer-confined`: a window drawn after its page was answered `Listed` is told as an empty `Declined`.

**Tests.** `viewer-core/tests/function_faults_after_drawing.rs`: `{ pop dup 0.5 gt { pop 1 1 } { dup dup } ifelse }`
says nothing before a draw, its fault once with the draw's answer and not at a second magnification; answered
`Listed`, nothing until the empty `Declined`; arms that agree say nothing. **Plant**: the ask in `rendered` dropped
fails the first test. The worker's empty `Declined` has no worker test of its own.

**Gates.** Over `pdf-model`, `viewer-core`, `viewer-confined`: fmt exit 0; clippy `-D warnings --all-targets` exit
101 on the new test's `expect` and `panic` in helpers, then exit 0; nextest under `xvfb-run -a`, unpinned, 3 337 of
3 337 passed; rustdoc `-D warnings --document-private-items` exit 0; `cd fuzz && cargo check --bins` exit 0.
Conformance after `git add -N`: exit 101 on this record's unwritten gates paragraph, then exit 0. Tier 2 is the
train's; no `raster_golden` page can move, since nothing drawn changed.

**Left of the rows.** §7.10.5.3 stays `implemented`. A printed sheet's grid is not asked (`print_page` words its
reports before the host draws); ADR 2848's image-level table remains.
