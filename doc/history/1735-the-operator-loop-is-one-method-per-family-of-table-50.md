# 1735 — The operator loop is one method per family of Table 50

Session 1735, slot 8 of batch 112, the standing code-quality slot; ADR 2304; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `Interpreter::run_from` in `crates/pdf-model/src/content/run.rs` (1 003 lines, under
`#[expect(clippy::too_many_lines)]`) is 138 lines with its comments and the `expect` is gone. `run_operator` is the one
`match` over `Operator`; it calls `run_graphics_state_op`, `run_path_construction_op`, `run_path_painting_op`,
`run_colour_op`, `run_text_op`, `run_text_showing_op`, `run_xobject_op`, `run_shading_op`, `run_inline_image_op`,
`run_marked_content_op` (with `end_marked_content`), `run_compatibility_op` and `run_type3_op`. The loop's local state is
the private `Machine`; the token read is `next_step`, the keyword handling `settle_step`.

**Proof.** The multiset of the file's non-comment lines against HEAD's: 1 189 lines at HEAD, 1 457 now. The 135 that
leave are the thirteen local declarations (now `Machine`'s fields), `&mut x` written `x`, `x = …` written `*x = …`, the
`expect`, and arms whose `match` header moved; the 403 that arrive are the method signatures, the `let Machine { … }`
destructurings, the dispatch's operator list, `Machine::begin` and `into_leftover`, and the same statements with the
dereference. No statement inside an arm was added, dropped or reordered. The file did not move; no ledger row names a
line range, so none changed.

**Teaching.** `how-a-page-lands-on-the-screen.md` names `run_from` and `run_operator` in its table row and its diagram
node; `doc/todo/74` §15 says the function is split.

**Gates.** `cargo clippy -p pdf-model --all-targets` (pedantic) clean; `cargo test -p pdf-model` (build of the export, `pdf-sandbox` worker built first) passes, 661 library tests among them; `raster_golden --ignored` on the export passes (23 s). Against HEAD's six arms on an export of this round's one file: cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and no page differs in frame digest or mean error (3 031 s under the lock). The conformance tests named in the brief are in the report.

**Found, not fixed.** `run_text_op` is 125 lines and `run_marked_content_op` still 90 code lines of one `BDC` arm that
reads six property-list entries in turn; either could be a method of its own. The shared worktree's
`crates/pdf-model/tests/drawn_while_read.rs` did not compile for a time (`finished.raster`) while a sibling edited
`render-cpu`; it is that slot's, and the export ran HEAD's copy. `ccitt` and `raster_golden` fail with a message naming
trap 10 until the `pdf-sandbox` worker is built in the same profile first.
