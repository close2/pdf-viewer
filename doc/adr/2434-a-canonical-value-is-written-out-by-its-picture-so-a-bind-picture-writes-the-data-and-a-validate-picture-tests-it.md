# 2434 — A canonical value is written out by its picture, so a bind picture writes the data and a validate picture tests it

Session 1800, a clause round in batch 120. Status: implemented. ADR 2435 is unused.

## Context

ADR 2414 built XFA 3.3's canonical forms and the picture-clause parser. It named two kinds of field
in `Written::xfa_unkept`, because each needs a picture's output formatting. One is a `bind` picture:
page 176 converts the data from canonical into its local format when it is saved. The other is a
`validate` picture: its format test (page 382) must format the canonical value, and page 162 updates
the data only once the test succeeds. RFC 0011 section 5 step 4 is that test. XFA 3.3 is cited by
chapter and printed page and paraphrased, never quoted (RFC 0011's preamble).

## Decision

1. **The formatter is a writer per category beside the parser** (`pdf_model::xfa::picture::format`,
   a child of the parser's module, so it reads the parser's own tokens). Chapter 26 defines each
   symbol for both directions, and the formatter writes the output half. That covers literals and
   the global symbols (page 1152), the date symbols (page 1167), the time symbols (page 1173), the
   numeric symbols with `z`, `Z`, `8` and `%` (pages 1177–1181), text (page 1183), and null and zero
   (pages 1184–1185). Chapter 25's `Format` (page 1125) says what each category takes: a date
   clause takes a date or a date-time's date, and a time clause a time or a date-time's time.
2. **Alternatives are tried in order for output as for input.** Page 1159 says so and EXAMPLE
   26.12 shows it. Pages 159 and 165 say only the first alternative formats. Under that reading a
   validate picture with alternatives could never pass its second, so chapter 26's reading is the
   one built.
3. **A value an alternative cannot write is not written by it.** Chapter 26 states no rounding
   and no truncation. So the formatter does not write a value where it would drop an integer digit,
   a fraction digit (except under `8`, which writes all of them) or a sign, or where it would have
   to invent a part, such as a day the date lacks or a second the time lacks. A time's fields below
   the picture's precision are dropped, as EXAMPLE 26.22 drops `h:MM A`'s seconds. A time carrying
   a zone is not written: no zone symbol is read, and the clock without its zone is another
   instant. This is a choice. It keeps parse ∘ format the identity, which the tests prove, and it
   keeps a format test from passing a value its picture cannot show.
4. **Both `warning` and `error` keep a value that fails the format test out of the data.** Page
   880 makes `warning` the default and lets the person go on. Page 385 lets the person dismiss or
   override, and page 162 updates the data only after the test succeeds. A save asks no one, so an
   override was never given. The field is named in `xfa_unkept` with the level and the reason. Page
   396 exempts an empty value, and `disabled` turns the test off.
5. **A `bind` picture writes the data, and a value it cannot write is written canonical.** This
   mirrors page 493's widget, which shows content its picture cannot format just as it is. It also
   mirrors Rule 2 (page 163), which reads such data back as canonical. A `bind` picture this
   program cannot read is named rather than skipped. Rule 2 names a `bind` picture beside the edit
   picture, and page 177 treats a script assigning a value as a person typing one. So a field with
   no edit picture is parsed by its `bind` picture, and Rule 2A's defaults apply only with neither.
6. **Three parser corrections the round trip found.** An `8` right of the radix reads every
   fraction digit, since page 1177 says it parses as it formats. A null or zero clause reads the
   numeric symbols, as page 1184 allows, and a zero clause matches only a zero. Before this change
   `zero{9}` was refused outright. A repeated date symbol and an hour of the meridiem without its
   meridiem are refused for parsing only (pages 1171, 1175). An `8` left of the radix is refused
   for formatting, because page 1181 leaves its meaning undefined.

## Consequences

- None of the four hybrids carries a picture (ADR 2414), so they save and read back as before;
  the proof is `crates/pdf-model/tests/xfa_datasets.rs` and three unit tests beside the parser.
  Those run chapter 26's output tables as printed and the round trip for each category. Both new
  integration tests fail when the bind picture or the format test is planted out (trap 13).
- Two printed rows are not matched. EXAMPLE 26.28's `$ZZ,ZZ9.99CR` prints two spaces after the `$`
  where four digits in five places pad one. Its `$z,zz9.99DB` row prints no trailing spaces for a
  positive number, while page 1178's row for `DB` writes them; the symbol's row is followed.
  EXAMPLE 26.37 tells a null from an empty string. This program's data holds both as the empty
  string, so `null{…}` formats the empty value.
- Not built, and named where it bites. Chapter 15's extended mapping rules include a default bind
  picture that a `config` packet's `transform` supplies (page 527). Whether they govern a `datasets`
  packet already inside the file is unread, and so is every other rule there; it is the next
  reading. The formatter does not write `E`, `w`, `Z`, `z` or the ideographic symbols, for the
  same reasons ADR 2414 gives for the parser.
