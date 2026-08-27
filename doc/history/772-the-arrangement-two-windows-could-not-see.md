# 772 — The arrangement two windows could not see

General-improvement round, subject chosen by argument (ADR 0711): `tools/state.sh quick`'s own
host-vocabulary reading named `Query::Collection` **"a debt, with the sharpest clause here"** —
§12.3.5's "shall present the document as a portable collection", a `shall` addressed to a viewer,
met by one window of three. Spec-driven track, correctness rather than performance, no contact with
the three siblings' subjects (ledger-prose sweeps, font cache, errata rows).

## What landed

- `viewer_host::panel::collection_rows` — §12.3.5 for the two native hosts, toolkit-free:
  §12.3.5.2's folder tree, Table 155's visible columns in `/O` order, §12.3.5.1's `/D` as a new
  `PanelRow::emphasis` flag. Both hosts' `Tab::Files` ask `Query::Collection` before
  `Query::Attachments`; GTK marks the initial document with Adwaita's `heading` class, Qt with a
  bold `Qt::FontRole`, because the clause states no appearance and *which row it is* stays one
  decision.
- **A defect on no list, found by reading §12.3.5.2 beside the code**: `viewer_ui::chrome` had
  dropped files from the panel since ADR 0202 — every `<n>`-tagged file of a collection with no
  `/Folders`, and any file whose key names a folder identifier the tree does not state. Two
  sentences of the clause decide both ("[i]f no folder structure is specified … show all files … in
  a flat list"; all files "shall be treated as members of the folder structure"). Fixed in both
  copies; the orphaned-identifier case is a documented choice (root), since the producer broke
  "[t]he value shall correspond to a folder ID" and the clause states no remedy.
- Ledger: §12.3.5, §12.3.5.1, §12.3.5.2 rows extended with the new code, tests and the defect's
  record; `tools/state.sh`'s reading now says "not a debt, since ADR 0711".

## Calibration and the look at the page

Trap 13, three times: with the placement rule reverted to the pre-fix shape, two `viewer-host`
tests and one `viewer-ui` test fail; restored, all pass. Fixtures are written in the tests because
not one of the 974 pdf.js documents states a `/Collection` (trap 8's converse). Trap 1: both native
hosts run under `Xvfb :72` on a hand-built collection document — GTK and Qt each photographed
showing the folder tree, its `/Desc` detail line, the schema's columns, the orphaned file at the
root, and `report.pdf` bold as the `/D` names it. The hidden Table 155 field appears in no row.

## Gates

By the change→gate map: every touched crate is in the "no corpus gate" rows, no pixel of a page can
move, not a fifth round. Core lines: `fmt` clean · `clippy --workspace --all-targets` under
`-D warnings`, exit 0 · fuzz check, exit 0 · doctests 0 fail · `cargo test -p conformance` — 11530
citations, 1086 quotations all verbatim, 0 unreviewed, ledger 875 rows (445 implemented, 223
partial). §5's binaries built `--release` and installed into `target/` before the visual check.

**`nextest --workspace`: 2515 of 2516 run, one failure, attributed to the machine and not to this
tree.** `viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it` failed
on every full parallel run and passed alone (three consecutive runs of its binary, 12/12) — and the
A/B is the evidence this record rests on: the identical failure occurs with this round's whole diff
reverted (`git apply -R`, re-run, same single FAIL). The test asserts one `Drawing::settle` inside
a 16.7 ms real-time budget answers page one; beside ~2500 parallel tests and three sibling rounds
(load average 30.8 at capture) the drawing thread gets no core, `settle` runs dry and the page
arrives through the poll — the exact mechanism the test's own doc comment says would "measure the
machine". Pre-existing, load-shaped, not in this diff's crates' changed files; left for the merge
round's quiet-machine sequence rather than patched around from a worktree whose base `main` has
since moved under it.

Shared cache: not used this round — no oracle or corpus gate was owed, so no `PDFREF_CACHE` load.

## What is still owed

Table 153's `/View T`, `/Sort`, `/Colors`, `/Split` and the `/Navigator` layouts are read, carried,
and presented by nobody — three windows now draw one presentation where the clause describes
several. §12.3.5 and §12.3.5.1 stay `partial` for exactly that, said in their rows.
