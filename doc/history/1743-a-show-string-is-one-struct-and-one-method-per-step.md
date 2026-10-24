# 1743 — A show string is one struct and one method per step of §9.4.4

Session 1743, slot 8 of batch 113, the standing code-quality slot; ADR 2320; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `Interpreter::show_text` in `crates/pdf-model/src/content/text/show.rs` (390 lines under
`#[expect(clippy::too_many_lines)]`) keeps the setup and a five-call loop; `ShowString` and `CodeAt` carry the
state; the steps are `read_code`, `place_in_text_layer`, `draw_code`, `draw_program_code`,
`account_for_a_missing_mark` and `advance_past`. The `expect` is gone. `BDC`'s arm in `content/run.rs` is
`begin_marked_properties`.

**Proof.** The multiset of each file's non-comment lines against HEAD's (script `split.py`, anchors asserted):
`show.rs` loses only the `expect`, three `mut`, and the names of the state written through the struct; it gains
signatures, the two structs and field bindings. `run.rs` gains a signature, a call and a brace and loses nothing.

**Teaching.** `how-text-is-selected-and-copied.md` names the steps in stage 1 and its diagram.

**Gates.** `cargo clippy -p pdf-model --all-targets` (pedantic, `-D warnings`) exit 0; `rustfmt --check` on both files exit 0; `cargo test -p pdf-model --no-fail-fast` exit 0, 661 library tests among them (the first run stopped at `ccitt` on a stale worker until `pdf-sandbox` was built); `raster_golden --ignored` on the export passes, 2 tests (19.71 s). Against HEAD's six arms on an export of this round's two files: cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and no page differs in frame digest or mean error (2 638 s under the lock, load average about 24). `cargo test -p conformance`: `names`, `teaching`, `ledger_notes`, `evidence`, `adr_status` and `records` pass; `batch` (the raster examples list lacks `sample_pattern`) and `fuzz_workspace` (`ctutils`) fail on siblings' files.

**Found, not fixed.** `show_text`'s setup is still about 100 lines of which most are the §9.3.6 and §11.7.4.4
reasoning. `run_text_op` remains the longest arm in `run.rs`. `cargo test -p pdf-model` stops at `ccitt` with a
message naming trap 10 until the `pdf-sandbox` worker is built in the same profile first.
