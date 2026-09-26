# 1328 — A hint table's item runs are each padded to a byte

Session 1245. Status: accepted and **built**.
Context: `crates/pdf-syntax/src/linearize.rs` (`hint_data`, `BitWriter::align`, the module's
section on packing), `crates/pdf-transform/tests/support/linearized.rs` (`Bits::align` and every
table decoder), `crates/pdf-transform/tests/linearize.rs`
(`every_item_run_of_a_hint_table_begins_on_a_byte`), the F.4.1 ledger row.
Answers: `doc/questions/Q131` — the owner's word of 2026-09-23 is in `A131`.
Amends: ADR 1293 section 4's first bullet, "F.4.1's packing is the annex's". ADR 1293 is not edited;
this record replaces that bullet and nothing else of it.
Clauses: ISO 32000-2 §F.4.1, Tables F.3 to F.12.

## 1. The reading

§F.4.1: "In general, this byte stream shall be treated as a bit stream, high-order bit first, which
shall then be subdivided into fields of arbitrary width without regard to byte boundaries. However,
each hint table shall begin at a byte boundary."

ADR 1293 read the exception as applying to whole tables only. The owner chose `Q131`'s option 2,
"pad each item-run", after a search that changed the recommendation, and `A131` takes it as the
sentence's **operative reading, not a departure**: aligning whole tables restates what the
dictionary's byte offsets for each table already require and does no work, while padding each run
is what the format's author has written since 1996 and what every reader of the tables known here
expects. So principle 5 is not set aside; the ambiguity is resolved toward the reading that gives a
normative sentence work to do.

**What a run is.** A table's header is one run. Each item of Tables F.4, F.6, F.8 and F.12, taken
across every entry (item 1 for every page, then item 2 for every page) is another, and the run is
followed by zero bits to the next byte. Every header field is 16 or 32 bits wide, so a header ends on
a byte already. Tables F.9 and F.10 have no per-entry items; F.10's identifier list is one run that
ends where the table does.

## 2. Evidence, as evidence

qpdf writes and reads this way, its source commenting that each row must start on a byte and its
manual saying it was tested against Acrobat's and pdlin's files; PDFium's page hint reader
byte-aligns after every run (`A131`'s reading); no reader of whole-table packing is known, and
pdf.js does not read the tables. After the change, `qpdf --check` on the fixtures no longer reports
a shared object length mismatch or a misread identifier. What it still reports is two other
readings: it always takes page 0 as the first page where F.3.7 lets `/OpenAction` choose (ADR 1293
section 6), and it counts a page's objects differently where thumbnails and beads sit in the page's
section. On ten corpus files written with classic tables, nine draw no linearisation warning from
qpdf (`TAMReview.pdf`'s one warning is its source's unsorted name tree), and `issue15590.pdf` is
exit 2 on a `/Pages` naming a page, as its source is. The same ten written with object streams each
draw qpdf's warning that an uncompressed object follows a compressed one in a cross-reference
stream, which is about F.3.1's numbering, not packing, and is left for its own reading.

## 3. Revisit

When an edition or a corrigendum of ISO 32000-2 states the packing either way, this tree follows
the text then. The cost if it states whole tables: a reader packing whole tables would misread these
files; none is known.
