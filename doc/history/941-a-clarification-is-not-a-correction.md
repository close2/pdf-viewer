# 941 — A clarification is not a correction, and a hundred thousand codes nobody encoded

Date: 2026-09-09 to 2026-09-10.
ADR: 0931 (a clarification is not a correction and not an opinion); 0932 (the codes a Type 1
program never encoded).
Question: `doc/questions/Q52`/`A52` — the owner obtained PDF Association TechNote 0010 and
answered with it.
Files: `crates/pdf-archive/src/clarification.rs` (new), `src/requirement.rs`, `src/report.rs`,
`src/table/metadata.rs`, `src/errata.rs`, `tests/corpus.rs`, `tests/verdict.rs`,
`crates/pdf-font/src/type1.rs`, `crates/pdf-font/examples/type1_encoding_census.rs` (new),
`tools/round.sh`, `doc/third-party-data.md`, `doc/todo/53`.

Both tracks, as `doc/todo/02` §1 requires: the spec-driven half read a normative clarification
against the clauses it names, and the demand-driven half closed a defect `doc/todo/53` had
recorded without a witness.

## The category the project did not have

`Q52` was asked in session 940 because three records disagreed about ISO 19005-2 section 6.6.6's
`parameters` field and none of them could be read in an authoritative form. The owner obtained the
one that could: PDF Association TechNote 0010, *Clarifications of ISO 19005, parts 1-3 for
developers of PDF/A creators and validators*.

Its A021 names ISO 19005-2 and -3 section 6.6.6 in a *Pertaining* line and carries an **ISO WG
Resolution** that both are to be read as if the TWG's proposal were part of the specification —
the proposal being that requirements on `xmpMM:History` are application requirements and so
irrelevant to ISO 19005 validation. It reaches all three fields, not the one the question asked
about.

**Part 4's row stands, and on evidence rather than on the answer's sentence.** A021 closes with a
note that the proposal was accepted in principle for PDF/A-next; PDF/A-next was published in 2020
as ISO 19005-4 with `action` and `when` still stated as requirements in its own section 6.7.5. A
note recording an intention does not outrank the standard published after it.

The interesting part is that this is a **third kind of input**. An erratum corrects the text; this
leaves the text intact and changes what the text asks of a validator. Principle 5 refuses
second-hand committee guidance, and correctly — `Q52` itself rejected two records for exactly
that. So ADR 0931 states the four conditions, all required, that separate the two: the document is
held and read here, it is published by the body that publishes that standard's corrections, the
item carries an ISO WG Resolution rather than a problem statement or a proposal, and the item
names the parts it reaches. **Four items in this very note show the working group leaving a
requirement alone**, which is what makes the third condition load-bearing rather than a formality.

`Check::OutsideValidation` is the row state that follows: not `Unchecked`, which is a debt and
nothing is owed here; not `Processor`, whose subject is a program's behaviour; not deleted,
because a reader looking for section 6.6.6 must find a decision rather than an absence.

**One witness moved the wrong way and is in the report rather than out of it.**
`6-6-2-3-3-t03-fail-b` omits `pdfaType:field`, which A029 permits in as many words, so it left
`agreed` for `missed`. The file is non-conforming — its property carries a field no value type
describes — but under a clause this crate does not check. Keeping the withdrawn rule would have
gone on producing the right verdict for a reason the committee had ruled out, which is not the
same thing as being right.

The note's licence is recorded as **stated copyright, no stated grant**: a copyright line, no
grant, no `dc:rights` in its XMP, and `pdfa.org` answering 403 to this machine. Two second-hand
reports say CC-BY-4.0, and a second-hand report of a licence is not a licence — so nothing in this
tree quotes the note, and `doc/third-party-data.md` says so rather than leaving a reader to infer
it from the absence of quotation marks.

## A hundred and nine thousand codes nobody encoded

`doc/todo/53` item 2 had been open since the five-hundred-and-fifty-seventh session, recorded
without a witness and left because the fix looked larger than the finding. The finding was larger
than the record.

`read-fonts` builds a Type 1 program's custom `/Encoding` as a vector pre-filled with `.notdef`,
so `Encoding::map` answers `Some(0)` for a code the array never mentions — *encoded, to
`.notdef`* — and `glyph_name` resolves the same slot, so neither accessor separates them. The CFF
reader beside it returns a real `None`. The two producers of `NameKeyed` therefore disagreed about
what *unencoded* means, while the module comment between them said they produce one shape.

The census this round wrote — `type1_encoding_census.rs`, over every corpus on this disk — counts
**724 bare Type 1 programs, 471 with a custom encoding array, and 109 789 codes claimed by a
resolved map that the array never assigns**. That is the ordinary shape of a subsetted font. What
kept it off the page is that the built-in table is consulted only where the PDF `/Encoding` names
nothing, so the wrong answer was computed a hundred thousand times and discarded at the last step.

**The fix was not the upstream API question the todo predicted.** The array is in the cleartext
header, before `eexec`, so which codes it assigns can be read without decrypting anything and
without a second font reader. A code the array assigns to a name the program does not contain
stays assigned, and that is section 9.6.5.2's own instruction rather than a convenience: the
distinction the API could not express is the one the clause draws.

Text extraction is unchanged at 99.3% over 974 documents, and the oracle's contradicted set is
unchanged at 61 pages, every one held by a group that already existed. Both are the expected
result and both are worth stating: this removes marks that should never have been drawn, in a
population no gate on this disk exercises.

## Two instruments that were not being read

`tools/round.sh` — the first thing `doc/HANDOVER.md` tells a round to run — could not open a round
on any branch this project uses. It reads the session number out of the branch name deliberately,
because a worktree is branched before its neighbours write their history files; but it read the
number as though it were the whole name, so `round-940/pdf-a-validator` reached a shell arithmetic
context as `940/pdf-a-validator` and `set -u` ended the script. Every branch here carries a slug.

And twice in two days a gate was consulted through a filter that hid its answer: `tools/state.sh
quick` was read by grepping its output for the words "fail" and "error" rather than for its exit
status, and `cargo fmt --check` was read through `head -5` with the diffs below the cut. **An
instrument consulted through a truncation is not consulted**, which is trap 1's sentence one
directory over.
