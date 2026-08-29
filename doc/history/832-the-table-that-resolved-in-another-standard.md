# 832 — The table that resolved in another standard

Finding: **twenty-one references to ISO/TS 32002's Table 3 and Table 4 were being checked against
ISO 32000-2's Table 3 and Table 4 — the escape sequences in literal strings and the examples of
literal names — and passing**, because `read_tables` had no foreign-document rule and the numbers
exist in both documents. Found while building the gate `doc/todo/01` had been owed since round 820,
for a designation that is not a number at all.

Date: 2026-08-29. Argued in ADR 0760. (0757, 0758 and 0759 are sibling rounds'; this number was
taken above that reservation.)

Touched: `tools/conformance/src/citation.rs`, `tools/conformance/src/clause.rs`,
`tools/conformance/tests/conformance.rs`, `tools/spec-errata/src/renumbered.rs`,
`crates/pdf-model/src/{ecdsa,eddsa,x509,signature}.rs` (doc comments only),
`doc/conformance/ledger.toml` (the §O, §O.2.1, §O.2.2, §12.8.3, §12.8.3.1 and §12.8.3.4 notes),
`doc/todo/01-ledger-partial-rows.md`, `doc/adr/0760-…`.

## Why the standing item rather than a page

Both demand-side instruments were asked first, because `CLAUDE.md` ranks them above an instrument
round. The corpus gate's *whose defect it is* composition puts **one** document in the `this
reader` row and it is `doc/todo/22`'s Arabic free text, already read and pinned; the oracle's
*ambiguous, undiagnosed* list is empty. Neither names work, which is the clause under which the
standing item was takeable.

## What was built

- `read_tables` classifies a table another standard is named in front of as
  `Scan::foreign_tables`, on `another_document` — the rule `read_citations` has had for a `§`
  since the eightieth session. Both ISO 32000-2 populations, `tables` and `designations`, are
  cleaner for it, and the foreign ones are **printed** rather than reported: naming the standard
  that captions a table is correct writing, unlike naming one before a `§`.
- `every_table_designation_names_a_table_the_standard_captions`, and the same question over the
  ledger's prose. The wider population — `Annex O.3`, `D.2`, `125a` — had been checked by nothing
  at all.

## Three findings that were not the subject

**`another_document`'s character tests admit punctuation as a name.** The sets are permissive
because `ISO/IEC` needs a solidus and `32000-2` a hyphen, and `all` over a permissive set is
satisfied by a string of nothing else — so `///` passed as an acronym and `-` as a number, and the
first run reported a document called `/// -`. Latent on the `§` side for that arm's whole life.

**An annex's table has two spellings and both are the standard's.** ISO 32000-2 captions
`Table Annex L.1` and, five lines above the caption, writes "Table L.1 provides a legend for use in
interpreting Table L.2". `designated_table_title` takes both, one way only.

**The five places that broke the new rule were the five stating it** — `spec-errata::renumbered`'s
module comment and the three Annex O ledger rows, writing Issue #700's amended designations as
`Table Annex O.1`. Each says in its own words that such a citation would name a caption no reader
can find, and each was right about a gate that did not exist. The repair is in the writing, in the
convention the same paragraphs already use one sentence earlier: the erratum strikes a
*designation*, so the designation is what a sentence about it names. The gate then caught this
round's own first draft of two of those repairs.

## The half that cost the most

The positional rule alone left the listing still printing both wrong titles, because the four
modules about ISO/TS 32002 also write its tables *bare* — `Table 4's second curve has no
verification here` — with the document established a paragraph earlier. That is correct English
and wrong by the convention `read_tables` states, and no line-based rule can see it. The rule
cannot be loosened to carry a document across a comment: `eddsa.rs`'s own first line names ISO/TS
32002 and cites ISO 32000-2's Table 260 eight words later. So the writing was brought to the rule
— every bare one named, two broken across a line reflowed — and the one that stays is a verbatim
quotation of ISO/TS 32002, which is not edited to please a checker.

`Table 3 — Escape sequences in literal strings` and `Table 4 — Examples of literal names` are
`pdf-syntax`'s now, which is what those tables are about.

## Gates and sweeps

The full §2 sequence, twice — the second time after the `pdf-model` doc-comment edits, because the
change→gate map puts any change to that crate under everything. Every gate green and every figure
at its standing value: the corpus's incomplete count, the oracle's seven verdicts, the word-box
verdict and judged set, quorra's four, and the ledger's row counts with no `unreviewed` and no
`silent`. §4's sweeps before and after, against a pristine checkout at the base commit with its own
build directory, closed with it — no defect count moved in any of the thirteen; the differences are
this round's own added prose. §5 is not owed: not a fifth round, and nothing was measured.

**One instrument artefact came out of taking the baseline, and it is a habit rather than a
finding**, so it is written in `doc/habits.md`'s *Measuring* and only pointed at here: a checkout
at the base commit does not carry the machine-local `doc/*.pdf`, and `--bin pointers` moved 113
pointers between *live* and *not carried* on those alone.
