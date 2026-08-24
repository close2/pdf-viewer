# 709 — The eleven questions with no symbol, and the doc comment that counted as a call

Date: 2026-08-24. ADRs [0576](../adr/0576-the-c-abis-other-half.md),
[0577](../adr/0577-counting-what-a-window-cannot-reach.md).

Touched: `crates/viewer-ffi/src/{answers.rs (new),abi.rs,kinds.rs,session.rs,panels.rs,lib.rs}`;
`crates/viewer-ffi/include/pdf_viewer.h`; `crates/viewer-ffi/c/open_a_page.c`;
`crates/viewer-ffi/tests/{every_query_reaches_the_abi.rs (new),header_and_library_agree.rs,unsafe_position.rs}`;
`tools/state.sh`; `doc/conformance/ledger.toml` (§7.7.2, §9.10.2, §12.2, §12.3.4, §12.3.5, §12.4.2,
§12.4.3, §12.5.6.14, §14.3.3, §14.7); `doc/todo/30-a-native-host.md`, `doc/ui-boundary.md`,
`doc/state-of-play.md`, `doc/traps/instruments-and-reports.md`.

The **sixth** round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking **item 5** of ADR 0509's
ordering and the gap the round before this one named.

`viewer-core` was not touched. No message was added and no variant changed shape — the eleventh
consecutive round since the six-hundred-and-seventh in which that has been true.

## Item 5: the C ABI's other half

`tools/state.sh hosts` reported `Query` reaching 20 of 31. All eleven reach a symbol now, and
`PDFV_ABI_VERSION` did **not** move for the largest addition this ABI has ever had: not one of the
fifty-two new entry points takes or returns a struct by value, which is the one kind of change that
constant exists to catch.

**The deliverable is the instrument.** `PDFV_EVENT_KIND_COUNT` protects a caller against a message
that *arrives* and protects it against nothing at all where a *question* is concerned — which is how
eleven accumulated with no symbol and no signal.
`crates/viewer-ffi/tests/every_query_reaches_the_abi.rs` matches exhaustively over `Query`, so a
question added to the boundary fails to compile in a test whose name says what it is for. It has **no
allow-list**, which is why all eleven had to land in one round; the enumeration's size is counted out
of `viewer-core`'s own source rather than written down; and all three of its assertions were run
against injected defects before being believed.

What each of the eleven turned out to be is in ADR 0576. Two things are worth repeating here:

- **six needed no new shape, and two cost one `pub` between them** — §12.4.3's threads and §14.3.3's
  properties are `viewer_host::article_rows` and `property_rows` already, so they cross as the panel
  handle this ABI has had since ADR 0346. ADR 0246 decision 3 holding for a third kind of host;
- **Table 147 is a keyed accessor**, not a struct and not nineteen symbols, because a struct passed
  by value would put that table's *size* in the ABI.

## §12.3.4 had to be designed against a defect rather than for a feature

The seven-hundred-and-fourth session found `viewer-ui` decoding a thousand miniatures at tab-open
while `/PageMode /UseThumbs` opens that tab as the document opens. So this ABI offers **no
`pdfv_thumbnails_read`** — there is no list-valued call for a C host to reach for — and
`pdfv_page_label` is a separate call, so a page list can name a thousand rows without decoding one
picture.

Measured with a C program outside this tree, compiled `-Wall -Wextra -Werror` against
`target/libviewer_ffi.so` and the header, after §5's release build, on a 233-page corpus document
carrying 231 miniatures:

| | miniatures | bytes | wall clock |
|---|---|---|---|
| the eight rows a panel is showing | 7 | 210 672 | **0.81 ms** |
| every page, which the ABI offers no call for | 231 | 6 952 176 | **21.6 ms** |

27× the work and 33× the memory, for rows nobody is looking at. All 233 page labels cost **8 µs**.

The in-tree C gate exercises all eleven on `PDF20_AN001-BPC.pdf`: `/PageMode UseOutlines`,
`/PageLayout 1`; 14 of Table 147's 18 entries answered; 14 property rows; page one called *Cover*;
its `/Thumb` 74×105 in 31 080 bytes with a conformant flag word; 11 occurrences of *the* in 22
shapes; 19 structure nodes with `Document` at the root; no collection, no open popup; and
`<7>report.pdf` resolved to *report.pdf* in folder 7.

## The window count, and the comment it counted as a call

704 wrote down that *nothing counts what a window cannot reach* the way `state.sh hosts` counts what
a C caller cannot. `tools/state.sh windows` does now: per host, for both enums, naming any variant no
window reaches at all. `viewer-host` is added to each window rather than counted on its own, because
a host calling `viewer_host::page_entry` reaches two queries without naming either.

**Its first run was wrong and that is the finding.** It reported both native hosts reaching §12.3.5's
collection, on the evidence of one line in `viewer-host/src/panel.rs` reading *"a different answer
([`Query::Collection`]) that this host does not yet ask"*. A count whose condition was "the name
occurs in the crate" reported the opposite of what the sentence said, four words later — trap 11
caught in the act, and `state.sh hosts` had carried the same condition since ADR 0509 without being
bitten. Both strip comments before matching now, through one shared helper, and trap 11 has a sixth
instance with the general rule attached: **a count over source text is a claim about what the text
*is*, and a comment is text.**

**A zero there is not automatically a debt**, which is written into the section and into ADR 0577:
all three windows learn about an edit from `Event::Dirty` and none asks `Query::Dirty`; a tier-2 host
never asks `Query::Frame` because it draws its own pixels; and most of what the native hosts do not
ask is a delegation, because a real `GtkEntry` owns its own caret.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctests, `check` over the fuzz targets, and `cargo test -p conformance` — the core plus the
conformance gate, which is `doc/todo/02` §2's row for a change to a host crate, a tools script and
the ledger. §5's binaries were rebuilt and installed before the measurement.

Nothing here can change a pixel a corpus gate rasterises: no `pdf-*` crate, no rasteriser, and
`viewer-core` untouched. The conformance gate caught one thing worth naming — an `ADR 0509 §4` in a
doc comment, which that gate reads as a citation of *this standard's* §4 and refuses by name.

## What is left, named rather than left silent

`AccessibilityNode::lines` — the per-character byte counts and boxes AT-SPI's `Text` interface wants
— does not cross. An element's own text is its `PDFV_ELEMENT_NAME`, so a C caller building a screen
reader has the tree and the extents and not the character offsets. Two accessors and no new decision;
it is in `doc/todo/30` item 5.

Nothing is queued for the owner's measurement loop: every number here is a C compiler, a corpus
document and a wall clock.
