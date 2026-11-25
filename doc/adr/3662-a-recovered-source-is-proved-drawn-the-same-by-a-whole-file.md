# 3662 — A source whose table was rebuilt is proved drawn the same by a whole file of its objects

Session 2240. Status: **accepted and built**; amends ADR 3558 item 1's construction of the second
file. Context: `T02-05-01_001`, `_003`, `_008` and `_012` (the format corpus's hand-built set), each
with a wrong `/Length` and a cross-reference table this reader rebuilt by scanning. Clauses: ISO
32000-2 §7.5.4, §7.5.6.

## Context

ADR 3558's proof appends the rewritten streams to the source as §7.5.6's incremental update and
draws both files. `pdf_syntax::write::incremental_update` refuses a recovered source
(`UpdateError::Recovered`): an update's `/Prev` "shall" name the section before it, and a table
rebuilt by scanning has none. So the proof answered *not the same* for every recovered file, and
`as-shown` refused `T02-05-01_008` and `_012` by `model/operands-left-over` — a class ADR 3641 had
built for them — though nothing on their pages would have changed. ADR 3641's consequence blamed
§9.4.1 for it; the survey of those two files names no such class.

## Decision

For a recovered source the proof's second file is written **whole**: every object the table holds,
as this reader read it, under its own identifier, each rewritten stream in its object's place, each
stream with the `/Length` of the data read, a classic table of one-entry subsections, and a trailer
stating the source's `/Root` and `/Info`. It is the proof's file and never an output, so it carries
no `/ID` and states version 2.0. An encrypted source is not written this way (its objects would go
out decrypted under no `/Encrypt`), and past 256 MiB the proof is not made; both leave the operators.

## Why this is sound

Each object of the second file is the object this reader read from the first; only the rewritten
streams differ, so a page that draws differently does so by the rewrite — or by a difference the
rewrite of the whole file made, which fails the proof and leaves the operators. It errs only
towards refusing, as the appended update does. The conversion's own output is written whole too.

## Consequences

`T02-05-01_008` and `_012` convert under `as-shown` at `content-as-drawn`, their leftovers written
out alone (ADR 3641); every other recovered file's operator classes are now proved, not refused.
