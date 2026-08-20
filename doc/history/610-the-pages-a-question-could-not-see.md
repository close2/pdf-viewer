# 610 — The pages a question could not see

The fifth round of the block on the owner's decision that **the UI is now work**, and the one that
takes `doc/todo/30`'s remaining column item: the three questions that answered for the current page
alone while a window shows several.

Date: 2026-08-20.
ADR: [0445](../adr/0445-the-pages-a-question-could-not-see.md).

Touched: `crates/viewer-core/src/{query.rs, viewer.rs, lib.rs}`, its `tests/{headless.rs,
accessibility_census.rs}` and `examples/accessibility_cost.rs`;
`crates/viewer-accessibility/src/{tree.rs, lib.rs, bridge.rs}` and `tests/tree.rs`;
`crates/viewer-host/src/{status.rs, lib.rs}`; `crates/viewer-ui/src/bin/pdf-viewer/{access.rs,
app.rs, dispatch.rs}`; `crates/viewer-gtk/src/host.rs`; `crates/viewer-qt/src/host.rs`;
`crates/viewer-confined/src/{lib.rs, protocol.rs}`, its `tests/confined.rs` and
`examples/confined_panels.rs`; `crates/viewer-ffi/src/{session.rs, abi.rs}`,
`include/pdf_viewer.h`, `c/open_a_page.c` and `tests/{unsafe_position.rs,
header_and_library_agree.rs}`; `doc/conformance/ledger.toml` (§14.7, §14.7.5.2, §14.7.5.4),
`doc/ui-boundary.md`, `doc/todo/30`, `doc/todo/31`, the ADR and this file.

## The shape, and why it is not a message

Three variants changed shape and nothing was added: `Answer::Reports` is a `Vec<PageReports>`,
`Answer::Readback` a `Vec<PageReadback>`, `Answer::Accessibility` a `Vec<PageStructure>`, one entry
per page the arrangement is showing and has read, in page order — the population `Answer::Frame`
and `Query::PageGeometry` already answer over.

The boundary's test was asked as the round instructed and it does not fall the way 606's did.
*Which* pages are on the screen is the viewer's arrangement, which is why the input direction
needed `Command::Layout`; but a host asking these three already holds half the answer — it knows
which page it draws where — so the mechanism is 596's, 606's and 609's, used a fourth time. Five
consumers failed to compile, `PDFV_EVENT_KIND_COUNT` stayed 16, and the C ABI went 114 → 116 for
the reason `pdfv_frame_count` exists: a C caller cannot fail to compile, so a column has to be
something it can *ask* about.

## The clause, which decided the accessibility half

§14.7.5.2's marked-content identifier "uniquely identifies the marked-content sequence within its
content stream", Errata Collection 3's Issue #308 says the same MCID "may reappear across pages",
and §14.7.5.4 keys the route in from the page's own `/StructParents`. So two pages' trees share no
numbering and §14.8.2.5 gives no order between them: they are siblings, each with its own indices,
and joining them is AT-SPI's question rather than this crate's. `viewer-accessibility` answers that
one with a `Role::Document` node per page, banded identifiers, and an untagged page keeping its own
sentence beside a tagged one.

## What running it said

A real client on a real bus, on the note that states `/OneColumn`: **two page nodes where there was
one**, `page Cover (1 of 5)` at (0, 0, 500, 708) and `page Copyright (2 of 5)` at (0, 716, 500, 708),
each with its own subtree — the second page's element at y = 1145, a number the old tree could not
have produced.

## What is left

`doc/todo/30`'s third column item — the gap between two pages having the page's own colour on
`viewer-ui`, which is a change to both backends — and `doc/todo/37`'s two.
