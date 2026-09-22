# ADR 1262 — An erratum's amended sentence cannot be quoted, so the retired one is marked

## Status

Accepted, 2026-09-22. Binds every place that cites Errata Collection 3 beside a quotation.

`§N` is ISO 32000-2 and nothing else.

## Context

`spec-errata applied` puts on its read-first list every quotation that lands on text Errata
Collection 3 struck and carries no mark of a correction. Six such places were in `crates/`:
`pdf-model/src/appearance.rs` (#287), `attachment.rs` (#374), `content/image.rs` (#79),
`optional_content.rs` (#225), `tests/missing_resources.rs` (#128) and `pdf-syntax/src/write.rs`
(#101).

The obvious repair — replace the quotation with the erratum's amended sentence — cannot be made,
and the reason is structural rather than a matter of effort. `doc/md/ISO_32000-2_sponsored_EC3.md`
is a conversion of the sponsored PDF, and that PDF records Errata Collection 3 as **annotations**:
`StrikeOut` and `Caret` objects lying over the 2020 text, which the conversion drops (ADR 0252).
So the file the conformance gate checks every quotation against holds the *retired* wording and
not the amended one. `doc/md/processed/` is the same conversion and carries neither. A quotation
of an amended sentence would therefore fail `every_quotation_is_the_standards_own_words`, and the
only way to pass it is to quote what the erratum struck.

## Decision

**Where a site follows an erratum's amended reading, it quotes the 2020 sentence and says in the
same breath that the collection retired it.** The quotation stays verbatim — that is what the gate
checks and what a reader needs in order to find the clause — and the retirement is stated in words
`spec-errata applied`'s `HISTORY` recognises, so the sweep marks the place as a correction rather
than as a defect. A caret's own words, which no text on this disk carries, are written as prose:
italics or plain, never between quotation marks.

Where the caret's words happen to coincide with words the struck passage also contained — the
amended §8.9.5.4 step c) keeps "the PDF is being printed" — the site says which of the two it is
quoting, because a reader cannot tell and neither can the sweep.

## Consequences

The read-first list falls from fourteen to eight, and the eight left are records (`doc/adr/`) and
ledger rows this round did not own.

**A round must not read the contract's "rewrite the quotation as the amended text" as available.**
It becomes available the day this tree holds an edition with the annotations applied, and that is
a change to `doc/md/` and to ADR 0252's position, not a change to a comment.

The instrument's blind spot is worth naming as such: a place is marked by prose within
`HISTORY_WINDOW` characters of the quotation, so a correction whose explanation sits further down
the doc comment reads exactly like a place that never heard of the erratum. `write.rs`'s
`startxref` was that shape — its erratum paragraph was correct and out of range — and moving the
paragraph up is the fix, not weakening the window.

Extends ADRs 0252, 0426 and 0440. Supersedes nothing.
