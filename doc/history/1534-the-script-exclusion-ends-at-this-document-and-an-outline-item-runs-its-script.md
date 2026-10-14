# 1534 — The script exclusion ends at this document and this viewer, and an outline item runs its script

Slot 1 of batch eighty-five, 2026-10-14, a docs round. ADR 1904; no question. One row moves:
§12.6.4.17 `out-of-scope` → `implemented`, on "Upon invocation of an ECMAScript action, a PDF
processor shall execute a script that is written in the ECMAScript programming language".

**(a) Built.** `CLAUDE.md`'s exclusion now holds Q286's sentence, which A286 ratified. One phrase
differs: `quorra-confined` is pinned as "a window that cannot supply a runner", since it can ask
(ADR 1904 section 1). **The premise did not fully hold.** Q286 said both requirements were executed at
`on`. Of every invocation site, only §12.3.3's outline item did not reach the runner:
`activate_object` refused a bookmark's `JavaScript` at every level. `run_outline_scripts` now hands
it over at Adobe's `Bookmark`/`Mouse Up` event, with `/Title` as `targetName`. The row's note
carries A100's ruling, the `off` default and A193's answer 1; §12.6.4's summary counts eleven types
performed. **Unfinished:** §12.11.5 still rests on the old exclusion ("This program runs none"), a
§12.11 round's (ADR 1904 section 3); the reference's bookmark object is not built.

**(b) The hypothesis did not hold.** The grep's 245 lines hold no history. `tools/comment-history.py`
sorts them 0 history, 21 unread, 109 legitimate, 115 in code. The 21, each read, are names: a
reader's session (`viewer-ffi` 5, the `script_corpus` walks 2) or the rule's own tests
(`ledger_notes.rs` 11, `records.rs`, `round_numbers.rs`, `state_sections.rs` 1 each).
`session.rs`'s 49 are a `Session` type and a document session. The debt is held at zero by
`spelled_ordinals.rs`, `round_numbers.rs` and `ledger_notes.rs` (ADRs 1698, 1767). **Calibrated,
one gap:** `viewer-core/tests/headless.rs:3471`, "Sessions 408, 410 and 411", escaped every gate,
because the capitalised form was matched only in the singular. `spelled_ordinals.rs` now holds the
plural and failed on that line until it was rewritten, as were `action.rs`'s, `script_sites.rs`'s,
`open_sequence.rs`'s and `Exclusion::Script`'s comments stating a retired state.

**Files.** `CLAUDE.md`; `doc/conformance/ledger.toml` (§12.6.4.17, §12.6.4); `doc/state-of-play.md`;
`doc/todo/65-the-remaining-frontier.md`; ADR 1904; this record; `crates/pdf-model/src/{action.rs,
view/script_model.rs,view/script_sites.rs,view/scripts.rs}`; `crates/pdf-script/src/{wire.rs,
engine/bridge.rs}`, `tests/{hook,wire}.rs`; `crates/pdf-script-worker/src/client.rs`;
`crates/viewer-core/src/{interact,scripting}.rs`, `tests/{script_triggers,open_sequence,headless}.rs`;
`tools/conformance/src/ledger.rs`, `tests/spelled_ordinals.rs`.

**Gates.** `rustfmt --check`, 17 Rust files: 0. Clippy `-D warnings` on `pdf-model`, `pdf-script`
(`engine`), `pdf-script-worker`, `viewer-core`, `conformance`: 0. `cargo nextest run` on the first
four: 0, 2603 passed, 26 skipped. The new outline test fails with `interact.rs`'s hunk reversed.
`cargo test -p conformance --no-fail-fast`: 101, `names` and `state_sections` only on siblings'
mid-move files. `--bin quotations`, `--bin pointers`: 0, no hit here. `tools/batch.sh check`: 0
(1 earlier, on a sibling's record). Duration 3 300 s.
