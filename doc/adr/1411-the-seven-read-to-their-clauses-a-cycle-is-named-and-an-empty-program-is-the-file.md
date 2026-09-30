# 1411 — The seven neither-one documents read to their clauses: a cycle is named, a program's own fault is the file's, and an outline-free program is what the file states

Session 1287. Status: accepted and **built**.
Context: ISO 32000-2 §9.6.4 (Errata Collection 3), §9.6.5.4, §9.7.6.3, §9.8.2 Table 121, §9.9
Table 124, §C.2 Table C.1. Code: `crates/pdf-model/src/content/run.rs` (`Interpreter::run`),
`content.rs` (`Interpreter::chain`, the font report in `complete`), `content/reader.rs`
(`NestedContent::identity`), `content/report.rs` (`Unsupported::NestingCycle`), `content/text.rs`
(`Coverage`), `crates/pdf-font/src/sfnt.rs` (`composite_cycle`), `crates/pdf-font/src/loading.rs`
(`holds_no_outline`, `states_both_symbolic_flags`, `composite_cycle`),
`crates/pdf-model/examples/nesting_census.rs`, `crates/pdf-model/tests/corpus.rs` (`INCOMPLETE`).
Builds on ADR 1401 (the population named), ADR 0793 (the bound's value), ADR 0270 / 0520 (the font
report). ADR 0793 is **not** superseded: its refusal stands.

## 1. The four fonts, one by one

Each program was extracted and read with fontTools; each code shown was traced through §9.6.5.4 or
§9.7.6.

| document | what the file states | clause | verdict |
|---|---|---|---|
| `issue17333.pdf` | code 0 via `MacRomanEncoding`, `/Flags 32`; the (1, 0) subtable maps only 165 | §9.6.5.4: "a PDF processor may supply a mapping of its choosing" | **neither one**, stays: no route, and this reader's choice is no glyph (ADR 0520) |
| `recursiveCompositGlyf.pdf` | every code reaches `space`, a composite whose components are glyph 1 and `space` itself; `maxp` is version 0.5 | Table 124: a `/FontFile2` program "shall conform to the TrueType Reference Manual", whose `maxp` states the depth of composite nesting as a finite count | **the file's**: the report now names glyph 2's cycle |
| `issue20232.pdf` | `/Flags 36`; code 71 is `/Ccedilla` by `/Differences`, `G` (empty) through (3, 0) | §9.8.2: "This flag and the Nonsymbolic flag shall not both be set or both be clear" | **the file's**: the report names the contradiction and the reading §9.8.2 prescribes |
| `issue12963.pdf` | a CID-keyed CFF holding CID 0 alone, `endchar` | §9.7.6.3: "the glyph for CID 0 (which shall be present) shall be substituted" | **complete**: the clause's own substitute is empty, so nothing is drawn by the clause's route |

`LoadedFont::holds_no_outline` is the condition for the last: every glyph the program holds draws
successfully and draws nothing. A glyph that fails to draw answers `false`, so a damaged program is
not silenced, and a subset short of the glyphs its page shows still has other outlines and is still
reported — the failure ADR 0520 exists for. `issue12963.pdf` leaves `INCOMPLETE`; the other two move
to the file's column with a group of their own.

**This amends ADR 0350's control.** Its hollow OCR fixture shown in mode 0 was to report the font as
drawing nothing; it now reports nothing, for the same reason as `issue12963.pdf` — every glyph of
that program, `.notdef` included, is empty, so what mode 0 owes is filling nothing. The control's
purpose survives in its counts: all ten codes still land in `codes_reaching_a_blank_glyph`, which is
what tells "nothing owed" from "nobody looked". The cost is named: a producer whose subsetter
emitted a program with no outline at all now draws a blank line with no report; a program with even
one outline still reports.

## 2. The three nesting documents are cycles, and the report says so

Read by hand, each re-enters a stream it is running whatever state it inherits, because the stream
re-entered selects its own paint: `operator_list_cycle.pdf` form 7 → form 9 → pattern 11's cell →
form 7; `issue19800.pdf` `/X1` → `/X0` → `/X1`; `ContentStreamCycleType3insideType3.pdf` glyph `a` →
glyph `c` → pattern `/P1`'s cell → glyph `a`. No value of `MAX_FORM_DEPTH` finishes them.

**The brief asked for refusal by identity; ADR 0793 declined it, and its argument was tested and
holds**: a form that fills with the current colour, under a tiling pattern whose cell draws the
same form in black, is on the chain twice and ends — `hostile_budgets.rs`'s
`a_form_that_reenters_itself_under_another_colour_draws_whole` is that file, and it draws whole. So
identity **names and does not refuse**: `Interpreter::chain` holds the identity of each nested run
(the address of the stream's cached bytes, which `Document::get` shares across every route to one
object), and at the bound a stream already on it makes the report `NestingCycle { stream }` instead
of `LimitReached`. The classification stays **neither one**: §9.6.4's errata makes the glyph case
"implementation-dependent" and Table C.1 leaves nested `XObject`s to the processor.

## 3. The bound's value, by census

`examples/nesting_census` over page one of 83 911 documents (pdf.js, `doc/corpora`, the SafeDocs
crawl, the Tika tracker's three batches, openpreserve), 1128 s behind the lock at four threads: 489 unread, **nine** reach the bound and **all nine
on a chain that re-enters itself**, none on a chain of distinct streams. The nine are the three above,
`GHOSTSCRIPT-698226-0.pdf` and `GHOSTSCRIPT-700301-0.pdf` (ADR 0793's two), and four PDFBox files
(`PDFBOX-4039-1`, `PDFBOX-4372-1`, `PDFBOX-4666-0`, and the reduced `PDFBOX-4372` page in
`doc/corpora/pdfbox`). So 64 cuts no finite chain the
population holds, and the value stays ADR 0793's.

## Consequences

- `INCOMPLETE` loses `issue12963.pdf`; with `freetext_no_appearance.pdf` leaving by ADRs 1413 and
  1414 in the same batch it is 59: 55 the file's, 4 neither one, none this reader's.
- `issue12963.pdf` page 1 joins the oracle's judged pages.
- A later round that finds a document reporting `LimitReached { limit: "MAX_FORM_DEPTH" }` has found
  a finite nesting this bound cut, which is what would reopen its value.
