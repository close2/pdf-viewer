# 731 — The tree two windows handed nobody

The tenth UI round, on the project owner's standing ask. ADR 0623.

## What it took, and the question it had to answer first

`tools/state.sh windows` was run first, as the briefing asked, and its reading named the work:
`Query::AccessibilityTree` and `Query::Readback` reached one window of three, so a screen reader on
`pdf-viewer-gtk` or `pdf-viewer-qt` was handed a picture. That is `doc/todo/31`'s and it was the
largest of the three debts left after 726.

The round could not start with code, because the debt has two shapes and they are not variations on
each other: publish §14.7's tree through each toolkit's own accessibility layer — GTK4's
`GtkAccessible`, Qt's `QAccessible` — or drive AccessKit directly as `viewer-ui` has since ADR 0214.

**It went to AccessKit, on the standard's argument rather than either toolkit's.** A screen reader
here talks to AT-SPI whichever route is taken, so what is actually being chosen is how many times
this project maps §14.7 onto somebody else's vocabulary. §14.7.3's role map is a `shall` on this
reader; §14.8.4's forty-one types are then mapped onto a platform vocabulary once. A toolkit route
inserts a third vocabulary between ours and AT-SPI's, does it *differently* in each of the two, and
puts a second tree builder outside the corpus census that ratchets the first — which is what "all
three hosts stay level" forbids in the one place a person cannot check for themselves.

## What is new

- `viewer_accessibility::Reading` and `Showing` — the six queries a host asks and when to ask them,
  taken out of `viewer-ui`'s `access.rs` before three windows made three copies of them.
- `viewer_accessibility::republishes` — which commands change what §14.7 says. `viewer-ui` had it
  privately; **a released pointer joined the list here**, because a delegated host receives an
  assistive technology's click as a press and a release rather than as an `Edit`.
- `Bridge::speak`, `Bridge::attended`, `Bridge::wait_millis`, `Bridge::LISTEN_MILLIS`.
- `crates/viewer-gtk/src/access.rs` and `crates/viewer-qt/src/access.rs` — the bridge up after the
  first frame, the tree published on a page turn or a resize, `Act` matched **exhaustively** in all
  three windows.
- `viewer-qt`: `QtPlace`, `window_placed`, `accessibility_wait`, `accessibility_pump` in the
  `extern "Rust"` half of the bridge; a second `QTimer` and `moveEvent`/`resizeEvent` in
  `cpp/window.cpp`. **The one hand-written `unsafe` token is untouched** and
  `tests/unsafe_position.rs` still passes.
- `viewer_host::trace::Topic::Access` and `viewer_host::delegated_click`.

## What was measured, and on what

Under `Xvfb`, on a session bus with `at-spi-bus-launcher`, `at-spi2-registryd` and `busctl` as the
client — `doc/verify.md`'s recipe, which now says what a native host adds to it.

- `doc/PDF20_AN001-BPC.pdf`, both native hosts: two page nodes, 32 elements, the tree `viewer-ui`
  publishes.
- `annotation-button-widget.pdf`: nine nodes declaring `click`, the same nine in all three windows.
- **`DoAction` on all seven `Link` elements of ISO 32000-2's cover opens the same URIs in all
  three**, read back off each host's own `link:` line.
- `Component.ScrollTo` answers true and moves nothing where the element is already on the screen.
- **Two applications on the accessibility desktop per native process**, both named for the binary:
  `accesskit_unix` embeds a root of its own beside the toolkit's. That is this decision's price and
  it is written down rather than left to be found.

## Five things that were wrong, and how each was found

- **The bridge came up before the first frame in `viewer-gtk`** — `accessibility bridge up` at
  1.656 s against `first frame on the screen` at 1.671 — because `refresh` runs once on the first
  allocation, before the document is open. `CLAUDE.md`'s startup rule, caught by the trace the round
  added. Gated on `presented`.
- **A click on a §12.7 widget does nothing in a host that delegates it.** `viewer-ui` toggles three
  of that document's nine actionable nodes; both native hosts toggled none while `DoAction` answered
  `true`. Refused by name now, 9 of 9. It also refutes `state.sh windows`' reading of
  `Query::FieldAt` as a delegation — that argument rests on a press landing on a control.
- **`state.sh windows`' population was short by a crate**, and its own `SPENT` check said so: moving
  the six queries into `viewer-accessibility` made it report `viewer-ui` reaching *fewer* queries
  than before, with `AccessibilityTree` credited to no window on the day all three began asking.
- **`viewer-gtk` claimed a floor it did not have.** Its module documentation said
  `#![forbid(unsafe_code)]` "is what makes subclassing the wrong answer here". Disproved by writing
  the subclass: a `GObject` implementing `gtk4::Accessible` compiles in that crate with the
  attribute untouched, because the lint does not fire on a proc macro's expansion. Trap 17, and
  ADR 0508's rule paying a third time.
- **`--trace` and `--trace=all` asked for four of five topics.** `EVERY_TOPIC` was the literal
  `0b1111` while `Topic` had grown to five, so 726's `Topic::Pointer` — the only instrument for a
  thing that cannot be photographed — printed nothing unless a person named it. Derived from
  `Topic::ALL` now, with a test run against the defect first (trap 13).

## Gates

Formatting, lint under `RUSTFLAGS="-D warnings"`, the workspace tests, the doctests, the fuzz
targets' `check`, both censuses after the sandbox worker was built (trap 10), and
`cargo test -p conformance`. §4's `--bin quotations` and `--bin pointers` show only their standing
false positives. §5's binaries are installed, from this worktree's own build directory.

`doc/conformance/ledger.toml` §14.7 and §9.10.2 both carry what this round did.
