# 2901 — PDF417 is built from ISO/IEC 15438:2006, and its matrix is the template's

Session 2042. Status: **accepted**, carrying out `doc/questions/A857` item 1. Clauses: ISO 32000-2 §K.2; XFA 3.3
pages 424–439, 442 and 611–620; ISO/IEC 15438:2006 clauses 5.2 to 5.11 and Annexes A, C, D, E, F, G and Q.

## The source

**ISO/IEC 15438:2006** as Public.Resource.Org publishes it (`law.resource.org/pub/us/cfr/ibr/inc/iec/
iec.15438.2006.pdf`, SHA-256 `5aa7d91c…a470d5f`, 112 pages), incorporated by reference into US federal regulation.
A857 has the symbology built from it, kept outside the tree, its tables carried as data and its text cited by
clause and paraphrased — the code quotes no sentence of it. Unlike Data Matrix (ADR 2900), this is the standard
itself, so every behaviour is derived from the text and the decoder is evidence only.

What the tree carries of it is **Annex A**, the 929 codewords' bar-space sequences in three clusters, transcribed
from the text layer row by row, and **Table 5**, Text Compaction's four sub-modes, read from the page image (the text
layer omits that page). Annex A is held by a test to clause 5.3.1 itself: eight elements of one to six modules, 17
in all, each entry in the cluster its bars compute, and no entry repeated among the 2 787.

## The construction

- **Compaction.** The data is cut into segments: 13 digits or more Numeric, five characters or more that Text holds
  Text, the rest Byte — Annex N's thresholds, its look-ahead not built. Which compaction carries a byte is the
  encoder's choice; each compaction's rules are clause 5.4's, and Table 6, Annex C and Annex D's worked examples pin
  them. Text Compaction reaches a sub-mode by Table 7's latches and shifts and pads an odd count with a shift or, in
  Punctuation, `al` (clause 5.4.2.4 case a). The byte shift 913 is not used.
- **Error correction.** Clause 5.10's algorithm step by step, its generator computed from the clause's product
  rather than copied from Annex F; Annex F's first three tables and Annex Q's worked example pin it. Absent
  `errorCorrectionLevel`, the level is ISO/IEC 15438 Table E.1's recommendation: XFA 3.3 states no default and the
  standard's own advice is the one choice that is not this program's.
- **The matrix.** XFA 3.3 gives three properties and a fallback: `dataRowCount` with `dataColumnCount` fix it,
  padded (pages 613–614); `rowColumnRatio` lets the rows grow at a ratio (page 615); and absent all three the
  processor picks legal counts that fit the field (page 438). The crate does not know the field, so `Shape` carries
  the three and two more: `Columns`, which the layout will compute from the field's width, and `Square`, used until
  then — the matrix whose rows at clause 5.8.2's least height of three modules come nearest the row's width. Each
  is held to clause 5.2.1's 3 to 90 rows and 1 to 30 columns and clause 5.2.3's 928 symbol characters.
- **Rows.** Each row is drawn from Annex A in its cluster (clause 5.11.1) between its row indicators (clause
  5.11.3), three module rows high (clause 5.8.2's `Y ≥ 3X`); `truncate` drops the right indicator and shortens the
  stop pattern to one module, Annex G's Compact PDF417.
- **Bounded arithmetic**, argued in one `expect` reason and held by the `xfa_barcode` fuzz target (ADR 0096).

## What is left

Macro PDF417 (`pdf417macro`, Annex H) and ECIs (clause 5.5) are not built; `pdf417macro` stays refused by name. The
layout that sizes a symbol to its field and draws it is `doc/todo/78` section 4's.
