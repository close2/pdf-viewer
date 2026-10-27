# 2410 — The seventh sweep reads the tables a clause names, and §13.7.2.3.1 is implemented for its assets

Session 1788, the clause slot of batch 119. Status: accepted; the move is authorised by the batch's
brief under ADR 2370's rule 2, and the instrument answers ADR 2376's named miss. ADR 2411 is unused.

## Context

ADR 2376 widened the seventh sweep (`--bin inapplicable`) to `out-of-scope` rows and gave every
swept row its own clause number as a term. It named what that still missed: §13.2.4.1, §13.2.6.1
and §13.2.7.4.2 were executed by `media.rs` under a **table** — "Table 284's /S", Table 294's `MH`
— or under a neighbour's citation, "§13.2.7.4's software identifier", and so were found by hand. A
clause whose subject is a dictionary is read by the code under that dictionary's table, and an
exclusion's note, written to one template, never says which table that is. Trap 272 forbids keying
the term on the note's words; the clause's own text in `doc/md/` is keyed on the row's number.

## Decision

1. **A swept row states the terms of every table its own clause names** (`tables_named`): the text
   under the row's heading, subclauses excluded, read from the conversion by the row's own number.
   A table is named where that text captions it, cites it by number, or names its caption's subject
   as a dictionary — §13.2.7.4.2's "a software identifier dictionary" reaches Table 303, captioned
   under §13.2.7.4.1. Each table is two terms: `Kind::Table`, `Table 284` as a whole designation, and
   `Kind::Subject`, the caption's subject between its first article and *dictionary* ("Entries in a
   **software identifier** dictionary"), matched in a source's prose with case, line breaks and
   comment markers folded, where the subject has two words or more — one word (`viewport`,
   `rendition`) is the tree's own English, and its table is still asked about by number.
2. **A table number a `32000-1` stands just before is the other edition's** and names nothing:
   `structure.rs`'s "ISO 32000-1's Table 334" is a heading level, this edition's a
   `RichMediaSettings` dictionary.
3. **The calibration is the tree that missed them.** `--bin inapplicable -- --root <tree>` reads a
   past commit's ledger, sources and `doc/md/`; on the commit before ADR 2376's merge, unpacked by
   `git archive`, the sweep lists all three rows by a caption term of reach 1 — `Table 284`,
   `Table 294`, `software identifier`, each named by `media.rs` alone — and with them §13.2.7.2,
   §13.2.7.2.1, §13.2.7.3 and §13.2.7.4.1, the other uncited rows ADR 2376 moved. The planted test
   `a_clause_executed_under_its_tables_number_or_subject_is_listed` is the same three rows.
4. **§13.7.2.3.1 moves from `out-of-scope` to `implemented`** for the one entry the clause 13 entry
   admits, the media file saved: Table 341's /Assets is "[a] name tree of embedded file
   specification dictionaries as detailed in 7.11.3", and `rich_media::assets` walks it into
   attachments (ADR 2399, A631); /Configurations and /Views present the artwork and stay on the
   exclusion. §13.7.2.3's and §13.7.2's notes name the exception; their status does not move.

## Consequences

On HEAD's ledger the sweep reads 90 rows and 32 carry a named table term. The rows it lists by a
term of reach five or less, and why each of the rest stays:

| rows | the table, and its reader | stays because |
|---|---|---|
| §12.6.4.16, §13.6.3.1, §13.6.3.2, §13.6.4.7 | Table 311, `pdf-archive`'s ISO 19005-4 B.2.2 check of a 3D stream's /Subtype | a validator reading a producer's entry is not the processor the clauses address |
| §13.6.6, §13.6.7.4 | Table 173 (XFDF's `<ex_data>`, refused as clause 13's), Table 351 (a correction in `content/reader.rs`) | the reader refuses or only names it |
| §13.6.7.3.6, §13.7.2.2.2, §13.7.2.3.4 | Table 341, `rich_media.rs` | the clauses cite the table for /Views or the instance's /Asset, which no code reads |
| §13.7.2.2.5, §13.7.2.2.6, §K | Table 338, Table 190 | a sentence about ISO 32000-1, and Table 190 cited beside §K for widgets |
| §14.10.1, §14.10.2 | web capture information, `merge.rs`'s /SpiderInfo not carried | the writer follows §3.15's "should not be written"; nothing is read |
| §14.11.2.2 | box colour information, `page.rs`'s page-only entry list | the permission to display guidelines is declined |
| §14.11.4, §14.11.6.3, §10.6.3 | Table 400, Table 126 | **miscitations**: `prepare.rs`'s "§14.11.5's Table 400" means Table 401, and `sfnt.rs`'s "§9.9 Table 126" names spot functions — reported to the files' owners |
| §14.11.7, §14.12.4.2 | `type 1 form` in `loading.rs` (a font), Table 409 (already cited) | noise, and a row ADR 2376's citation already listed |

A table term can find a wrong table number as readily as a reader, which is the ninth sweep's
subject from the other end; two of the list are that. The rows above reach five or less; the rest
are shared tables (Table 5, Table 29, Table 166) under the clauses that cite them in passing,
printed last by reach.
