# 1030 — The sentence that changes font half-way

Slot: corpus. Branch `batch-1026-1031`.
## What was taken

The oracle over the whole corpus, then `pdfref --bin undrawn` over its log. **The gate's
*ambiguous, undiagnosed* ranking prints an empty list**, and all 62 contradicted pages are held by
a `CONTRADICTED_*` group, so the undiagnosed head the slot asks for does not exist. The sweep is
byte-identical to the thousand-and-eighth's: 839 listed, 839 measured, 18 at or past −1.00, 15 of
them reported.

Below the alarm, the two highest rows no group holds and no paragraph of `doc/todo/00` had read are
**`issue6127.pdf` pages 1 and 2**, −0.797 and −0.792. Page 3 of the same document is held by
`AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`; pages 1 and 2 are *reported*, so they sat outside the ratchet
by construction.

## What it is

§9.7.5.2's "The Identity-H and Identity-V CMaps shall not be used with a non-embedded font" — ADR
0433's clause, whose population the corpus gate counts at eighteen documents while that ADR names
eleven. **We are right, and this page is the population's control**: the attestation paragraph runs
in `/C2_2` (`DPOKGC+TimesNewRoman`, `/Identity-H`, descendant with a `/FontFile2`) as far as
*"…Assurance Maladie Maternité et "*, and the next operator selects `/C2_14` — same face, same
`CMap`, no program, no `/ToUnicode` — and continues *"ne pas remplir les…"*. 585 codes go through
the two refused fonts.

The four references are the prohibition's own rationale, and here it is arithmetic: the file's CIDs
are the standard Macintosh glyph ordering (CID = ASCII − 29, `é` at 112), and `poppler` draws the
CID as a character code, `hayro` draws CID + 31, `mupdf` indexes a face in that ordering and is
right, `ghostscript` indexes a substitute of its own and is right on ASCII and wrong on every
accented letter. Inks: ours 7.825, `hayro` 8.622, `mupdf` 9.500, `poppler` 12.053, `ghostscript`
12.141.

## What changed

No rendering code. Two tests in `pdf-model/tests/silent_fonts.rs` — the refusal pair plus the drawn
half of the sentence as the control, and a witness reading both font dictionaries out of the file.
Calibrated (trap 13) by admitting the construction: the report becomes `no outline for any of the
389 code(s)` and the test goes red naming it. Ledger §9.7.5.2 gains the control and the two tests
and stays `implemented`. `doc/todo/00` and ADR 0433 carry the reading.
