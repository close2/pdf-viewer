# ADR 0089 — A declaration a document can be held to

Status: accepted, 2026-08-01.

## Context

After ADR 0088 the ledger's silence was twenty-seven rows: §12.8's long-term validation, its
document timestamps, its legal attestations — and §12.7.8's FDF, which is a file format.

## Decision

**§12.8.4's document security store and §12.8.5's timestamps are counted; §12.8.7's legal
attestation dictionary is checked.**

The store holds X.509 material — certificates, "an array of all Certificate Revocation Lists
(CRL) (see Internet RFC 5280 )", OCSP responses — and none of it is this program's to interpret.
What a program that cannot validate can still answer is whether **the document carries what a
validator would need**, which is precisely what §12.8.4 exists for, so `SecurityStore` counts each
array and the `/VRI` entries. A document timestamp "is a standard signature dictionary as
described in [Table 255]", so it needs no new reader at all — only its `/Type`, and the byte-range
check ADR 0088 built, which is the one requirement of that family a program without a trust store
can hold a file to.

§12.8.7 is the interesting one. The clause exists because a PDF can mislead:

> The PDF language provides a number of capabilities that can make the rendered appearance of a
> PDF document vary. These capabilities could potentially be used to construct a document that
> misleads the recipient of a document, intentionally or unintentionally.

and it asks an author certifying a document to *declare how many* of those capabilities are in it
— scripts, launch actions, alternate images, external streams, TrueType fonts. **Ten of those
counts are things this reader can count itself.** So `Legal::disagreements` counts them over the
object graph and names every entry where the author's number and the file's contents differ.

That is the project's oldest habit — §12.3.3's `/Count`, an LZW stream's length, §12.4.3's bead
arrays, §14.8.2.5's two orders — applied to the one dictionary in the standard whose whole purpose
is to be checked. It is a *question* rather than a verdict, and the note says why: the clause
states no counting algorithm, so a producer that counts one shared action once where this reader
counts two references is not lying.

## And §12.1, which had been wrong since the fifty-seventh session

The clause-12 aggregate said this tree implements "none of the parts that make a document
interactive". A click has followed links since the fifty-seventh session, seven §12.6.4 actions
are performed, and this run of ten sessions read §12.2, §12.3, §12.4, §12.8, §12.9 and §12.10. It
is `partial`, and what remains silent underneath it is **one file format**.

## Everything re-verified

The numbers in `doc/HANDOVER.md` are worth what their last measurement was worth, and ten sessions
had passed. Re-run this session, not inherited:

- **800 tests**, `clippy` clean under `pedantic`, `cargo fmt --check` clean.
- **`cargo deny`: advisories, bans, licenses, sources — all ok.**
- **All four fuzz targets clean at 50 000 runs apiece.**
- The corpus gate: 974 documents, unchanged. The text gate: unchanged. The oracle: **837 agree, 65
  contradicted**, exactly as at the eighty-ninth session — ten sessions of new readers moved no
  pixel, which is what a specification track that touches no rendering clause should look like.
- Speed against `hayro`, measured this afternoon: **ours 7.08 s over 859 complete pages** against
  49.03 s, median 2.15× — and over all 946 pages, 7.73 s against 110.85 s. The seventy-third
  session's 6.91 s was 858 pages on a different afternoon, so the totals are quotable only
  against each other; the median is where it was.

## Consequences

- `silent` falls 27 → **14**, and every one of the fourteen is §12.7.8's forms data format,
  §12.7.7's named pages that exist to be imported from it, and §12.1's aggregate over them.
- `implemented` is 350, `partial` 224, `reported` 53 of 823 rows.
- Ten sessions took `silent` from 93 to 14 without moving a gate, which is the shape this project
  wanted from a specification track: the corpus cannot rank a requirement no file exercises, and
  now the ledger says which requirements those were.
