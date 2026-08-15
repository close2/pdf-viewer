# ADR 0380 — The ninth sweep becomes a program, and the table it had been reading short

Status: accepted, 2026-08-15.

## Context

`doc/todo/01`'s ninth sweep asks the one question none of the others do: **does the table a
sentence cites state the key it attributes to it?** `tools/conformance`'s gate checks that a cited
table *exists* and prints its title, and a number that exists and names the wrong table reads
exactly like a right one — which is why this sweep has paid on four of its five runs, and why its
findings arrive in **blocks**: a run of consecutive rows, or one `enum`'s doc comments, written in
one sitting against ISO 32000-1's numbering.

It was a description, and every round that ran it rebuilt the instrument from the paragraph. The
consequences are in the records: "409 headings parsed, ~1000 citations checked, 105 suspects", then
"1024 citations, 61 suspects", then "1104, 80", then "1074 *attributed* citations, 7 defects" — four
different populations under one name, so no run could be compared with the one before it. Each
rebuild also re-met the same instrument shapes, wrote them down, and lost them again.

## Decision 1 — `cargo run --release -p conformance --bin tables`

**Taken.** `conformance::tables`, the ninth of the fifteen sweeps to be a committed program, beside
`entries`, `quotations`, `unread`, `blockers`, `capabilities`, `retired`, `callers` and `pointers`.
It reads `ledger.toml`'s notes, every `//` comment under `crates/`, `tools/` and `fuzz/`, and every
Markdown document under `doc/` bar `doc/history/` — the same three populations `retired` and
`pointers` read, for the same reason: a wrong number is wrong wherever it is written.

**What makes a key a claim about a table** is the whole of the sweep's precision, and the program
states it rather than a round restating it:

- a **possessive** (`Table 191's /H`) reaching [`WINDOW`] words, which is three;
- an **apposition** (`Table 385, with a /Subtype`), the same reach;
- one of twelve **verbs** (`Table 124 defines /FontFile2`) reaching [`VERB_WINDOW`], which is six,
  because a verb introduces a predicate and a possessive introduces a noun phrase;

and then the reach ends at another table, a bracket, a parenthesis or a clause break, and a key
adjacent to another key is that key's **value** rather than a second entry. Both windows were
measured against the corpus of this project's own prose rather than chosen: at six, "Table 349,
whole, from the trailer's `/Info`" and "Table 163's beads walked from `/F`" are attributions, and
both name a key of the dictionary the sentence has moved on to.

**Two judgements move into the program**, and they are what every hand-run redid:

- **which table does state the key**, printed under each suspect — the sentence a correction is
  written from, and the discriminator that ranks the list, because every defect this sweep has
  found had exactly one other table stating the key;
- **whether the sentence is a correction or a standing claim** (`retired::kind_of`), because this
  project writes the number it retired into the sentence that retires it, so its own records would
  otherwise read as hits for ever.

**And a denial is read as a claim in the other direction.** "Table 119 gives a Type 0 dictionary no
`/FontDescriptor`" is this project reading a table correctly and saying so; a sweep that reported it
would print the tree's most careful sentences back at it every round. So a negation before the key
flips the judgement: the standard agreeing is nothing, and the standard *contradicting* a denial is
the same defect from the far end. Its own noise shape is that a negation can attach to another noun
("Table 31 makes a page stating no `/Contents` an empty page"), and all three hits on the first run
were that.

## Decision 2 — the table parser reads a caption's blocks as one table

**Taken**, in `conformance::entries::tables_in`, which the fifteenth sweep shares.

The conversion breaks a table longer than a page into a run of pipe tables, each repeating its
`| Key |` header, separated by whatever the page break carried — a blank line, a running footer, or
a base64 image. The parser filed the **first block and stopped**, so Table 31 stated six of its
twenty-eight entries, Table 124 one of three and Table 125 none of the three the row above it names.
Every citation of one of the others was a suspect, which is how the first run produced 382 of them
and why the earlier hand-runs recorded "the parser missing a split-header table's keys" as noise
they could not remove. A caption's span now runs to the next caption, and each block inside it is
asked for itself whether its first column is `Key` — which keeps Table 92's abbreviations and Table
104's rendering modes out, because those blocks say `Full Name` and `Mode`.

**And `key_of` stripped trailing `1`, `2` and `3` as footnote markers**, so the standard's `/BG2`,
`/UCR2`, `/TR2`, `/Length1`, `/Length2`, `/Length3`, `/FontFile2`, `/FontFile3`, `/DW2`, `/W2`,
`/UR3`, `/MD5`, `/BlackIs1` and Annex-table `/A1` … `/N2` were keys of nothing. Not one table in the
standard carries a numeric footnote marker in its first column — checked, all 305 keyed tables —
so the stripping was pure loss.

**This is a level change in the fifteenth sweep as well, and it is a finding rather than a
regression**: the entries sweep's population goes from 640 stated entries to **756**, and its
reading list from 112 to 174. It had been reading 116 of the standard's entries as though they did
not exist.

## Consequences

**Two defects in the source and nine in this project's documents**, all of them the block shape.

`damaged_stream_census.rs` attributed `/ShadingType` to Table 78 — the five-hundred-and-thirty-
seventh corrected four numbers in that file and this was the fifth, one function away — and
`collection.rs` said "Table 159 makes `/Folders` an indirect reference", which Table **153** does,
the folder dictionary being what `/Folders` points *at*.

The nine are one sentence each, and **six of them are a round's own correction that stopped at the
code**:

| document | said | ISO 32000-2 | the round that retired it |
|---|---|---|---|
| ADR 0366 | Table 122's `/Encoding`, Table 78's `/ShadingType` | **119**, **77** | 537, in `damaged_stream_census.rs` |
| ADR 0123, twice | Table 192's `/H` | **191**'s; 192 is the `/MK` dictionary | 489, in §12.5.6.19's row |
| ADR 0244 | Table 189's `/R` for a widget | **192**'s | ADR 0245, which *named this ADR* as carrying it |
| ADRs 0032, 0045, 0118, 0211, 0323, `doc/ui-boundary.md` | Table 122's `/Ascent`, `/Descent`, `/FontName`, `/DW2` | **120**'s, and `/DW2` is **115**'s | ADR 0216, in nine comments and rows |
| ADR 0200 | Table 158's `/I` title of a thread | **162**'s | never — 158 is the collection split dictionary |

**A number a round retires in the code goes on living in the document the code came from**, and
ADR 0245 is the sharpest instance this project has: it found "Table 189's `/R`", wrote down that
`viewer-gtk`, ADR 0244 and `doc/todo/37` all carried it, corrected two of the three, and left the
ADR. That is ADR 0265's rule — the round that disproves a claim amends the ADR that made it — and
it is the second time a round's own record has named the place it did not sweep (ADR 0352 found
the first).

**Why it is not a gate.** ADR 0249's ratio argument. Seventy-five suspects survive on a clean tree
and three noise shapes account for nearly all of them: a table's *value* named beside its entry
("Table 169's cloudy `/BE`"), a rule a table states *about* a key it does not state ("Table 177
makes the file's own `/AP` decisive over its `/DA`" — which is §12.5.6.6's own `/DA` row, and the
form the five-hundred-and-twenty-fifth session corrected a defect *into*), and this project's own
records of every number it has ever retired. What decides a hit is a question no program can ask.

Tests 2013 → 2025.
