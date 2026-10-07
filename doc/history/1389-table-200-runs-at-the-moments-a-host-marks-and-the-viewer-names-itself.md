# 1389 — Table 200 runs at the moments a host marks, and a script learns this viewer's name

Scripts slot of batch sixty-one, RFC 0008 section 11 item 3 (d) and the census's next members.
ADRs 1614, 1615; no row moves (§12.6.3 stays `implemented`, its note now says Table 200 runs);
§12.6.4.17 stays `out-of-scope` under A193's `Owes` line. No question.
**(d), ADR 1614.** `ViewState::run_document_scripts(document, DocumentTrigger, page)` runs the
catalog's `/WC` `/WS` `/DS` `/WP` `/DP` chain in the document's realm (`ScriptSite::Document`),
`event` `Doc`/`WillClose`… with the document as `target` (the open's too, now). `event.rc` false
is reported and the close, save or print goes ahead: Table 200 says only when each action "shall be
performed". A non-ECMAScript action in the chain is named, not performed. For round 1390, in
`viewer-core`'s `viewer.rs`: `WillClose` at `Command::Close`; `WillSave` before `ViewState::save`,
`DidSave` on its `Ok`; `WillPrint` at `Printing::Start` after `Standing::Proceed`, `DidPrint` at
`Printing::Finish`; and `ViewState::take_focus_request` after any call that ran scripts.
**Members, ADR 1615.** `util.printx` and `util.printd` (the `aform` functions; numbered formats
0/1/2, §7.9.4's spelling; XFA pictures refused), `app.viewerType`/`viewerVariation` `quorra`,
`viewerVersion`/`formsVersion` 0.1, `platform` `UNIX`, `language` `ENU` (A193 answer 11),
`Field.getArray`, `Field.setFocus` (`ScriptEdit::Focus`), and the four text flags writable
(`Property::TextFlag`; `comb` held to Table 231, setting `doNotScroll`). `pdf_script::wire::VERSION`
3. The reference's examples are fixtures (`tests/members.rs`).
**Columns.** Both walks now run Table 200 after the commits. In process: 90 763 PDFs, 3 845 fields,
20 789 runs — 11 188 finished, 0 refused-finished, 0 over a budget, 9 593 threw, 8 unparsed — in
100 s. Table 200 ran 31 times and threw 18 (`app.alert` 9, `this.dirty` 5, `global` 2, a
`ReferenceError` and a strict-mode `TypeError`). Refusals that remain: `global` 31, `app.alert` 15, `event.commitKey` 8,
`this.dirty` 7, `event.fieldFull` 6, `app.response` 5, `event.changeEx` 4, `util.printf` 4,
`this.info` 3, `this.getOCGs` 2, `Field.buttonSetCaption` 1 (86); every bridged member is at 0.
**Through the worker** (`pdf-script-worker/tests/script_column.rs`): the same 20 789 runs in the same
five columns, count for count, in 141 workers, 0 lost, **0 `SIGSYS`**, in 112 s; `Profile::Script` unchanged, no strace owed.
**Touched beside siblings.** `tests/script_corpus.rs` carries round 1394's ADR 1625 ceilings, which
it raised to 9 593 / 20 789 for my Table 200 runs; `tests/hook.rs`'s textColor assertion updated to
round 1390's ADR 1617 (the colour is drawn now, so no "not drawn" report).
**Gates.** `rustfmt --check` 0 on my 18 files. Clippy `-D warnings` 0: `pdf-script` and
`pdf-script-worker` with and without `engine`, `pdf-model`. `cargo nextest run`: `pdf-model` 1 874
/ 1 874; `pdf-script` + `pdf-script-worker` with `engine` 87 / 87, without 18 / 18. `cargo test -p
conformance` 393 passed, 2 failed — rounds 1390's and 1394's records in progress, not mine. Behind
the lock, one `bounded.sh` run: Tier 1 column 0 (100 s), worker column 0 (112 s), 249 s whole.
User AI's threads at the first heavy run: 249.
