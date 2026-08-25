# 755 — The table whose only gate was its own inverse

The errata selection rule's fourth step, written by the round that could not run it, and the head it
produces once it runs. The head is `implemented`, its three errata move nothing, and reading them
found that the row's only gate could not have seen a wrong transcription at all.

Date: 2026-08-25.
ADR: [0671](../adr/0671-the-table-whose-only-gate-was-its-own-inverse.md).

Touched: `crates/pdf-syntax/tests/pdf_doc_encoding.rs` (new), `crates/pdf-syntax/src/text_string.rs`
(one doc comment), `doc/conformance/ledger.toml` (§D, §D.3), `doc/errata-read.md`, `doc/todo/01`,
the ADR and this file. **No pixel moves and no behaviour moves**: the transcription was right at all
256 codes, which is what the round set out to check rather than what it assumed.

## The rule, as it now reads

Both of 750's repairs are in `doc/todo/01`'s recipe, where a round running the rule reads them:

- **Step 4 ranks over *every* row, not a second list read after the first**, and the ordering is
  argued rather than stated. A live row's count ranks a debt the ledger already declares; a settled
  row's count ranks a *claim*, and `CLAUDE.md` says a claim decays — its own §10.5 entry being the
  standing example of an `inapplicable` that was wrong. So: one ranking over every row, the settled
  row winning a tie, because ranking the settled rows above the live ones outright would throw away
  the count and the count is the whole instrument.
- **Step 2 gains a writing rule rather than a third grep.** An issue number written outside
  `doc/errata-read.md` carries the `Issue #` prefix; the bolded bare `**#214**` is invisible to both
  greps and a bare-number search collides with `doc/HAYRO_ISSUES.md`. An erratum read only far
  enough to break a tie is left in the population on purpose — which is why the bare numbers 746
  wrote for #357 and #224 were **not** prefixed this round: those two are still what §14.8.5.3's
  rank rests on.

## What the corrected rule gave

302 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret; **126 are named
nowhere in this tree**. Over live rows the head is §14.8.5.3 with seven annotations — the plateau
750 left standing, §7.7.4 having gone off the ranking because 750 read it. Over every row **§D.3
carries fifteen**, more than twice that, and is `implemented`. §9.6.4 and §7.4.1, the two 750
measured from outside at 11 and 8, are second and third and both still unread.

Reproducing 750's two figures exactly is what said the arithmetic was right before it was trusted.

## What the issues said

All fifteen annotations fall under one `emit` heading, so nothing had to be reassembled — the first
use of this rule where that is true, and it is because Annex D.3 is one table on six pages.
`doc/errata-read.md` has all three with the rectangle that places each.

- **#562** strikes `(END OF TEXT)` off code 0x04 — where it is already 0x03's — for *END OF
  TRANSMISSION*, and 0x05's `(END OF TRANSMISSION)` for *ENQUIRY*; and in the `Character` column,
  the `u` and `v` printed for 0x18 and 0x19 where a breve and a caron belong.
- **#285** corrects 0x16's `Unicode` cell from U+0017 — printed twice, on 0x16 and 0x17 — to U+0016,
  and `SYNCRONOUS` to *SYNCHRONOUS*.
- **#461** strikes the `Š` printed in **0x8a**'s `Character` column, whose `Unicode` cell says
  U+2212, and writes `−` and *MINUS*.

**The three glyph corrections are written in `PDFDocEncoding` itself.** Each caret's `/Contents` is
the single byte 0x18, 0x19 or 0x8a, so `spec-errata emit` prints the erratum's own replacement
*through the table it corrects* — `pdf_syntax::text_string` is what decoded them. The annex's
`Unicode` column is the independent side of that circle and is what the new test reads.

## What reading them made this round look at

**§D.3 was `implemented` on a test that is the table's own inverse.** Its cited gate was
`every_text_string_survives_the_round_trip`, and `encode_text_string` searches the array
`text_string` indexes — so a code transcribed with the wrong character round-trips perfectly. 232
mappings had no other assertion than seven spot-checked codes.

**Issue #461 names the exact mistake that gate would have missed.** `Š` is 0x97's character, printed
in 0x8a's row; a transcription taken from the column the erratum corrects would decode a minus sign
as a capital S with caron. Planted, **all ten of the module's tests stay green**, the round trip
included.

`crates/pdf-syntax/tests/pdf_doc_encoding.rs` compares all 256 rows against `doc/md/` in both
directions — decode and encode stated separately, because a character transcribed onto two codes
decodes differently and encodes alike and only the reverse direction sees it. **Every row agrees.**
Calibrated per trap 13 with three plants, each removed: 0x8a as `Š`, which fails both tests; two
`U` codes given the `Unicode` cell the annex prints beside them, which fails the decode test alone
and which **no pre-existing test sees**; and a swap at 0x18/0x19, which the module's own
`the_table_is_not_latin_1` does catch. The parse reads a row's single `0xNN` and single `U+NNNN`
positionally-independently, because the conversion shifts this table's columns from one page block
to the next; the one place needing care is code 0x55, whose *character* is `U` and whose row would
otherwise read as carrying the annex's `U` note.

**And the row named the wrong table for its whole life** — "the fourth column of Table D.2", which
is a glyph-name font encoding in `pdf-font`, for what is Table D.3's code-to-Unicode column. Nothing
could print it: the ninth sweep reads `Table NNN`, and an annex table's number is outside its
population. §D's parent claim that all five tables are "transcribed from `doc/md/` and gated" was an
overstatement of this member's evidence and is true now.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this **is** a fifth round, so §2's sequence ran whole and §5 rebuilt and
installed the seven binaries — which `round.sh` had flagged as absent from `target/` — before it.
Both workers were built before any gate that decodes an image (trap 10).

`fmt`, `clippy -D warnings`, `nextest` (2665 tests, 18 skipped), the doctests, the fuzz `check`, the
sandbox worker, corpus, `pdfref-hayro`, oracle, text extraction (98.26% of matched words in bounds,
486 of 508 documents fully in bounds), selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them re-run after the final
document edit. The only clippy output was `viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines,
which §2 documents as not lints. **The oracle says no pixel moved.**

The lint run failed once and the failure was this round's: the new test file drew five
`clippy::pedantic` errors — a redundant closure, a `panic!` outside an `expect` list, and three
`arithmetic_side_effects` in the byte-offset arithmetic of its parser. The parser splits on the
prefix now and carries no arithmetic at all, and the plants were re-run against the rewritten
version rather than trusted across it.

**The machine was loud when the round opened** — a one-minute load average past 390 on 24 cores with
three sibling rounds running — and had fallen below 30 by the time the gates ran, so the lines that
spawn a reference renderer were not held.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them. The
"before" run was taken by restoring the four edited files and removing the two new ones, since the
round had started editing before it swept. `quoted` and `unpriced` were not run: this round touches
no page-list note and both take the oracle's log as their right-hand side.

`entries`, `unread`, `blockers`, `capabilities`, `callers`, `overstated` and `spec-errata check` and
`moved` are **byte-identical**. **Not one defect bucket moved:**

- `counts` 8171 ← 8117 sentences with 427 ← 426 attributed counts, **149 the family agrees with, 58
  "no such way" and 4 places counting one family twice, all three unchanged**; the one new count is
  attributed to a clause with no rows below it, 220 ← 219.
- `quotations` 6332 ← 6327 document quotations over 978 ← 977 documents with **diverging unchanged
  at 38**, and 1945 ← 1944 ledger quotations with **diverging unchanged at 2**.
- `tables` 6657 ← 6638 sentences with **key citations unchanged at 2470 — absent 100, contradicted
  denials 6, keyless 58**.
- `pointers` 8434 ← 8403 with **absent unchanged at 131** and 140 ← 138 symbol pointers, the two
  being the ledger row's new test names, with **13 undefined unchanged**.
- `owed` **182 unnamed terms over 113 rows, unchanged**; `inapplicable` unchanged at 55 / 233 / 224;
  `overtaken` 577 ← 576 decision records with **48 overtaken unchanged**.
- `spec-errata applied` grew to 751 ← 734 places naming an erratum over 56 772 ← 56 715 places read,
  with **the read-first list unchanged at 10, the corrections quoting retired wording at 90 and the
  places inside `errata-read.md` at 72**.
