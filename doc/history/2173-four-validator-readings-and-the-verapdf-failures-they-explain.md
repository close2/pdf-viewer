# 2173 — Four validator readings, and the veraPDF failures they explain

Session 2173, an archive round in the `lane3` worktree: round 2168's veraPDF failures that are
this tree's, in its order. ADRs 3393 to 3396; no question. Ledger rows 7.3.3, 9.6.2.1, 9.6.3 and
14.6.2 (`implemented` before and after).

**`475121` (ADR 3393).** §7.3.3 makes a digit run an integer at any length; the lexer handed a
34-digit run over as a real, inside 6.1.13's real bound. The lexer now records an `i64` overflow
beside the real (`Lexer::integer_out_of_range`), the reader's `AsWritten` mode (the validator's)
hands the run over, and the survey keeps integer extremes as `i128`. Drawing is unchanged.
`475121` is refused at 2b by the fence; at 4 it converts and passes veraPDF.

**`PDFBOX-4532-reduced` (ADR 3394).** The object is no orphan: `/Properties /MC0` names it and no
content stream does. Section 6.2.2's exemption takes it, its metadata with it, out of 6.6.2.3.1;
the withdrawal stands, the disagreement is veraPDF's, and removing exempt entries is queued.

**`issue1350` (ADR 3395).** New row `fonts/base-font-is-a-name` (Tables 109, 115, 119) and
`Rewrite::BaseFontNamed`: `/BaseFont`, and a `/FontName` that is no name either, written as the
embedded `sfnt` program's name ID 6 in its own alphabet (`AACADT+ArialMT`). Four outputs pass
veraPDF.

**`issue17679_2` (ADR 3396).** `123 /oc1 BDC`: the survey now reads `BDC`'s property list from
the last operand where it is a name after another operand, so the undefined `/oc1` is reported
and defined empty (ADR 2684). Four outputs pass veraPDF.

**`427669`** after ADR 3380: converts at 4 under `as-if-printed`, pages 17 and 18 photographed now
for the 6.2.2 operator row; refused at 2b (6.1.13) and at 4 under `as-shown`; veraPDF passes it.

**Census.** With the pack, corpora 2b one more refused under each profile (448 and 413 converted):
`475121`, now written at 4; pdf.js unchanged; without the pack the same, `issue15053` as before.
veraPDF: 5518 written as conforming, 87 failing outputs of 26 documents over 21 rules (was 95, 29,
24), 20 unfinished; only `475121`, `issue1350` and `issue17679_2` left the list.

**Left, in order.** `T02-04-01_005` (no `/Subtype`, so no row reads the font: a Table 109 row and
a remedy); exempt-entry removal (ADR 3394); `issue7901`, `bug920426`, the malformed CFF reals.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt`: fmt 0;
clippy 0; `pdf-sandbox` build 0; nextest 5066 passed, 5 failed, the manifest's five clause 11
audit tests (difference none); rustdoc 0; `fuzz` check 0; conformance 0 once this record stated
its gates.
