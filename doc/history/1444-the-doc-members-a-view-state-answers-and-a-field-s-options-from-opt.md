# 1444 — The `Doc` members a view state answers, and a field's options read from `/Opt`

Slot 1 of batch seventy, 2026-10-08, a script round. ADRs 1724, 1725; no row moved, no question.

**Premise.** Held for the list (`NOT_BRIDGED` held 43 names, not 32). Did not hold for "zoom and
layout as `Request`s the edit log already carries": script edits reach a host as a page, a focus, a
timer and a sound; `view::Request::Display` is the action path — so the window's view is refused by
name (ADR 1724 §3). Nor for "`activeDocs` is Tier 2": RFC 0008 §4.2 admits it for this document,
bridged so. `documentFileName` is refused as the brief said, a departure from §4.2 recorded.

**Census first** (90 763 files, a counting patch applied and reversed; 79 s, 2.68 GiB):
`currentValueIndices` 9 documents, `numItems` 6, `getItemAt` 6, `calculate` 5, the word pair 5,
`zoomType`, `gotoNamedDest`, `activeDocs` 3; the page members none; `pageNum` 34, the control.

**Built.** Every page's §12.4.2 label, Table 31 boxes and `/Rotate` cross with the document
(`PageState`, at most 32 768); `getPageLabel`, `getPageRotation`, `getPageBox` in a documented
rotated user space, `BBox` refused; `gotoNamedDest` a page turn through §12.3.2.4's lookup, its view
not applied and said; `title`; `calculate` stops the `/CO` walk; `app.activeDocs` is `[this]`. A
field's `/Opt` and §12.7.5.4 selection cross (`FieldState::options`, `selected`); `numItems`,
`getItemAt`, `exportValues` (Table 230), `currentValueIndices` read and written as a person's
choice through `view::chosen`. A third list, `surface::REFUSED`, refuses with its own reason the
window's view, `documentFileName`, the history pair, the four `/Opt` rewriters. `NOT_BRIDGED` is 21
names, ranked in ADR 1724 §4, the word pair first.

**For slot 5.** The wire is version 10: `fuzz/corpus/script_wire` is re-seeded by `wire_seeds` (32
seeds; the directory is the main checkout's, through the worktree's link). `fuzz/fuzz_targets/script.rs`
has one hunk in `document()` (three pages) and one in `field()` (`options`, `selected`).

**Unfinished.** The window's view needs a host round (the view told, a script's request taken); the
`/Opt` rewriters an override the appearance, controls and save all read.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy --all-targets`
on `pdf-model`, `pdf-script` (with and without `engine`), `pdf-script-worker`: exit 0. `cargo nextest
run`: `pdf-script --features engine` 128, `pdf-script-worker` 4, `pdf-model` 2 003 passed, 0 failed;
the `/CO` gate planted off failed its test. `cargo check --manifest-path fuzz/Cargo.toml`: exit 0.
`cargo test -p conformance`: 417 passed, 0 failed. Behind the lock, `--tree 6`: the Tier 1 column
exit 0 (21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed, 0 over, as HEAD; 0 of 21 listed
reached, nor any of the 22 moved; 145 s, 1.46 GiB); the worker column exit 0 (149 workers, 0 lost, 0 `SIGSYS`; 127 s);
`pdf-model --test corpus` exit 0 (every ratchet at its ceiling); `pdf-model --test script_corpus`
exit 0 (356 held, 0 moved). `save_round_trip` not run: no save path changed.
