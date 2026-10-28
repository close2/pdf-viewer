# 1855 — A document opened beside is asked when its frame is shown, and a form's JavaScript runs in the worker

Slot 5 of batch 126, 2026-10-28, a "script" round (Opus 5.5), two sessions; ADRs 2544 and 2545 used, Q721 left unused.
I worked in `scratchpad/r1855/` and the shared worktree, no export; the first session handed over at the handover point.

**Premise.** "(1) is `Due` keyed by document": it did not hold — `Due` lives in `Showing`, which each tab swaps; the
miss is the moment, a document behind the first having its page drawn while behind (driven under Xvfb with two
documents, GTK logged `Report`/`Presented` once). "(2) is ADR 2524's conversation with a JavaScript realm": held, with
one question more, a path from a given container (XFA 3.3 page 107).

**Built (ADR 2544).** `report::owed`, `Due::is_owed`; GTK's `refresh` and Qt's `painted` ask from the main loop when a
frame of the focused document is held. No C++ change. Driven again: each window asks once for each document.

**Built (ADR 2545).** `pdf_script::xfa`'s `FormRealm`, `FormHost`, `som` (`this`, naked references, `rawValue`,
`isNull`, children, `resolveNode(s)`; the rest refused by name). `FormLanguage` widens `Runner::run` and
`run_form_script` at all eight implementers; `form_wire` carries a language byte and `ResolveFrom` (kind 14), greeting
`PDFSCW03`; the worker's `Conversation` is the `FormHost` of one lazily built `FormRealm` (`Budget::FIELD_EVENT`).
The withheld question names the language; the open note and the `warn` note no longer say JavaScript is not run.

**Ledger.** §K.2 stays `partial`: JavaScript calculations and `change`/`initialize` events run; the other events,
validations that are scripts and page 642's dismissal remain. `doc/todo/65`, RFC 0011 section 13 say the same.

**Plants.** Qt's test fails with the paint's hook taken out. Through the real worker and the view state's entry and
commit, `for (;;) {}`, a nested busy loop, unbounded recursion and a 90 000 × 90 000 loop are each stopped, the entry
kept, the other calculated value dropped, the report naming the steps limit, the worker not lost (0.43 s for all
four); the clock alone stops the last on the realm. The held-container bound refuses at 4 097 and runs at 4 096; that
test peaked at 23 MB against 14 MB for one that holds none (getrusage; `/usr/bin/time` is not installed).

**Gates.** `rustfmt --check --edition 2024` on my 22 files: exit 0. `RUSTFLAGS="-D warnings" cargo clippy --no-deps
--all-targets` on `pdf-model`, `pdf-script --features engine`, `pdf-script-worker` with and without it, `viewer-core`,
`viewer-host`, `pdf-formcalc`: exit 0 each (`viewer-gtk`, `viewer-qt`: exit 0 in the first session). `cargo test`:
`-p pdf-script --features engine` exit 0 (`xfa_javascript` 8 passed); `-p pdf-script-worker --features engine --
--test-threads=1` exit 0 (`form_script` 12 passed); `-p viewer-core`, `-p viewer-host`, `-p viewer-gtk`, `-p
viewer-qt`, `-p pdf-formcalc`, `-p pdf-model --test xfa_datasets`: exit 0 each. Fuzz: clippy and `fmt --check` exit 0;
`script_wire` 241 s on 43 seeds, 70 492 656 runs, exit 0, no artifact. `--bin cited`, `--bin quotations`: exit 0.
`cargo test -p conformance --no-fail-fast`: exit 101, three reds, none mine — `every_quotation_is_the_standards_own_words`
(`pdf-model/tests/transparency_groups.rs:2213`, the first session's unidentified red), the evidence gate (record 1854)
and the records gate (records 1858, 1859). Not run: `--test script_corpus` behind the lock. Durations about 4 600 s
and 5 100 s.
