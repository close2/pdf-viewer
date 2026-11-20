# 3256 — A font program whose stream stops in its framing is written whole again

Session 2138, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R08 entry "a program whose stream stops short" (PDFBox
`F001u_3_7j`). Context: ADR 2619, ADR 2636 decision 3 (which named this remedy and refused it);
`crates/pdf-transform/src/archive/fonts.rs` (`written_whole`, `Shortfall::WrittenWhole`),
`crates/pdf-transform/src/archive/prepare.rs`. Clauses: ISO 32000-2 §9.9 and Table 125, §7.4.4;
ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1.

## Context

`F001u_3_7j`'s `/FontFile2` decodes all 28 976 bytes its `/Length1` states, and its `/Length` ends
two bytes into RFC 1950's Adler-32. The decode stops short, so §9.9 does not count the program as
embedded (ADR 2619), yet this reader loads and draws it, so ADR 2636 would not drop it.

## Decision

1. Where the decode stops short (`Damage::stops_short`), this reader loads the font, and the
   decoded bytes are exactly the extent Table 125 states — `/Length1` for a `/FontFile2`, the sum
   of `/Length1` to `/Length3` for a `/FontFile` — those bytes are written again at the program's
   own object as one `FlateDecode`, the producer's dictionary kept but for `/Filter`,
   `/DecodeParms` and `/Length`. They are the program the producer described, whole, and what the
   window drew: no mark is invented, no shape changes, and no loss word is needed.
2. A `/FontFile3` states no extent, and a decode stopping at a length no entry states may stop
   inside the program; both keep the refusal, whose sentence now says so.
3. A preparation that substituted nothing has no history entry to record, so the rule that a
   substitution be recorded in `xmpMM:History` no longer refuses it.

## Result

`F001u_3_7j` converts at 2b and 4 under both shipped profiles; veraPDF 1.30.3 passes all four
outputs; page 1 at 150 dpi differs from the source's in 15 bytes by at most 1.
