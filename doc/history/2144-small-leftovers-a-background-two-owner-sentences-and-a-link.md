# 2144 — Small leftovers: a self-test's ratio, a background, two owner sentences, a link

A host round in the `gwg` worktree on the leftovers of rounds 2122 and 2129 and one merge failure.
ADRs 3277 to 3280; no question.

- **`tools/bounded.sh --self-test`** (ADR 3277). The flat-tree case failed at a merge ("one sample
  cost 1195 ms"). It now holds the walk to two ratios, each the least of three interleaved runs:
  at most thirty walks of a tenth of the table, and at most twenty plain reads of its rows. Under
  load the old case failed (1327 ms) and the new one held; two planted walks fail it.
- **§14.8.5.4.2's `/BackgroundColor`** (ADR 3278). `Tree::background_colour` reads it under
  §14.8.5.4.1's owners. `viewer_core` carries the element's own fill, or else the nearest one
  stated above it, a written-down reading of "treated as if its background were transparent"; it
  crosses the confined protocol and is set on the runs (AT-SPI `bg-color`).
- **The C ABI** (ADR 3278). `quorra_structure_layout` answers the alignment (round 2122's
  leftover) and the background, with `QUORRA_ALIGNMENT_*`, `AlignmentKind`, the C# declaration and
  constants, and 385 entry points; `open_a_page.c` reads both.
- **§14.8.5.5 and §14.8.5.7's owner sentences** (ADR 3279). `Tree::list_attribute` and
  `Tree::table_attribute` narrow the List and Table readers, each with a control test.
- **The WinUI link** (ADR 3280). `NativeWindow::hand_link_over`, `link_through` and
  `answered_through`: the C ABI's window owes a link as `QUORRA_OWED_OPEN_URI` after
  `handable_uri`'s scheme check. The C# `LaunchUriAsync` already handled that record, and only CI's
  Windows job compiles it.

**Ledger.** §14.8.5.4.2, §14.8.5.5, §14.8.5.7 and §12.6.4.8 stay `implemented`, each with its
note, code and tests extended; `/BackgroundColor` moves from inapplicable to exported.

**Gates.** Over pdf-model, viewer-core, viewer-confined, viewer-accessibility, viewer-host,
viewer-ffi, conformance: fmt 0; clippy `-D warnings --all-targets` 0 (after a 102-line function
was split); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 4670 of 4671,
the failure this round's (a blockquote attributed to the wrong clause), passing after; no
difference from `expected-failures-current.json`; `cargo test -p viewer-ffi` 0; rustdoc 0; fuzz
`cargo check --bins` 0; `cargo test -p conformance` 0 once this paragraph was written.

## Left

Watching `Launcher` open a link on Windows (`tools/drive-windows.sh`).

**Habit.** A self-test bound on a cost asserts the algorithm's shape as a ratio taken side by side, because
both processor time and wall time belong to the machine.
