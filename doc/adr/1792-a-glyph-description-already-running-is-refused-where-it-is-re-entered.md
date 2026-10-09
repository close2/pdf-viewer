# 1792 — A glyph description already running is refused where it is re-entered

Session 1478. Status: **accepted** and **built**. Amends ADR 0793 §2 and ADR 1411 §2 for one of
§7.8.2's kinds of nested stream, the Type 3 glyph description; both stand for every other kind.
Clauses: ISO 32000-2 §9.6.4 (Errata Collection 3, Issue #111), Table 110. Code:
`crates/pdf-model/src/content/text.rs` (`draw_type3_glyph`), `crates/pdf-model/src/content.rs`
(`Interpreter::descriptions_running`, `MAX_FORM_DEPTH`'s comment). Tests:
`crates/pdf-model/tests/hostile_budgets.rs` (`a_glyph_showing_itself_in_its_inherited_font_is_refused_at_its_reentry`,
`a_glyph_showing_another_glyph_of_its_own_font_draws_whole`,
`a_type3_cycle_through_a_tiling_cell_is_refused_at_its_reentry`,
`a_form_cycle_through_a_tiling_cell_is_refused_by_its_list_bytes`), `crates/pdf-model/tests/type3.rs`
(`a_glyph_that_shows_itself_is_refused_at_its_reentry`).

## Where the time went

Round 1471's `page` campaign left a slow unit, `fuzz/corpus/page/021ace25f4a1008ff03f4cf3e79749adfa95731e`
(164 694 bytes): two crawled documents spliced by the fuzzer, so that a Type 3 font's `/CharProcs`
resolve to the other document's page streams. Those show codes with no `Tf` that finds a font, so
§9.6.4's inherited graphics state supplies the font being drawn, and nearly every code names `/g0`,
the description running. Before this change, in a release build:

| | before | after |
|---|---|---|
| `interpret`, page one (`examples/open_one`) | 2.24 s | 71 ms |
| instructions (`examples/callgrind_interpret` under callgrind) | 40 695 905 071 | 385 420 755 |
| display-list commands | 2 529 | 1 034 |
| reports | `Text { operations: 16 716 558 }`, `MAX_FORM_DEPTH`, `MAX_OPERATIONS`, `NestingCycle` /g0, `NoninvertibleMatrix` (2 514) | `NestingCycle` and the spliced streams' own faults |

99.8 % of the instructions are under `show_text` inside nested descriptions, and no stage holds
them: decoding the description again (`content_stream`, 21.7 %), tokenising it (15.8 %), stepping
over its strings (8.7 %), allocation (15 %), cloning the state (3.7 %), formatting the detail string
(3.4 %). So no per-run lever was worth building; the volume was the defect. A description showing n
codes that reach itself is a tree of n^d runs at depth d, so `MAX_FORM_DEPTH` (64) is asked of
almost none of it and `MAX_OPERATIONS` is what ended the page — **and with it every mark after the
text**, which is where the 2 514 commands at a singular matrix came from and the page's own content
went. The confined worker's `REQUEST_TIMEOUT` bounds decodes, not interpretation; nothing outside
the interpreter's own budgets was reached, and the viewer's one-second `WARN` would have fired.

## Decision

**A Type 3 glyph description already running further out is refused where it is re-entered**,
before its stream is decoded, and reported as `Unsupported::NestingCycle` naming the glyph. Its
identity is its `/CharProcs` stream object, because that is what "itself" is: two codes or two
fonts reaching one stream reach one procedure. The text position still advances by `/Widths`, as
for any glyph that paints nothing.

The clause is what licenses it, and it is why the exception is the glyph's alone. §9.6.4, as
Errata Collection 3 inserts it below NOTE 1:

> Implementations also need to avoid potential infinite recursion if a Type 3 glyph description
> refers to itself directly or indirectly. The result in all such cases is implementation-dependent.

"In all such cases" covers the finite re-entry ADR 0793 §2 was written to protect — a description
reaching itself through a pattern whose cell selects another paint — so refusing it is a result
the clause permits, and a report on it fires on the condition the clause states. For forms, tiling
cells, soft masks and appearances no clause says that, ADR 0793's argument holds, and identity
still only names at `MAX_FORM_DEPTH`. A glyph showing a *different* glyph of its own font — an
accent built over its base letter — is not a re-entry and draws whole; that is the control test.

## What it moved

- `display_list_digest` over the first page of all 1 477 documents of `doc/pdf.js` and
  `doc/corpora`, two binaries of the same name in two directories: **one** line moves,
  `ContentStreamCycleType3insideType3.pdf`, 1 762 627 commands and `ListBytes` → 44 commands and one
  `NestingCycle` naming `/rect`. Looked at (trap 1): before, `MAX_LIST_BYTES` cut the page inside its
  first glyph and the page's second glyph, the green triangle, was never drawn; after, the triangle
  is drawn and the red square holds its text. `Type3Test.pdf` and
  `ContentStreamNoCycleType3insideType3.pdf` do not move. `raster_golden` agrees: 973 held, that page
  moved, its line regenerated.
- `hostile_budgets.rs` held `MAX_LIST_BYTES` with a Type 3 cycle, which no longer reaches it. A form
  is refused by depth alone, so the bound's test is now a form whose tiling cell draws the form under
  Helvetica text in mode 2 — 537 MB charged, inside a sixteenth of the bound.
- The §9.6.4 row's note said the corpus witness did not reach the bound because its glyph was `d1`;
  its glyphs are `d0`, and ADR 1411 had already found the cycle. The note now says what is built.

- `examples/nesting_census` over page one of 90 763 documents (the crawl, the Tika tracker,
  openpreserve, `doc/pdf.js`, `doc/corpora`; 803 s, peak 6.35 GiB): **10** report a re-entry — ADR
  1411's nine, and `poppler-102718-0.pdf`, new to the population since and a glyph cycle at HEAD as
  well (`MAX_FORM_DEPTH`, `MAX_OPERATIONS` and `NestingCycle` /striangle, 773 ms, 333 338 commands,
  the page's second `b` never drawn; now 0.7 ms, 6 commands, both drawn). So no document on the list
  is a finite re-entry this refusal cut short, which is the case ADR 0793 §2 feared. Page one only.

## Consequences

- A cycle through glyphs costs one comparison per description running, at most 64, asked before
  decoding.
- A file whose glyph reaches itself through a pattern under another paint — finite, and drawn whole
  before — now draws that inner glyph as nothing. The clause permits it and the census found none; a
  round that finds one owes the census its name, not a reopening of the rule.
