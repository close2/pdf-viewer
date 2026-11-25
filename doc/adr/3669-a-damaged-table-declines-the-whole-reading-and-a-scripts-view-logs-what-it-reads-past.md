# 3669 — A damaged table declines the whole reading, and a script's view logs what it reads past

Session 2242, the silent-repair campaign's census burn-down, third pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3574 (the census and its `HELD` ratchet), ADR 3597, ADR 3621;
ISO 32000-2 §7.3.4.2, §7.6.2 Table 20, §8.11.2.1 Table 96, §12.5.2 Table 166, §12.7 Tables 226
and 232; the XFDF 3.0 text, *String encoding conventions* (pages 28 and 29).

## Context

Round 2230 left 515 held sites. This round read the largest held files: the content reader, the
XFDF annotation reader, the pair-kerning reader, the standard security handler, the script model,
the rich-text style and markup readers, the geospatial reader, the soft-mask and JPEG readers, and
three font readers. Most sites are a reader's own bounds; a few replaced a value the file stated.

## Decision

- **A kerning table that does not read declines the program, at read.** `Pairs::read` already
  answered `NoPairs::Unreadable` for a `kern` subtable header that does not read; it now reads, once,
  every table `pair_values` and `kerned` later consult — the script and language-system tables, the
  `kern` features, each reached lookup, each pair subtable's coverage and class definitions, a
  format 0 or 2 `kern` body — and declines the program where one does not. Before, a damaged
  subtable matched nothing at layout, where no report reaches; now the run is unkerned and the
  rich-text layout says `kerning-mode:pair, in a face whose kerning tables could not be read`.
- **A negative key length is refused.** Table 20's `/Length` "shall be a multiple of 8, in the
  range 40 to 128"; a negative one was read as absent and took the 40-bit default, so the file was
  refused later as a wrong password. It is refused where it is read, and the test that pinned the
  old leniency now pins the refusal.
- **`/V` joins `syntax/encryption-entry`.** Table 20 makes `/V` required of every handler; one
  absent or not an integer is read as 0, now logged beside the standard handler's `/R`, `/O`, `/U`
  and `/P`. The class's sentence names it; its mapping is unchanged.
- **XFDF's octal escape keeps its low eight bits.** The convention's escape is "just like escaped
  sequences in PDF literal string", and §7.3.4.2 says of those "high-order overflow shall be
  ignored"; `\777` was read as a backslash and three digits. A `length` attribute that is not a
  count is a damaged `<data>` element, said on `owed`, where it was read as no length.
- **The script model logs what it reads past**, on the document it holds: a `/MaxLen` or a `/V`
  of a type the field tables do not give (`field-entry-unusable`), a widget's `/F`, `/MK` caption,
  `/BS /W` or `/Border` width, and a `/Rect` that is not four numbers (`annotation-entry-unusable`,
  through `annotation.rs`'s own `flags_as_stated` for `/F`).
- **One new class**, `model/optional-content-group-name-unusable` (§8.11.2.1, Table 96's required
  `/Name`), logged where the script model names a layer by the empty string. It is outside content:
  section 5.1 alone, and the converter's whole-file rewrite carries the group as the file states it,
  leaving the output's own check to refuse by the class's name (ADR 3621's rule).

## Consequences

- The sites read as a reader's bound, a clause's default or a decline carry `// not a repair:`
  markers; three Sonnet helpers marked the image, rich-text, geospatial and font files, and every
  marker was read back here before the count was lowered.
- Held as repairs, for the rounds after: a soft mask's absent `/BitsPerComponent` read as 8 and its
  undecodable data drawn opaque; the JPEG 2000 box readers' short boxes; the rich-text markup that
  fails to parse, an `<ol start>` or `<li value>` that does not read, a list tag a style cannot
  write; the geospatial reader's nameless system and datum, an unreadable EPSG code, a unit node with
  no name, an absent `Auxiliary_Sphere_Type`.
