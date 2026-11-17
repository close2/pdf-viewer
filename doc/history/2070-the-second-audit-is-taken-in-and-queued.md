# 2070 — The second specification audit is taken in and queued

Session 2070, the intake round of review 2038. The owner, 2026-11-17: "a new audit has finished. please take a
look at the new audit branch on origin". The orchestrator narrowed the round to section 0 of its brief (the
merge, the manifest re-run, the queue); the first fix group went to its own round so groups can run in lanes.

**The merge.** `origin/audit/specification-evidence-2026-11-16` (based on `b901326c`, 39 commits behind main)
merged with `--no-ff`, the round's one commit. Two conflicts, both sides kept: `pdf-transform/tests/archive.rs`
(main's `type3_width_as_drawn` and `type3_glyphs` modules, then the audit's appended tests) and
`viewer-gtk/src/host/printing.rs` (main's `/Enforce` dialogue `tests` module, then the audit's
`print_defaults_settings_spec_audit` module). Git refused the merge while `doc/arlington-pdf-model` was a
symbolic link in the worktree; it was swapped for an empty directory for the merge and restored.

**The manifest on the merged tree** (`expected-failures-current.json`). The eleven-package default-feature run:
7431 run, 7320 passed, 111 failed, 64 skipped. 110 are the manifest's in-scope identities, still failing; the
eleventh, `named_page_geometry_spec_audit apref_page_rotation_crop_and_inherited_resources_reach_the_widget_pixels`,
is new: main's c77df9e2 reports a simple font without `/Widths` (Table 109) as this viewer's layout, and the audit
fixture's font has none. The five GPU identities fail on the RX 580 with the manifest's pixel messages, and the
engine-only Matrix2D identity fails. D51's ignored gate passes, but only because its `PDFIUM-407-0.pdf` row is
skipped: `tika-issue-tracker/batch5` is absent on this VM (Q882). No main commit fixed a manifest identity.

**The queue** (`fixes.md`). All 318 findings (78 B, 172 T, 68 D), one line each, in 143 groups ordered by crate.
Four Sonnet helpers located each finding's source and test files; groups are the connected components of
shared production files, so two groups never share a source file (only `doc/conformance/ledger.toml`). G001 is
the owner's first group (B54, B07, B21, B57, B09, B10, B11 and the T and D findings in their files). A path
marked `?` was matched by subject only and is confirmed by the round that takes its group.

**Gates.** fmt `-p pdf-transform -p viewer-gtk` 0; clippy `-D warnings --all-targets` over the same two 0; the
broad run 100 (111 failures as above), the ignored gate 0, the GPU five 100, the engine one 100; `cargo test -p
conformance` 101, five tests, all from the audit's own files and none edited here (the owner's record): a `§`
after RFC 4013 in `pdf-syntax/src/crypt.rs`, "Table 3.1" in `calculator_functions_spec_audit.rs`, a ledger row
naming a test `accessibility.rs` no longer holds, a round number in `public_key_witnesses.rs`, and record 2038's
Gates paragraph. They are queued in `fixes.md` for the groups that own those files.

**Habit.** A worktree whose submodule path is a symbolic link cannot run a merge that touches the index there;
swap the link for an empty directory for the merge and put it back.
