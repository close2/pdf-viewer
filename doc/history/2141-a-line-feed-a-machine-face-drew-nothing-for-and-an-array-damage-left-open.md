# 2141 — A line feed a machine's face drew nothing for, and an array damage left open

Session 2141, an archive round in the `gwg` worktree, on round 2138's next R08 and R03 rows.
ADRs 3265 to 3267; ledger rows 9.8.3.1, 7.3.6 and 9.6.5.1 (`implemented`, before and after; each
gains its code or test); no question.

**`219789` (ADR 3266).** Printed at the refusal: one font, `TimesNewRomanPS-BoldMT`, drawn from a
machine's face, was deferred to the photograph on all 54 pages, so 54 letter pages at 300 dpi
passed ADR 2517's 2^28 pixels. Its proof failed on code 10, a line feed `/WinAnsiEncoding` names
nothing for: the converted CFF face reached the empty glyph `super::empty` gave it, and the
machine-face arm of `same_glyph` asked for the shipped face's own glyph, which the code has none
of. That arm now takes the empty glyph as the other arms do. The pixel budget is unchanged: only
the 19 fenced pages are photographed and 35 keep their text. Converts at part 4; veraPDF 1.30.3
fails 6.2.10.7 on one glyph of page 53, reading a carriage return inside a literal string as code 13
where §7.3.4.2 makes it 0Ah (recorded in `doc/pdfa-blocking-issues.md` section 1).

**`427669` (ADR 3267).** The bound was `MAX_OPERANDS` on page 17: its third stream fails its
Adler-32 and its bytes end inside four `[`, so the next five streams piled up as one array's
operands until the cap. Under the wider gate a page that also names a stream failing its decode
and a keyword inside an array has the cap baked in as the stream's own fault. Converts at part 4,
veraPDF passes; pages 17 and 18 at 60 dpi match the source but for resampling.

**`507676`.** Four other stops (an image or shading not built, three TrueType rows), none answered,
so it is left.

**The `/CIDSet` (ADR 3265).** Section 6.2.11.4.2's check also fails a set marking a CID the program
does not hold, as veraPDF does; the converter's removal answers it as before.

**Fixtures.** `TimesNewRomanPS-BoldMT`, `NimbusRoman-Bold` and `DejaVuSans` (both subtypes), each
showing a line feed, convert at 2b and 4 under `as-if-printed`; veraPDF passes all eight outputs.

**Census.** With the pack, corpora `as-if-printed` part 4 converts 481 (was 479) and refuses 4 (was
6); the other seven lines are unchanged. Sections 1, 2 and 9, the R03 paragraph, `rank.py`'s R03
and R08 statuses, two mitigation entries.

**Gates.** Over `-p pdf-archive -p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0
(after a type name in a doc comment took backticks); `pdf-sandbox` build 0; nextest `--lib --bins
--tests` 0, 1633 passed, none expected in `expected-failures-current.json`; rustdoc 0; conformance
0; the census 0. No public type changed, so no `fuzz` check is owed. The machine-face test fails
with the old arm planted back.
