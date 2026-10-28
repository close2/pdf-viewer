# 2526 — A `/Prev` that names no section leaves the numbers below `/Size` to their own headers, and the older sections' object streams to theirs

Status: implemented. Session 1846, batch one hundred and twenty-five, the parsers slot. Context: ISO 32000-2
§7.5.5 Table 15 (`/Size`, `/Prev`), Annex F §F.3.3, §F.3.4, §F.3.11, §7.5.7, §7.3.10; ADR 0789 (a declared
entry that was never read is not a deletion), ADR 2502 decision 7 and ADR 2503 (`027613` refused for stating
429 pages where one was drawn). ADR 2527 is the second half of the slot.

## 1. The premise, and what held of it

The brief's hypothesis was that the walk takes the first-page section as the whole table. It does, in effect,
and the cause is narrower. `027613` is linearized (`/N 429`, §F.3.3 Table F.1). Its first-page section stands
at 173 and is read; that section's trailer names `/Prev 1569328`; the main table's `xref` keyword stands at
1569316, twelve bytes earlier; the file is twelve bytes shorter than its own `/L` says. `read_from_startxref`
ended the walk at the first offset that began no section and returned the entries read so far, which are the
260 of the first page. No `/Prev` was "not followed": it was followed, to bytes that are no section, and the
walk stopped there without saying anything. The main table, had the offset been right, would not have been
trustworthy either: its offsets drift from the objects by up to twelve bytes (object 94 is one byte off,
object 1413 twelve), which `load_by_header` already repairs per object.

The same cause stands behind 21 other documents of the 65 944 in the SafeDocs cache and 4 other documents of
the corpora; ADR section 5 has the pages.

## 2. What the clauses say

Table 15 gives `/Size` as "[t]he total number of entries in the PDF file's cross -reference table, as defined
by the combination of the original section and all update sections", so the newest trailer states how many
object numbers the whole chain describes. Annex F says the same of a linearized file, where the first-page
trailer is the newest one and its `/Prev` the only road to the main table:

> The Size value shall be the combined number of entries in both the first-page cross-reference table and the
> main cross-reference table.

and §F.3.11 puts every object outside the first page in that main table. So when a `/Prev` leads to bytes that
are no section, the file has still stated, by `/Size`, that entries exist for the numbers below it that no
section read describes. That is the shape ADR 0789 already handles for a subsection header whose entries did
not read: declared, not described, and therefore not a deletion; the object's own header (§7.3.10) is the
file's remaining statement and is taken. Nothing in the standard says what a reader does with a `/Prev` that
names no section: Annex C.4 advises on a damaged table only as "may attempt to rebuild the table by scanning
all the objects in the file", which is the whole-table recovery this construction is smaller than.

## 3. The construction

1. **The walk records that it stopped on a section it was told of and did not read**: a `/Prev` that is no
   integer, one past the end of the file, one whose bytes begin no section, or the section cap
   (`MAX_XREF_SECTIONS`) reached. A cycle is not that (every section on it was read), and `startxref` itself is
   not a `/Prev`; a table with nothing read is the caller's to rebuild as before.
2. **`declare_unreached` pushes `(0, Size)` onto the table's declared-and-unread ranges**, `Size` being the
   merged trailer's, the newest. One range, never one entry per number, so a `/Size` of 4 294 967 295 allocates
   nothing (a test holds it). A trailer without a usable `/Size` states no range and nothing is invented.
3. **A number a section that was read describes is never asked**, so §7.5.6's deletions stay deletions
   (`XrefTable::location` answers first; a free entry is an entry). The cost, stated: a number only the unread
   section freed is now read from its own header if the bytes are still in the file, which nothing in the
   bytes distinguishes from an object the unread section never described.
4. **The older sections' object streams are found by their own headers.** `recover_compressed_objects`, which
   entered §7.5.7's compressed objects into a scanned table, now also runs for a table whose chain was cut:
   the streams come from the one memoised scan `load_by_header` already makes, a stream is located by its
   header where the table has no entry, a member is entered only where the number is declared and unread (item 2), so an
   object stream cannot place a number beyond `/Size`, and a number the scan also finds at the outermost level
   is left to that object (the sentence of §7.5.7 that `XrefTable::enter_compressed` quotes) and counted in
   `CompressedRecovery::already_at_top_level`. The decode budget of the scanned case is the bound; nothing else is
   read eagerly.
5. **The reading is reported, not hidden.** `Document::cross_reference_chain_was_cut` says it; the viewer
   states it in its notes; the corpus classes count the document as `damaged`. This keeps `117202521857413588359819480739728143692.pdf`
   (`PDF-HUL-86`) in the class and the note it had, because its catalog now resolves and it is no longer
   rebuilt by a scan, which is what `was_recovered` means and still means.

## 4. What was not done

No search near the wrong `/Prev` for the section it meant (the standard defines no such rule and a window is
a number made up), no use of the linearization parameter dictionary's `/T` (a second statement of the same
offset, wrong by the same amount in the files read), and no hint-table reading. The incremental writer is not
changed: it still refuses a document `was_recovered` says was rebuilt, and appends to one whose chain was cut
as it always appended to one whose chain was read to a stop.

## 5. Evidence

The pages `Pages::new(&document).len()` counts, HEAD against this change, for every document of the corpora
and of the whole SafeDocs cache (65 944 files) whose walk reached the new range. Every document the change
reaches now counts the pages its own `/N` or `/Count` states; none of the 974 pdf.js corpus documents is among
them, so the six corpus arms, compared against HEAD's six arms, have none of them to differ on.

| document | pages at HEAD | pages now | what the file states |
|---|---|---|---|
| `027613` (the corpora) | 1 | 429 | `/N 429` |
| SafeDocs `0300856` | 1 | 60 | its `/Count` |
| SafeDocs `3498826` | 1 | 60 | its `/Count` |
| SafeDocs `3006777` | 1 | 56 | its `/Count` |
| SafeDocs `4605705` | 1 | 56 | its `/Count` |
| SafeDocs `2391853` | 1 | 14 | its `/Count` |
| SafeDocs `4728473` | 1 | 14 | its `/Count` |
| SafeDocs `6573176` | 1 | 6 | its `/Count` |
| SafeDocs `1284945` | 1 | 2 | its `/Count` |
| SafeDocs `4100899` | 1 | 2 | its `/Count` |

The other four documents of the corpora that reach the range, `PDF-HUL-86` among them, and thirteen more of the
SafeDocs cache (twenty-two reach it in all) keep their page counts: the objects their cut chain hid were not
pages. `PDF-HUL-86` changes in one other way: it was rebuilt by a scan before (`was_recovered` true), its
catalog now resolves from the cut chain and it is not (`was_recovered` false), with the same fifteen pages and
the `damaged` class kept by `chain_cut`.

Memory under a hostile input: a file of 20 000 object streams, `/Size 4294967295` and a cut chain opens with a
peak of 0.01 GiB in one second. The recovery is one pass of `object_headers()` and the decode budget
(`RECOVERY_DECODE_BUDGET`) the scan-recovered tables already have, so neither the file's `/Size` nor the
number of streams sets what is held.

The new tests in `crates/pdf-syntax/tests/cross_references.rs` were planted: each fails with the line of
the construction it tests taken out.

Two further tests pin the construction on a linearized file. `the_pages_of_a_linearized_file_are_reached_through_a_main_section_its_previous_misses`
builds Annex F's layout (parameter dictionary, first-page section after it with a `/Prev` that is right, seven bytes
into the main table, or twelve bytes before it, the pages' tree and second page only in the main section near the end)
and `a_linearized_file_whose_previous_names_no_section_draws_all_the_pages_it_states` pins `027613` at its `/N` of 429.

## 6. The signed revision's comparison

`pdf_signature::revision` compares the revision a signature covers with the current document under §12.8.2.2.2.
It turned away a document whose table was rebuilt by a scan (`was_recovered`), because comparing two tables where
one is this reader's guess compares a file's word with a guess. A document with a cut chain is not
`was_recovered`, so it reached the comparison, and it is the same kind of table: the numbers of the older sections
are placed by headers, and a number freed only by an unread section comes back (item 3 above). Both the signed
prefix and the current document read alike, so an unchanged object compares equal, but an object freed in an
unread section and reused by a later update would be classed as changed rather than added, and the class is what
a modification-detection permission ranks. The comparison therefore refuses a cut chain on either side with the
same variant (`NotComparable::RecoveredByScan`, its text now saying "recovered, by scanning or past a cut
chain"): a document that cannot be compared is reported as such and never as unchanged.
