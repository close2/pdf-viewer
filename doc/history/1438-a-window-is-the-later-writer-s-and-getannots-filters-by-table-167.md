# 1438 — A window is the later writer's, and `getAnnots` filters by Table 167

Slot 1 of batch sixty-nine, 2026-10-08, a script round. ADRs 1720, 1721; no row moved, no question.

**Premise.** Held for (1): the session map was `Open::popups`, in `viewer-core/src/open.rs` rather
than `viewer.rs`, read before the view state's overlay. Half held for (2): the reference lists six
filters but no `ANFB_ShouldNoView` or `ANFB_ShouldNoZoom`; its flag-shaped three are Print, View and
Edit, and AppearInPanel, Summarize and Export are Acrobat's pane, summary and export. Did not hold
for (3): no reader edits a text note's or a popup's `/Contents` here, only a free text annotation's,
and a popup is not one of the seventeen; the census's reason was wrong too (below).

**Built.** One map of windows in the view state, written by a click (`set_popup_open`, which
`toggle_popup` now calls) and by a script, so the later wins; only a script's write is saved, as the
window is shown at the save, and `this.dirty` measures that (ADR 1720). Print, View and Edit read
`crate::annotation::displayed` and `interacts` — the functions the page is drawn and hit-tested by —
once per annotation with `Hidden` set aside, carried as `AnnotationReach`, `Hidden` composed in the
realm; the other three are refused by name (ADR 1721). A text note's `contents` is a reader's edit in
the view state (`set_note_text`, a group subordinate refused), shown in its window and saved as
`/Contents` with the icon kept. Wire version 9.

**Census.** `app.viewerVersion` answers 0.1, this program's release (ADR 1615). Adobe's
attachments shim (read in `fileAttachment.pdf`) tests `v < 7`, then calls `getAnnots` only under
`v >= 6 && v < 7`; at 0.1 neither inner branch runs, so the seventeen never reach it.

**Unfinished.** No host command lets a person type into a text note's window; the view state's edit
waits for a host round. `contents` on the other fifteen subtypes stays refused by name.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
-p pdf-script -p pdf-script-worker -p viewer-core --all-targets` with and without `engine`: exit 0.
`cargo nextest run` on the four: 2 471 passed, 0 failed. Two planted defects (a script's write that
does not replace a click; Print ignoring `Hidden`) each failed one new test. `cargo check
--manifest-path fuzz/Cargo.toml`: exit 0. `cargo test -p conformance`: 415 passed, 0 failed (an
earlier run met 2 failures in `bounded.rs`, slot 6's file mid-edit). Behind the lock, `--tree 6`:
the Tier 1 column exit 0 (21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed, 0 over, as HEAD);
the worker column exit 0 (149 workers, 0 lost, 0 `SIGSYS`); `pdf-model --test script_corpus` exit 0
(356 held, 0 moved); `pdf-model --test corpus` exit 0; `save_round_trip` exit 0.
