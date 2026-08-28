# 805 — A view is a question, and the answer has to go back

The reader's magnification and place on the page now survive a confined worker's death, exactly:
`Query::View` asks where they are looking and `Command::View` puts them back.

Date: 2026-08-28. ADR: [0737](../adr/0737-a-view-is-a-question-and-an-answer-that-goes-back.md).
Subject: `doc/todo/15`'s next piece — the view across a worker restart, which ADR 0734 named as its
own limit.

Touched: `crates/viewer-core/src/{command.rs,query.rs,viewer.rs,lib.rs}`,
`crates/viewer-core/tests/headless.rs`, `crates/viewer-confined/src/{lib.rs,protocol.rs,resume.rs}`,
`crates/viewer-ui/src/bin/{pdf-viewer-confined.rs,pdf-viewer/trace.rs}`,
`crates/viewer-ffi/src/{abi.rs,kinds.rs,session.rs}`, `crates/viewer-ffi/include/pdf_viewer.h`,
`crates/viewer-ffi/c/open_a_page.c`, three `viewer-ffi` tests, `doc/ui-boundary.md`,
`doc/todo/15-ship-the-confinement.md`, and the two documents above.

## What was taken

ADR 0734 made a worker's death a refusal of the page rather than of the document and stated its
limit: *"The magnification and the position on the page are not restored."* It named the shape of
the fix — a `Query::View`, answered by the viewer, held by the host per frame — and got half of it.
The other half is that the answer needs a **way back**: `GoTo(Index)` is absolute and `Zoom::Scale`
is absolute, but the third part of a view has only `Scroll { dx, dy }`, which is a delta the viewer
clamps. So the round added three messages rather than two, which is the largest addition this
vocabulary has taken since it was frozen.

The alternative — replay `GoTo`, then `Zoom`, then a `Scroll` of the difference against a second
`Query::View` — was priced rather than dismissed, and it loses on exactness: `a + (b - a) == b` is
not an identity in `f32`, and over two million uniform pairs in a device pixel's range **16.5% do
not round-trip**, the worst by 0.0039 px. It also makes a host know which of the three commands
resets the others. ADR 0737 §2 has the argument.

`viewer_confined::Reopen::page` became `Reopen::view`, which is ADR 0734's own rule kept: what a
resume goes back to is `Resuming`'s, because it is the part two confined hosts must not answer
differently.

**And `ConfinedError`'s `#[non_exhaustive]` is resolved rather than left** — 801 recorded it as a
tension. It stays, with the line written down in both places: `doc/ui-boundary.md`'s rule binds a
*vocabulary*, whose population is this project's own and every member of which a host must decide
about; that error is a *failure population* and it is the kernel's, and what a host decides about
one of them is `Resume` — two arms, closed, matched exhaustively, with the wildcard-free match over
every error variant kept inside the crate that declares them.

## What it cost the boundary

Two consumers failed to compile: `viewer-confined`'s wire (three matches) and `viewer-ui`'s trace
line for the command; `viewer-confined` and `viewer-ffi`'s `every_query_reaches_the_abi` for the
question. `PDFV_EVENT_KIND_COUNT` stayed where it is, because none of the three is an event. The C
ABI gained `pdfv_view`, `pdfv_set_view` and its first struct passed by value since it was written —
named `pdfv_viewing` for `pdfv_frame`'s reason, which is the second time C's one namespace for a
struct tag and a function has decided a name here. `PDFV_ABI_VERSION` did not move: a struct
*added* is a shape an old caller never passes. `MAGIC` moved `PDFVCF04` → `PDFVCF05`, on a reason
the bytes alone did not demand — see ADR 0737 §4.

`doc/state-of-play.md` was deliberately **not** touched: nothing it claims stopped being true, and
its confined-window entry has not recorded a boundary ADR since 0657.

## Proof, under Xvfb on the release programs (llvmpipe; illustration, not a gate)

`PDF20_AN001-BPC.pdf` on a 900×1100 virtual display, 800×1000 window, ADR 0734's instrument — a
`kill -9` on the worker, which from the host's side is what a ceiling breach is. **The reader is
zoomed twice and scrolled twice before the kill**, which is exactly the case ADR 0734 measured as
the cost of its limit.

| | device path | `--cpu` |
|---|---|---|
| the view the trace says crossed back | `View(Viewing { page: 1, zoom: Scale(1.855943), scroll: (153.125, 377.25) })` | the same |
| title before the kill / after the restart | *page Copyright (2 of 5)* / the same | the same |
| the window's pixels, before against after | **identical** (`magick compare -metric AE` → 0) | **identical** (0) |
| second worker started and confined in | 1.3 ms | 1.3 ms |
| and it still turns pages | *page 2 (3 of 5)* | *page 2 (3 of 5)* |

**The proof was calibrated against the behaviour it replaces**, which is the same discipline trap 13
asks of a test: with `Host::reopen` restored to ADR 0734's `GoTo(Index)` and the release program
rebuilt, the identical script reports **91 412 pixels differing** — 9.2% of the captured screen —
and 0 with `Command::View`. That is ADR 0734's own documented cost, photographed and then removed.

**§5's binaries were not installed into `target/`, and the rule behind it was kept rather than the
letter.** That section exists so that a measurement is never taken against a stale binary and so
that a person can run what the round built; this is not a fifth round, the round does not merge, and
the two programs the measurement used were built from the final tree immediately before the run, in
this worktree's own build directory — trap 15's rule about whose tree a binary carries.

One thing worth knowing about the instrument, found while reading a run that surprised me: **the
death is discovered by the next message, so the first key press after the kill is the one that finds
it — and that press is deliberately not re-sent** (ADR 0734). A script that reads the title after
the kill without pressing anything is reading a window that has not yet noticed. No zombie beside
the window after either run, and `q` exits.

## Trap-13 calibration

Every new or rewritten test was run against an injected defect before being believed; all ten
failed, and the suite is green as committed. **Two of the ten did not fail on the first defect I
injected, and both were the test's fault rather than the defect's:**

| injected defect | failed |
|---|---|
| `restore` drops the scroll | `a_view_answered_is_the_view_restored_exactly` |
| `restore` does not move the page | `a_restored_view_announces_the_page_it_moved_to` |
| neither clamp on the zoom ladder (`stepped` **and** `magnification`) | `a_view_is_not_what_a_host_asked_for_and_that_is_why_it_is_a_question` |
| the wire drops a view's scroll | `protocol::tests::a_view_crosses_and_comes_back_unchanged` |
| the wire drops a view's magnification | the same |
| `encode_command` carries a default view | `protocol::tests::every_carried_command_round_trips` |
| `showing` records only the page | `resume::tests::a_resume_returns_to_the_last_view_that_answered` |
| the window restarts at the page alone | `a_dead_worker_leaves_a_restart_owed_at_the_readers_view` |
| `pdfv_view` answers a scroll it did not read | `a_c_program_opens_a_document_turns_a_page_asks_a_query_and_gets_pixels` |
| `pdfv_view` answers a magnification it did not read | the same |
| `Host::reopen` restored to ADR 0734's page-only `GoTo` | the Xvfb proof, above |

- **The zoom ladder is clamped in two places**, and removing one of them left the test passing:
  `Open::stepped` clamps the step and `Open::magnification` clamps a `Zoom::Scale` again on the way
  out. A defect that only removed the first was not the defect the test is about.
- **The C program compared one answer with another**, so an accessor that reported the same wrong
  number twice satisfied it. The fix is that the *values* are asserted now as well as the verdict —
  `view: page 0, zoom 3 at 2.500, scroll 385.1,672.4`, which is the note's first page at 2.5 in an
  800×1000 window, where a scroll of (40, 120) is clamped to the page's own corner and the offset is
  not something a caller could have asked for. The same reading made the C program set the
  magnification and the scroll **before** reading the view it hands back, because a page that fits
  the window is scrolled by nothing at all and a defaulted answer would have passed.

## Gates

Run last, after the final edit, as `doc/todo/02-every-round.md` §2's change→gate map assigns for a
change in `viewer-core` plus four host crates plus documents: the four core lines, the fuzz check,
the two censuses `viewer-core` is under, the conformance gate, and the quotation and pointer
binaries. Not a fifth round (`tools/round.sh`), and nothing here can move a pixel of a rasteriser.

| | |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent |
| `cargo nextest run --workspace` | 2777 tests run, 2777 passed, 18 skipped, in 32.9 s |
| `cargo test --workspace --doc` | 1 passed, 0 failed |
| `RUSTFLAGS="-D warnings" cargo check` over `fuzz/` | silent |
| `selection_census` | 1000/1011 words selected (98.91%) over 453 documents; **0 selections differing from the interpreter's own readback**; 2094 caret offsets over 459 text fields; 0 panics |
| `accessibility_census` | 102 853 elements over 876 untagged pages answering the honest empty tree; 0 given structure they do not state; 0 lines whose characters and text disagree; 0 panics |
| `cargo test -p conformance` | 200 passed, 0 failed |
| `--bin quotations`, `--bin pointers` | in the sweep table below |

The reference-spawning gates (oracle, corpora, text extraction, quorra's corpus, fixed documents)
were **not** run, and that is the map's answer rather than an omission: this change cannot reach
what they measure, and §2's own note is that a gate spawning a reference on a loaded machine
measures the load.

## §4 sweeps

Fifteen sweep binaries run over the pristine `main` checkout at the branch point and again over this
worktree, and the two outputs diffed. Every delta accounted, and none of them is a finding:

| delta | what it is |
|---|---|
| `blockers`, `capabilities`, `tables`: six standing hits at new line numbers | the same hits, moved down their files by lines this round added |
| `parts`: `doc/ui-boundary.md:434` → `:470` | the same standing hit, moved down |
| `counts`: 8862 → 8868 governing sentences; 443 attributed counts unchanged | new prose |
| `overtaken`: 625 → 626 decision records; 49 overtaken unchanged | ADR 0737 exists |
| `inapplicable`, `owed`: every `named by N file(s)` rung up by exactly one | the new ADR and the new history file |
| `pointers`: 9064 → 9072, live 5142 → 5149, a form 197 → 198; **absent 98 and undefined 13 unchanged** | the new documents' own pointers resolve |
| `quotations`: 6733 → 6738 over 1089 → 1091 documents; **verbatim 2814 and diverging 38 both unchanged** | five phrases this round quotes are this project's own sentences — ADR 0734's limit, `Open::magnification`'s comment, `doc/ui-boundary.md`'s rule, and this file quoting two of them again. The sweep's *after* was taken before this file was finished, so the last two are from the run watched after the final edit; the two figures that could be a finding did not move in either |
| `tables`: 6976 → 6977 sentences naming a table, 2416 → 2417 key citations the table agrees with | `doc/ui-boundary.md` cites Table 29's `/PageLayout` once more |
| `unread`, `retired`, `callers`, `entries`, `overstated` | identical |

The `pointers` instrument difference 801 recorded holds here too: `tmp/hayro/hayro-jbig2/src/file.rs`
resolves in the main checkout and answers *no file of that name* in a worktree, because `tmp/` is
gitignored and `tools/worktree.sh` does not link it.

## Contradictions with the briefing

- The briefing said `doc/todo/15` names the exact restore as "a view question on the boundary", and
  it does — **and a question alone is not enough**, which the tree shows and the entry did not: the
  answer has no absolute way back, because `Command::Scroll` is a delta. The tree wins; ADR 0737 §2
  is the argument and `doc/todo/15` now says so.
- The briefing offered the `ConfinedError` question as optional. It fitted, it is resolved by
  argument rather than by changing the attribute, and the line is now in `doc/ui-boundary.md` beside
  the rule it qualifies.
- `tools/round.sh` reports the next session as 804 because `doc/history/` ends at 803; sibling rounds
  are in flight and have not written their files. Nothing was read from it but the fifth-round
  question, answered the same way for 804 and 805.
