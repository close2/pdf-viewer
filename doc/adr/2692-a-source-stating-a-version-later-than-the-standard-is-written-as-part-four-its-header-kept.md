# 2692 — A source stating a version later than the standard is written as PDF/A-4, its header kept

Session 1938, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/header.rs`: `later_than_the_standard`,
`history`; `archive/mod.rs`: `version_for`, `LOWERED_TO_PART_TWO`; `archive/prepare.rs`:
`NO_PLACE_TO_RECORD_HEADER`).
Context: ADR 2490 (R06a, "a header no version this tree reads"; R06b, "a part the source cannot
meet"), ADR 2558 (R06a as built), ADR 2498 (`--or-else-to`), ADR 2689 (the `UnknownFilter-*`
streams).
Clauses: ISO 32000-2 §7.5.2 and Annex I; ISO 19005-2 and ISO 19005-4, section 6.1.2.

## The premise

The census refused the nine `UnknownFilter-*` files (`%PDF-3.1` to `%PDF-3.9`) at both parts with
`version_for`'s sentence that "the target's part requires a %PDF-1.n header". At part 4 that was
false: `version_for` compared the stated major with the part's, and every major above the part's
fell to the one refusal written for a 2.x source at part 2. `pdf_syntax::Version::parse` reads
`%PDF-3.1` as 3.1, so the file does state a version this tree reads, which is why ADR 2558's route
(`Document::version` is `None`) never caught it. The reader is right to read the digits: Annex I asks
a processor to warn of "a version number newer than the version that it supports", and
`viewer_core::notes` does, so `pdf-syntax` is not changed.

## The reading

§7.5.2's header is `%PDF-1.n` or `%PDF-2.n`; a 3.n is a version later than any the standard
defines, and §7.5.2 adds that a processor may be able to process a file of a later version.
ISO 19005-4 section 6.1.2 admits `%PDF-2.n` alone, and ISO 19005-2 section 6.1.2 `%PDF-1.0` to
`%PDF-1.7`. Annex I says a writer should never change a file's version to an older one.

## The decision

1. **At part 4 the output's header is 2.0**, ADR 2558's construction: the output is a file this
   program writes, it holds only what this reader of ISO 32000-2 read (a stream it could not
   decode is ADR 2689's named loss, an operator it does not know is the fence's refusal), and
   §7.5.2 has the writer of a file conforming to ISO 32000-2 identify it as 2.0. The source's first
   line and the version it stated go into `xmpMM:History` and the report's `recorded`, a condition
   of the header as in ADR 2558. Annex I's *should* is departed from deliberately: the only way to
   keep 3.n is to write no PDF/A-4, and the record keeps what the version was.
2. **At part 2 it is refused**, as a 2.0 source is (ADR 2490's R06b): a source stating a later
   version states at least what 2.0 added. The sentence now says PDF/A-2 and section 6.1.2, and
   that PDF/A-4 admits the header; it is reachable only at part 2 (`LOWERED_TO_PART_TWO`).
3. A `2.n` past 2.0 keeps the clause's shape and part 4 admits it, so it is unchanged: kept at 4,
   refused at 2.

## Rejected

- **Reading `%PDF-3.n` as no version** (R06a whole): it would write 1.7 at part 2 over a source
  stating more than 2.0, the lowering the part-2 refusal exists for.
- **Writing 2.9**, the nearest admitted version: no standard defines 2.9, and §7.5.2 names 2.0.
