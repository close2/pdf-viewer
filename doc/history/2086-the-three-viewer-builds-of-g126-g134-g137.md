# 2086 — The viewer builds the documentation audit left open: D02, D55, D59, and D53's probe line

Session 2086, an audit-fix round in `pdf-viewer-gwg`. The three findings of groups G123 to G143 that round
2082 left `open` (D02, D55, D59) and D53's owed probe line.

- **Fixed 4 of 4; argued 0, owner 0, open 0.** No owner audit file was edited.
- D02 (ADR 3040 builds ADR 3020): `viewer_core::interact::initial_document` presents a collection's
  named embedded PDF, or the first item in the panel's order, on the first open by the `GoToE` route with
  the container as its parent. A document presented inside a collection answers `Query::Collection`,
  `Query::Attachments` and `Command::Extract` with the container's (`viewer::answers::collection_frame`),
  so the panel still lists the collection. A document that cannot be presented keeps the container's pages,
  and the window says why. The hook is in `viewer/opening.rs`, which round 2084 also touches: one block
  after the open action.
- D55 (ADR 3041 builds ADR 3021 §1): `edit_of` is gone. `viewer_host::TypedPaths` keeps a file-select
  control's typed path until the commit, and `commit_of` reads the file there, once. GTK, Qt and the
  platform layer use the first, and `viewer-ui` uses the second at Enter. A refused read abandons the
  typing instead of committing a path with no file behind it. Both choosers commit. `chosen_reads` counts
  the reads.
- D59 (ADR 3042 builds ADR 3021 §2): `quorra-confined` carries `--restrictions=` and
  `--ignore-restrictions` and sends `Command::Restrict` before the document and after a restart, `Off` by
  default. An `ask` goes on its question card.
- D53: `WindowProbe.Controls` (Probe.cs), called from `ApplyChrome`, writes the status line's and the
  pane toggle's state as read back. It has not been built or run here. The CI step with a `/HideWindowUI`
  document is still owed (`doc/todo/30`).
- Ledger: 12.3.5.1's note now records the presentation, and 12.7.5.3's note names `commit_of`. No status
  moved.

**Gates.** Over `-p viewer-host -p viewer-core -p viewer-gtk -p viewer-qt -p viewer-ffi -p viewer-ui`: fmt
`--check` exit 0; clippy `-D warnings --all-targets` exit 0; `cargo build -p pdf-sandbox --bins`, then nextest
`--lib --bins --tests` under `xvfb-run -a`: exit 100, 1 failed of 1329, exactly the manifest's one for these crates (B73's `image_xobject_measurement_spec_audit`, round 2084's), none new and none fixed, so `expected-failures-current.json` is unchanged; rustdoc `-D warnings --document-private-items` exit 0;
`cargo test -p conformance --no-fail-fast` exit 0 once this record stated its gates and `doc/teaching/how-a-form-field-is-filled.md` named `TypedPaths` rather than a method of it. `fuzz/` was not checked: the one public item
removed, `viewer_host::form::edit_of`, is named by no fuzz target. The C# probe line was not built: there is
no .NET toolchain on this machine.
