# 2138 — A program written whole again, and a CID a map stream makes present

Session 2138, an archive round in the `lane3` worktree, on round 2134's leftovers and R08's next
cause. ADRs 3253 to 3256; ledger rows 14.3.3, 9.7.4.2 and 9.9 (`implemented`, before and after;
each gains its code or test); no question.

**The XMP writer (ADRs 3253, 3254).** `supplement`, `restate` and `record` refuse a value XML
1.0's `Char` production excludes, by the property's name (`WriteError::NotXmlText`); the predicate
is public (`xml_admits`) and the archive's information rows ask it rather than a copy. A carriage
return is written as `&#xD;`, which section 2.11's end-of-line handling leaves, so a Table 349
value and a carried one holding it come back unchanged; the carried rows no longer refuse one.
Tests apply the end-of-line handling to the written bytes by hand before reading them.

**The `/CIDSet` under a map stream (ADR 3255).** Table 115 gives every CID the stream reaches a
glyph index and sets none apart, and §9.7.6.3's one absence under a stream is a CID it does not
reach; so a CID is present where the stream gives it an index the program holds, glyph 0
included. `pdf-archive`'s 6.2.11.4.2 check judges a `CIDFontType2`'s set by it. veraPDF 1.30.3's
source agrees but skips CID 0, and also fails a set marking a CID the program lacks.

**R08: a program whose stream stops short (ADR 3256).** `F001u_3_7j`'s `/FontFile2` decodes all
28 976 bytes its `/Length1` states and stops in its Adler-32. Where the decode is exactly Table
125's stated extent and this reader loads the font, the bytes are written again whole at the
program's own object; a `/FontFile3` or a length not reached keeps the refusal. A preparation that
substituted nothing no longer needs a history entry. The file converts at 2b and 4 under both
shipped profiles, veraPDF passes all four, page 1 at 150 dpi differs in 15 bytes by at most 1.
The fixtures' outputs (the cut program at four, the carriage returns at part 4) pass veraPDF.

**Census.** With the pack, corpora converts one more on each of the four lines; nothing moved the
other way. Sections 1, 2 and 9 (section 9 was empty), the R08 paragraph, `rank.py`'s R08 status,
three mitigation entries.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform`: fmt 0; clippy `-D warnings
--all-targets` 0 (after `assert_eq!` for two emptiness checks and a method path for a closure);
`pdf-sandbox` build 0; nextest `--lib --bins --tests` 0, 4515 passed, none expected; rustdoc 0;
`fuzz` `cargo check --bins` 0 (`WriteError` gained `NotXmlText`); conformance 0 (after a
blockquote of Table 115 became prose); both censuses 0.

**Next.** R08 and R03 now stop the same two `as-if-printed` documents: `219789` (a page too large
to photograph within the pixel budget) and `427669` (a bound stopped the drawing); `507676` has
four other stops.
