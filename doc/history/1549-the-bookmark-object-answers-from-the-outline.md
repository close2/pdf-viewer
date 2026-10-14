# 1549 — The bookmark object answers from the outline, and `execute` is a click

Slot 4 of batch eighty-seven, 2026-10-14, a script round under the owner's switch to code improvement.
ADR 1934; no question; no row changes status. §12.3.3's row gains a code path and a test.

**Premise.** It held. `bridge.rs`'s `ScriptSite::Outline => JsValue::null()`, the wire's tag 7, and
ADR 1922 §2's six refused members were all as the brief said.
**Hypothesis.** Half of it held. Sending the outline at the open was never needed. ADR 1762's rule
for the pages' words applies as it stands: the outline is read when a script spells `bookmarkRoot` or
an outline item's script runs. A launch has no runner, so `open_kinstructions` cannot move. The
gate agrees: 3976.1, 12755.6 and 68851.0 thousand, against HEAD's 3972.6, 12752.4 and 68847.1 in the
toml's own record.

**Built.** `DocumentState::outline`, a flat list in reading order with each parent named by place,
read through `Outline::read` and bounded at 2^15 items. Wire version 14. `ScriptSite::Outline` names
the item's object. A new `engine/bookmarks.rs` carries `children`, `name`, `open`, `color`, `style`,
`parent`, `doc` and `execute` on the root and on every item. Writes are refused: the four Table 151
and 152 values as Tier 2's structure reason, `children` and `parent` as read-only. `execute` becomes
`ScriptEdit::Bookmark`. The view state holds it only for an item of the outline it told the realm
(new `view/script_bookmarks.rs`). `Viewer::handle` then activates the item through
`activate_object`. `event.target` at `Bookmark/Mouse Up` is the item. The wire's `edit` decoder had
grown past clippy's line bound and lost its property arm to `property()`; the round trip covers it.

**Files.** `crates/pdf-model/src/view.rs`, `view/{script_bookmarks,script_model,script_sites,scripts}.rs`;
`crates/pdf-script/src/{wire,surface}.rs`, `src/engine/{bookmarks,bridge,mod}.rs`,
`tests/{census_members,fixtures,hook,wire}.rs`; `crates/pdf-script-worker/src/client.rs`;
`crates/viewer-core/src/{scripting,viewer}.rs`, `tests/script_triggers.rs`;
`fuzz/fuzz_targets/script.rs`; `doc/conformance/ledger.toml` (§12.3.3's `code` and `test`);
`doc/state-of-play.md`; ADR 1934; this record.

**Gates.** rustfmt clean on every file. `clippy -D warnings --all-targets` exit 0 on `pdf-model`,
`pdf-script` (with `engine` and without), `pdf-script-worker` and `viewer-core`. `nextest` exit 0:
2240 passed on the first three, 380 on `viewer-core`. `cargo test -p conformance` 436 passed. Four
plants, each a patch applied and reversed, and each turns a new test red: the reading call removed
(4 of 5 red), the membership check (1), `execute`'s note (1), and the viewer's carry-out (1).
`t2-script_corpus` exit 0: 356 held, 0 moved. `t2-script_corpus_engine` exit 0: 21101 runs, 9315
threw, 0 over a budget, within the held ceilings. `launch_path`'s counted half, one `--clock` hold:
exit 0, 31 figures banded, 0 outside. `tools/batch.sh check` exit 0, the fuzz workspace included.
Duration 3278 s.
