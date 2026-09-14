# 1058 — What "encrypted beyond us" and "pageless" actually are

The corpus slot. The oracle's undiagnosed head was empty a fifth time (1957 pages, no rows), so the
work was the two populations `tests/corpus.rs` counts and nobody had opened: one document *encrypted
beyond us* and five *pageless*.

**`PDFBOX-4352-0.pdf` is not an encryption gap.** Its `/Encrypt` is `/V 5 /R 6` over `AESV3`,
implemented since ADR 0031; one byte of `6 0 obj` reads `E<` where §7.3.7 puts `<<`, so the object
does not parse and §7.3.10 makes the reference null. Restoring that byte opens it on §7.6.4.1's
default user password and page one interprets to nine commands — the control that makes the refusal
a measurement (trap 13), both halves now in
`an_encrypt_entry_naming_an_unparseable_object_is_refused_by_name`. **The file broke it**: no clause
says how to read a dictionary whose opening token is gone. `poppler`, `ghostscript` and `hayro`
refuse; `mupdf` 1.28 ignores the entry and writes a blank page out of ciphertext, which is what
§7.6.2's second half does not license. **The refusal cited §7.6.1, and so did the ledger row, for a
sentence that is §7.6.2's** — §7.6.1 is one line and says nothing about the entry. Both corrected.

**The five pageless: four *the file broke it*, one silence.** `REDHAT-1531897-0.pdf` (truncated at
871 of its `/L`'s 7945) and `bug1020226.pdf` (184 bytes, no `xref`) are refused by all four
references. `poppler-937-0-fuzzed.pdf` is ADR 0305's NUL-for-`[` *and* an object 3 whose
`/MediaBox` closes with a SEMICOLON, so the entry resolves to null. `poppler-85140-0.pdf` declares
its one page as `3 18446744073709551616 obj` — a generation outside any representation §7.3.3
permits a reader to have — so `3 0 R` is §7.3.10's null object; three references refuse, and `hayro`
draws 595 × 65535 with 1785 black pixels, which is evidence about `hayro`.

**`Brotli-Prototype-FileA.pdf` is the finding.** Its entry said `poppler` names the unknown filter,
which read as agreement; `mupdf` 1.28 and `ghostscript` 10.07 now decode `/BrotliDecode` and draw
the page **in full** (1224 × 792, looked at — trap 1). Nothing changes: ISO 32000-2 defines no such
filter, so there is no reading for their agreement to raise confidence in. **An agreement in a
comment measures the references installed the day it was written.**

**The instrument that was missing was a sentence.** The gate printed `unusable` over the pageless
documents *and* the unopenable ones. `why_no_page_one` asks §7.5.5's `/Root`, Table 28's `/Pages`,
Table 30's `/Kids`, §7.3.10's null, then the walk, and prints which clause each document stopped at.
All six answers are planted in `the_page_tree_diagnosis_names_each_clause_it_can_stop_at`
(trap 13), and planting them corrected three readings of this reader's own recoveries: a lost
`/Root` over a `/Type /Catalog` object is rebuilt, a `/Kids` that is one reference to a page object
is scanned back, and §7.7.3.3 makes a child declaring no node a *page*. **A classifier written by
reading the code cannot see what the code recovers.** Neither bound moved; both are exact. No pixel
moved — `raster_golden` held 974, moved 0, the oracle unchanged at 991/62/835.
