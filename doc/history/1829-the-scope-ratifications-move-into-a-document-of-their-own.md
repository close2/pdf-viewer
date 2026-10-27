# 1829 — The scope ratifications move into a document of their own

Slot 7 of batch one hundred and twenty-three, 2026-10-27, a docs slot (Sonnet, high); ADR 2492 used,
ADR 2493 unused; no Q. I worked in `scratchpad/r1829/`. No ledger status moves.
**Premise** (`doc/questions/A680`, "Q680 agree"): `CLAUDE.md`'s scope detail (dates, Q and A numbers,
ADR pointers, the amendments' constructions) moves into a document named in the file's "moved out of
this file" table, each exclusion, amendment and reason kept in `CLAUDE.md` in a sentence, and the word
"fifth" dropped from the XFA dynamic-half paragraph. **Hypothesis** held.
**Built (ADR 2492).** `doc/scope-ratifications.md` holds the full ratification of the four
exclusions and the three amendments to the authoring line, the assembling paragraph and the user's
edit paragraph; it is a row of the table in `CLAUDE.md` and a row of `doc/HANDOVER.md`'s index.
`CLAUDE.md` keeps one or two sentences per exclusion and per amendment, the boundary line, the
immutability of `pdf_syntax::Document`, "Revisit an exclusion by argument, never by attrition", and
the principles and the in-scope list untouched. "fifth" is dropped in both texts. The file went from
31 834 to 27 002 bytes.
**Proof that nothing moved out of the two texts.** A sentence diff of the old `CLAUDE.md` against the
new one plus the document (a script run as `python3 -I -B`, normalising whitespace and list markers):
of 211 old sentences 180 stand unchanged in `CLAUDE.md`, 29 unchanged in the document, 2 in neither;
the two are the lead-in "The exclusions, closed, each with its reason:" (kept, with a parenthesis
naming the document) and the XFA sentence that held "fifth" (in the document without it). New text:
12 sentences or table rows in `CLAUDE.md`, each the short form of a paragraph the document holds
whole, and 14 sentences or headings of introduction in the document. No rule is reworded or added.
**Found, not fixed**: `cargo test -p conformance --test conformance` fails
`every_table_designation_names_a_table_the_standard_captions` on `crates/pdf-jpx-write/src/file.rs`
and `lib.rs` (Table M.14, M.22 and M.23 of T.801 written without the standard's name), which is
slot 1's and not mine.
**Gates.** `cargo test -p conformance --test documents --test teaching` exit 0; `cargo run -p
conformance --bin quotations` exit 0 (16 diverging records, none in my files); `--bin pointers` exit 0
(8 absent, none in my files); `cargo test -p conformance --test evidence` exit 0, 2 passed (first run
failed on my ADR naming the scratch path, fixed); `cargo test -p conformance --no-fail-fast` exit
101, 2 targets failed: `conformance` (the Table M.* designations in slot 1's `pdf-jpx-write`, not
mine) and `evidence` (mine, then fixed, 2 passed on the re-run). Code is untouched, so against HEAD's
six arms nothing moves. About 700 s. **Validation** (the orchestrator): the Opus validator's verdict was PASS — its own sentence-level check found every old sentence in one of the two texts but the two reworded ones, every date and pointer kept, nothing weakened; one short form (the XFA reopening trigger) was given its exact wording back by the orchestrator.
