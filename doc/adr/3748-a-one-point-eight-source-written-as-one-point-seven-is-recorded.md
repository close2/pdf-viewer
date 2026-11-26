# 3748 — A `%PDF-1.8` or `1.9` source written as 1.7 is recorded, not refused

Session 2261. Status: **accepted and built** in `pdf-transform` (`archive::version_for`,
`archive/header.rs`'s `history`). Answers audit 2260's D04. Clauses: ISO 32000-2 Annex I.2, §7.5.2;
ISO 19005-2 section 6.1.2.

## Decision

Annex I.2: "a PDF file's version should never be changed to an older version". §7.5.2 defines 1.0
to 1.7 and 2.0, and ISO 19005-2 section 6.1.2 admits a `%PDF-1.n` header no later than 1.7, so a
source stating 1.8 or 1.9 — a version no clause defines — was written at PDF/A-2 as 1.7 with no word.
It keeps converting, because a part 2 output has no other header to write and the *should* does not
outweigh losing the document for a digit no clause gives a meaning to; the lowering is recorded as
ADR 2692 records a 3.x source's: the source's first line and the reason go into the output's own
`xmpMM:History` and the report. At part 4 such a source is raised to 2.0, as every 1.n is, which
Annex I.2 does not speak against.
