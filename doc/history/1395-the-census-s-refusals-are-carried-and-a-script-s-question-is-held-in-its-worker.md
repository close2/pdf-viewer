# 1395 — The census's refusals are carried, and a script's question is held in its worker

Scripts slot of batch sixty-two. ADRs 1626, 1627; no ledger row moves (§12.6.4.17 stays
`out-of-scope`, `Q286`), no question. User AI's tasks at the first heavy run: 211.
**Members (ADR 1626).** `global` per realm (`setPersistent`, `subscribe` refused); `event.commitKey`
(`ViewState::commit_field_by`), `fieldFull` and `changeEx` from Table 232's `/MaxLen` and Table 231's
`DoNotScroll`; `this.dirty` against `ViewState::mark_saved`, a write read back for its run only;
`this.info` read-only; `getOCGs()` by name, `state` through `set_group` (a locked group stays);
`util.printf` on `aform`'s writers; `buttonGetCaption`/`buttonSetCaption` as `/MK /CA /AC /RC`,
drawn and saved. The reference's examples are fixtures (`tests/census_members.rs`, `view_members.rs`).
**Depth.** Measured on 2, 4, 8 MiB threads: 22.8–25.6 KiB a bracket, 51.8 KiB a nested function,
3.4 KiB a prefix operator, 0.75 KiB a `+1` term (11 136 terms overflow 8 MiB, optimizer off).
`pdf_script::depth::estimate` costs each token at the dearest construct it begins; `Budget::depth`
4 MiB, `eval`/`Function` held to it through Boa's compile hook. Corpus deepest: 670 KiB.
**Optimizer off**: largest library parse 8.71 → 2.91 ms, evaluate 1.33 → 1.12 ms.
**Questions (ADR 1627).** Round 1396's ADR 1628 rules the window's thread never waits, so the worker
holds the script: `FRAME_QUESTION`/`FRAME_ANSWER`, `run` returns at once having changed nothing,
later triggers queue, `ScriptWorker::take_question`/`answer`, `ScriptRunner::waiting`/`take_resumed`,
`ViewState::apply_resumed` (a late refusal puts back the pre-commit value). Deadline restarted per
exchange; unanswered after 120 s the runner answers Cancel/No/OK or `null`. Client API sent to
1396 through the orchestrator. **`SIGSYS` at exit:** `sigaltstack` admitted; the test reads 159 → 0.
**Tier 1 column, per site** (runs, then throws, before → after): Calculate 15 249 → 15 318 and
9 473 → 9 488; Format 393 → 395 and 14 → 2; Keystroke 4 531 and 28 → 18; Validate 265 and 27 → 26;
Library 303 and 21 → 1; Page(Open) 17 and 12 → 4; WillClose 8 and 7 → 0; WillPrint 16 and 10 → 1;
DidPrint 7 and 1. Totals 20 789 → 20 860 runs, 11 188 → 11 308 finished, 0 → 3 finished refused
(`this.pageNum=` caught), 9 593 → 9 541 threw, `NotAllowedError` 86 → 0; ceilings moved to them.
Worker column: count for count, 141 workers, 0 lost, 0 `SIGSYS`; 20 runs put a question. **Beside
owners:** `pdf-model/src/appearance.rs` (the caption arm), `pdf-sandbox/src/lockdown_linux.rs` (one
call), `pdf-model/tests/script_properties.rs` (one clone), `doc/verify.md` (the `script` line).
**Gates.** `rustfmt --check` 0 on my 30 files. Clippy `-D warnings` 0: `pdf-model`, `pdf-script`,
`pdf-script-worker`, `pdf-sandbox` with the engine, and the two script crates without. Nextest 0:
2 106 / 2 106 with the engine (four crates), 25 / 25 without. `cargo test -p conformance` 0, 396
passed. Behind the lock, one `bounded.sh` run: Tier 1 column 0 (113 s), worker column 0 (99 s).
Fuzz `script` (`-s none`, by path), fresh seeds (6 272) each time: the first campaign found a
timeout in 187 s — the depth scan decoded the rest of the script per character, quadratic on a
`Function` of 630 000 characters outside ASCII; fixed to a four-byte window, pinned by
`a_long_run_of_characters_outside_ascii_is_scanned_once`. Two more stopped at 48 s and 91 s on the
seed `new Array(1048576).fill(0)` taking 2.1 and 3.4 s against the target's 2 s at nice 19 beside
siblings' builds (0.57–0.68 s a run alone; 95 ms in release, optimizer on or off). At nice 0: exit 0,
655 007 runs in 1 201 s, `INITED cov` 14 844, final `cov` 23 561, `ft` 67 997.
