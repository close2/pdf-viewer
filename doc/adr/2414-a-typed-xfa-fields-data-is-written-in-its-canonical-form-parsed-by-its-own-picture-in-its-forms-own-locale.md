# 2414 — A typed XFA field's data is written in its canonical form, parsed by its own picture in its form's own locale

Session 1790, a clause round in batch 119. Status: implemented. ADR 2415 is unused.

## Context

ADR 2396 made a hybrid form's save write each changed field's `datasets` value. It named every
numeric and date field in `Written::xfa_unkept`, because the Data DOM keeps such values in XFA
3.3's canonical form (chapter 4, page 156; chapter 24, pages 1045–1048), and a `/V` is what a
person typed. RFC 0011 section 5 step 3 is that conversion. XFA 3.3 is cited by chapter and printed
page and paraphrased, never quoted (RFC 0011's preamble).

## Decision

1. **The canonical forms are a recogniser and a writer each** (`pdf_model::xfa::canonical`).
   A date is `YYYY[-MM[-DD]]`, a time `HH[:MM[:SS[.FFF]]]`, a date-time the two joined by `T`, a
   number page 1047's literal. Both of page 1045's spellings are recognised. The hyphen and colon
   spellings are written, which is the form chapter 4's own examples print. A time is written to
   the precision its picture stated. A number keeps the fraction digits that were typed and drops
   leading zeros (page 1182).
2. **A typed value is canonicalised by chapter 4's input-parsing rules** (pages 162–164), in
   `xfa::datasets::typed`. With an edit picture, the first alternative that matches wins (Rule 2).
   With none, a `date`, `time`, `float` or `decimal` is parsed by the locale's default picture
   (Rule 2A). Page 166 makes number localisation depend on a `numericEdit` ui. **Where nothing
   matches, the text is written as typed**, because Rule 2 says such text counts as canonical
   already. This applies to `textEdit` too: page 163 makes canonicalisation follow the picture,
   whatever the widget.
3. **The picture-clause grammar is read for parsing** (`xfa::picture`, chapter 26). It reads
   alternates and compound, locale-specific and predefined clauses. It infers a bare clause's
   category from its symbols, and the field's own value type breaks a tie (page 1157). It reads
   the standard date, time, numeric and text symbols, the null and zero categories, and the global
   `?`, `*`, `+`. The matcher backtracks, and it remembers the failed positions of numeric and text
   clauses, so a long run of optional digits costs polynomial time and not exponential.
4. **Locales come from the form's own `localeSet` packet and nothing else** (`xfa::locale`,
   chapter 19). They are inherited from language to root as page 158 describes. Page 158 requires
   a form inside a PDF file to carry its locales resolved, so this program ships no locale
   database. A property the packet lacks is named, not guessed. The prevailing locale follows page
   1153: the clause's own, the container chain's, then the `config` packet's `common` `locale`,
   then `en_US`, because this program supplies no ambient locale of its own.
5. **Choices, each documented where the standard leaves it open.** The predefined `default` is
   `med`, because page 1156 makes it implementation-defined, and Rule 2A's default date and time
   pictures are `med` too. A percentage is divided by a hundred on input: page 1181's prose says
   multiply, its example table divides, and only division makes 12 % mean twelve hundredths. A
   radix may be absent where every fraction symbol after it is optional.
6. **What is not done is named** (trap 5), each with its sentence. That covers non-numeric text in
   a `numericEdit` field, which page 156 would make zero, putting the data out of agreement with
   the `/V`. It covers a `bind` picture, whose data would have to be formatted, and a `validate`
   picture, whose format test (page 382) must pass before the data is updated (page 162); that is
   step 4. It covers a `dateTime` typed with no picture, where page 165 joins the default date and
   time pictures with a `T` and page 1156 with a space. And it covers a time zone, an exponent, a
   week number, the full-width and ideographic symbols, and the `datetime{}` category.

## Consequences

- None of the corpora's four hybrids has a typed field. The brief's hypothesis that two of them
  carry one failed: their `integer` values sit on check buttons, whose `items` already gave the
  data. The proof is two fixtures in `crates/pdf-model/tests/xfa_datasets.rs`, a hybrid built in
  the test with an `nl_NL` `localeSet` like the corpora's. One field per type writes its canonical
  value, and five fields that cannot be written are named. With the canonicalisation planted out,
  both fail (trap 13). The four hybrids still save and read back as they did.
- Four inconsistencies in XFA 3.3 were found and read in the open. Page 1181's percentage prose
  contradicts its table. Its second row types one fraction digit against two required `9`s, so the
  test types two. Page 1176 parses `HH:MM` to an hour alone but `H.MM` to hour and minute. Page
  1047's date-time example carries four fraction digits where page 1046 requires exactly three.
- The next step is step 4, validations. The formatter it needs, which turns a canonical value back
  into a picture's output, also unblocks the `bind` pictures named here.
