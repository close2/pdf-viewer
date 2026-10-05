# 1521 — A repeated report is one lookup, and a show with no font reads none of its string

Session 1343. Status: accepted and **built**.
Context: ADR 1507 section 4 (the `page` target's slow unit of shows with no font, left to
`doc/todo/49`); ADR 1424 (a too-long string stepped over by the grammar that ends it); CLAUDE.md
principle 3. Code: `Notes` and `KeywordNote` in `crates/pdf-model/src/content/report.rs`;
`Interpreter::note` and `note_keyword` and the operand closure in `content/run.rs`;
`ContentReader::with_operand` and `Strings`, `LiteralStringEnd`, `hexadecimal_string_end` and `ahead` in
`content/reader.rs`; the field types in `content.rs`. Tests: `reader.rs`'s
`a_string_stepped_over_ends_where_the_lexer_ends_it`, `tests/hostile_budgets.rs`'s
`text_shown_with_no_font_is_counted_once_a_page_and_reads_none_of_its_strings`.

## 1. What the slow units were

Not shows. `fuzz/artifacts/page/` holds two units that report text with no font:
`slow-unit-46c2ca1b…` (553 483 shows) and `slow-unit-021ace25…` (16 716 558). Both are Type 3 glyph
descriptions that invoke themselves (`NestingCycle`, `MAX_FORM_DEPTH`) until `MAX_OPERATIONS`, so
their time is four million operators. A show with no font already returned at once and was counted
once a page. Callgrind (release, the base commit) put the first at 15.34 G instructions, of which
`Interpreter::note` and the sentences built for it were about a fifth: 3 794 874 notes of a few
hundred distinct items, each formatted, cloned, compared down a `BTreeMap` and dropped twice. The
second spends 46.64 G, and its 16.7 M strings each cost the lexer a growing `Vec`, the operand an
`Arc<[u8]>` copy, and two frees.

## 2. What the standard says such a show is

§9.3.1, Table 103's `Tf` row: "There is no initial value for either font or size ; they shall be
specified explicitly by using Tf before any text is shown." The file is in defect; nothing defines
what to draw, so nothing is drawn and the page says so (`Unsupported::Text`, once, with the count).
That decides the cost: a show that draws nothing should cost lexing it.

## 3. The decision

- **`Notes`** replaces the page's `BTreeMap<Unsupported, Unsupported>`: an ordered set asked before
  it is changed, plus a memo of keywords by their bytes for the three sentences a keyword produces
  (unknown operator, keyword inside an array, operator short of operands). A repeat is one hash of
  a few bytes and builds no sentence; `notes_raised` still counts it. One value, so a checkpoint
  saves and restores both halves together. A `BTreeMap` memo was measured and was slower.
- **`with_operand(Strings::StepOver, …)`**: while no font is in force the operand loop steps over a string by
  §7.3.4.2's balancing parenthesis (one `LiteralStringEnd`, now shared with `Window::drop_token`)
  or §7.3.4.3's `>`, and lends an empty string, which shares one `Arc`. Exact, not heuristic:
  §7.8.2 makes an operator's operands the ones since the last operator and every operator clears
  them, so a string read under no font reaches only an operator dispatched under no font. A string
  the window has not wholly buffered takes the ordinary road.

## 4. Measured

Instructions, release, the base commit against it with only these hunks: `46c2ca1b` 15.34 →
12.61 G (−17.8%), `021ace25` 46.64 → 40.62 G (−12.9%). Reports byte-identical over all 70 units of
`fuzz/artifacts/page/`. The cost on ordinary pages, `callgrind_interpret` once: page 101 of ISO
32000-2 172.07 → 172.35 M (+0.16%), `tracemonkey.pdf` page 1 29.68 → 29.70 M (+0.05%) — the font
test per operand token and the look past white space before the first `Tf`. The first build was
+0.18% on `tracemonkey.pdf` because the string scan, inlined, stopped `with_token` inlining into
the operand loop; `step_over_a_string` is `#[inline(never)]` for that reason (trap 73's shape). What is left of the two units is lexing and `NestedContent::of` per glyph run (a fifth of
the second: `Document::nested_content_source` asked again for every code), which is per-operator
work `MAX_OPERATIONS` counts.

## 5. Why the fixture asserts no time

A wall-clock bound in a test is a bound on the machine's load, and `interpret` takes no clock (ADR
1507 section 3); no allocation counter exists without `unsafe`. So the fixture holds what the cost
was made of: every fontless show counted, one report for ten thousand unknown keywords, and a fill
after every string drawn — which moves if a string is ended anywhere but where the lexer ends it.
