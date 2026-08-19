# 606 — The column a window could not hold

The first round of the block, and the first on the owner's decision of 2026-08-20 that **the UI is
now work**. Its item was Table 29's `/PageLayout`, which was read, reported, and not obeyed.

Date: 2026-08-20.
ADR: [0441](../adr/0441-the-column-a-window-could-not-hold.md).

Touched: `crates/viewer-core/src/{layout.rs (new), open.rs, viewer.rs, command.rs, query.rs,
interact.rs, lib.rs}`, `crates/viewer-core/tests/headless.rs`,
`crates/viewer-confined/src/{lib.rs, protocol.rs, protocol/panels.rs}` and its tests and example,
`crates/viewer-ffi/src/{abi.rs, kinds.rs, lib.rs, session.rs}`, `include/pdf_viewer.h`,
`c/open_a_page.c` and both header tests, `crates/viewer-gtk/src/host.rs`,
`crates/viewer-qt/src/{bridge.rs, host.rs, keys.rs}` and `cpp/{window.h, window.cpp}`,
`crates/viewer-ui/src/bin/pdf-viewer/{app.rs, trace.rs}`,
`crates/pdf-model/tests/names_are_bytes.rs` (two backticks, see below),
`doc/conformance/ledger.toml` (§7.7.2 and §12.2), `doc/ui-boundary.md`, `doc/todo/30`, the ADR and
this file.

## What was found on the way, and was not on any list

**`Viewer::pointer` applied §7.7.3.3's inverse twice.** It mapped its point into default user space
with `Self::user_space` and then put the *result* through `content::user_space_at` again to find the
annotation under the pointer. On every page whose crop box does not begin at the origin, and on every
page `/Rotate` turns, §12.5.5's appearance state, §12.6.3's four pointer triggers, the focus a press
gives a widget and §12.5.1's popup activation all hit-tested somewhere the pointer was not.

Trap 12a for the third time, and it survived for the trap's own reason: no gate clicks, and
`Query::LinkAt` next door applies the inverse once and was right — so the two disagreed in a place
nothing compared them. Found by reading the function in order to make it page-aware.

**Neither native host had a wheel binding at all.** `viewer-gtk` and `viewer-qt` never sent
`Command::Scroll`; under `SinglePage` at `Zoom::FitPage` there is nothing to scroll, so the gap was
invisible. A continuous arrangement is a thing a person moves through, so both gained one — a
`GtkEventControllerScroll` and a `PageArea::wheelEvent` — and the same chosen distance per notch,
because what a notch is worth is not a fact about a toolkit.

**Two `clippy::pedantic` warnings were already on `main`**, in `pdf-model/tests/names_are_bytes.rs`,
which is the shape `doc/todo/02` §2's merge paragraph describes. Fixed here rather than left, because
that section says warnings are errors.

## The measurements

Time to first page, `pdf-viewer-gtk --trace=launch` under Xvfb, three runs apiece, **both documents
in `OneColumn`, which is the layout each of them states**:

| | pages | time to first frame |
|---|---|---|
| `doc/PDF20_AN001-BPC.pdf` | 5 | 116.9, 117.1, 112.8 ms |
| `doc/ISO_32000-2_sponsored_EC3.pdf` | 1023 | 168.1, 171.5, 165.8 ms |

A document two hundred times longer opens in about one and a half times the time, on the continuous
path, which is what `CLAUDE.md`'s rule asks. The structural reason is in the ADR: a row is measured
only when it is about to be placed and `layout::MOST` bounds the walk — visible in the same trace as
`4126528 bytes into 8 texture(s)` once the 1023-page document is put into `TwoColumnLeft`.

## What was seen on the screen

Xvfb :78, 900×1100, all three hosts.

- **`viewer-gtk`**: `PDF20_AN001-BPC.pdf` opens in a continuous column, because that is what the file
  asks for; `l` cycles all six and each says which it moved to; `TwoColumnLeft` shows 1|2 and 3|4
  with page 5 opening the third row; `TwoColumnRight` leaves page one alone on the right, which is
  the bound book the clause describes. Twelve wheel notches scroll across the page boundary and the
  title bar becomes *Copyright — page 2 of 5*, which is `Event::PageChanged` and §12.4.2's label
  following the scroll.
- **`viewer-qt`**: the same document, the same six, the same scroll.
- **`viewer-ui`**: unchanged. It prints what the document asked for and that it has asked the viewer
  for `SinglePage`, and the trace shows the message going through in 2.8 µs.

## What is left

`viewer-ui` owes the column, and the ADR says what it would cost: a tier-2 surface draws one
`Arc<DisplayList>` per frame and `crate::stale`'s reprojection is keyed on that list's identity.
`doc/todo/30` carries it as the next item rather than as a footnote.
