# 1383 — A document's scripts share one realm, and a calculation walks its order

Scripts slot of batch sixty, RFC 0008 section 11 item 3 (b) and (c). ADRs 1602, 1603; no row moves,
no question. §12.6.4.17 stays `out-of-scope` until the owner amends `CLAUDE.md` beside Tier 1.
**(c), ADR 1602.** `pdf_script::Realm` is one Boa context per document; `Engine` holds it on a thread
of its own (16 MiB stack). Requests tell the realm the fields changed since it last heard, measured
by comparing the view's override maps. `ViewState::run_open_scripts(document, page)` runs Table 32's
tree in order, `/OpenAction`, page `/O`, then `/PO` — the call round 1384's hosts make after the
first present. `run_page_scripts` and `run_annotation_scripts` run Table 198's and Table 197's
scripts beside `perform_all`. Table 200 is item (d), untaken: no host moment marks a close or save.
**(b), ADR 1603.** A commit runs `/K`, `/V` (`rc` false refuses, said), every `/CO` `/C` through the
runner (`event.source`, `rc`, a runaway named "entry 2 of 3"), then `/F`. The drawn and saved
appearance read the runner's format (`AnnotationView::displayed`). `Field` reads 20 properties;
`getField` reaches any field, with `getNthFieldName`, `numFields`, `calculateNow`, `resetForm`.
Writes are edits: values into the log, `display` into the hide sets, `readonly` where `set_field`
refuses; colours, alignment, border and limit kept and reported as not drawn.
**Budgets.** The realm's heap bound is ADR 1609's 96 MiB `RLIMIT_AS`: Boa has no allocator hook, so
`realm_bytes` is gone. New `Budget::nesting`, 128 — **Boa 0.22's parser is unbounded**, about 26-32
KiB of stack a level (8 MiB parses 256, not 320). Realm heap, largest library: +19.1 MiB.
**Column.** 90 763 PDFs, 20 699 runs, 106 s: `ReferenceError`s are in 15 documents for 9 names.
None is a function the document's own library defines (`TFMC` 8 892, `defaultValue` 223,
`Matrix2D` 40, …; ADR 1602 section 7). Next: `util.printx` 59, `app.viewerVersion` 36.
**For 1386.** `FieldEvent`/`FieldResult` are `ScriptEvent`/`ScriptResult`; `Request::of`; `wire`
version 2. Its `end_to_end.rs` nesting test now asserts the named stop (one hunk, mine).
**Gates.** `rustfmt --check` 0 on my 18 files. Clippy `-D warnings`: `pdf-script` with and without
`engine` 0, `pdf-model` 0, `pdf-script-worker --features engine` 0. `cargo test -p pdf-script
--features engine` 0 (21 fixtures, 13 hook, 13 realm, 4 wire, 8 lib); without 0. `cargo nextest
run -p pdf-model` 0 (1 866). `cargo test -p pdf-script-worker --features engine` 0 (11). `cargo
test -p conformance` 0. Behind the lock: Tier 0 `script_corpus` 0 (347 held, 0 moved, 40 s);
Tier 1 column 0 (106 s); four realm measurements 0. All under `ulimit -u 8192`; user AI's threads
at the first heavy run: 247.
