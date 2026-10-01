# 1452 — A question file is an instruction document, and its `§` is ISO 32000-2's

Session 1308. Status: accepted. Context: `tools/conformance/src/documents.rs` (the population and
its argument), `tools/conformance/tests/documents.rs`
(`every_section_sign_in_an_instruction_document_is_iso_32000_2s`), ADRs 0987 and 0997 (a section
sign belongs to a document), ADR 1440 (`tools/state.sh main-checkout`). Code:
`tools/conformance/src/bin/section_signs.rs`, `tools/main-checkout.py` (`section_signs`),
`doc/environment.md` (*After a merge*, item 6).

## 1. What happened

The `§` gate failed in the main checkout and nowhere else: `doc/questions/Q169` line 18 wrote a
`§` after ISO 19005-4 and `Q170` line 12 after ISO/TS 32002. Both were written beside their answers
and left untracked there, so no worktree held them and every worktree run was green. A round may
not edit the main checkout, and the question was whether the files are the gate's at all.

## 2. The population is right

`documents.rs` takes `doc/questions/Q*.md` in and leaves `A*.md` out, and its reason decides this
case. An answer is the owner's words, and correcting it would change what the record says the owner
wrote. A question is **this project's** writing: both files open "Source: this round". And a
question is read to do work — while open, by the round the owner's answer sends back to it, whose
`Owes:` line builds on what the question established. The gate's purpose is exactly that reader:
every instrument this project has reads a `§` as a clause of ISO 32000-2. These two numbers happen
to name no clause of it, and the instruction documents are not yet held to naming one
(`documents.rs`, "the half this does not gate"); the next section number that is also one of ISO
32000-2's — ISO 19005-2's 6.3.3, the example `tests/documents.rs` opens with — resolves, passes,
and sends the round to the wrong standard's section. That is as true of a question as of a todo file.

Exempting `doc/questions/` as correspondence would exempt the half of the correspondence the
project writes, to save an edit the project can make. So nothing is exempted. The owner's session,
which commits these files, writes the other standard's section in words — "ISO 19005-4 section
6.2.7.3", "ISO/TS 32002 section 5.1.3" — and `doc/environment.md`'s *After a merge* says so, naming
the two files.

## 3. The main checkout's run is visible from the worktree

`tools/state.sh main-checkout` prints every `§` after another standard's name in the main
checkout's uncommitted documents under `doc/`. `--bin section_signs <root> [files]` reads the same
population through the same scanner as the gate — `documents::instructions` and
`citation::scan_prose` — under any root, and writes nothing. On its first run it named the two
lines above, among three uncommitted question files.

**One gap it shares with the gate, found here and not closed.** Q169's line 18 opens with a second
`§` whose document, ISO 19005-2, ends line 17. The scanner looks for a document's name on the
sign's own line, so a name wrapped onto the line before is a clause of ISO 32000-2 to both. Reading
across a line break changes `scan_prose` for every population the gates hold — the Rust sources,
the ledger's notes, these documents — and belongs to a round that can re-run all of them. Until
then the fix for a finding is the whole sentence, not the line.

## Costs

The owner's session has two edits to make before `cargo test -p conformance` passes in the main
checkout. `main-checkout` builds and runs one more `conformance` binary, a second or two warm.
