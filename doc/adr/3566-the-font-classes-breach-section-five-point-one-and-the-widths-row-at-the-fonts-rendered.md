# 3566 — The font classes breach section 5.1, and the widths row at the fonts a page renders

Session 2216. Status: **accepted**; extends ADR 3506. Context: ADR 3565 (the classes); ISO 19005-2
and ISO 19005-4 sections 5.1, 6.2.11.2, 6.2.11.4.1, 6.2.11.5 and 6.2.11.7.2.

## Context

ADR 3506's `MAPPING` sends every class to section 5.1 and to any row of this part the repair also
touches. The font rows differ from the file-structure rows in one respect: their population is the
fonts a content stream renders, not the file, so a row that read the whole log would fail a font no
page uses.

## Decision

- **Every font class breaches section 5.1** but `font/to-unicode-past-a-bound`, a bound of this
  reader, mapped to no row by name.
- **The widths classes** also breach `fonts/widths-agree-with-the-program` (section 6.2.11.5).
  The row reads the log through `record_logged_at`, narrowed to the objects of the fonts it judges
  — the `/Widths`, `/W`, `/W2` and the descriptor — so a repair in a font no page renders is
  section 5.1's alone.
- **The program classes** (`sfnt-loca-*`, `sfnt-table-past-the-program`,
  `collection-cut-to-one-face`, `cff-font-dicts-rewritten`) **breach section 5.1 alone, and
  `fonts/font-programs-embedded` by name not yet.** Section 6.2.11.4.1 is the row they touch — the
  program embedded is not the one rendered — and the first census of this round mapped them to it.
  The converter reads that row's failure as a font with no program and has no route that writes a
  program as this reader read it, so ten pdf.js documents were refused "not built yet" with a
  sentence false of them. Mapped to section 5.1, the output's check refuses them by the class's
  name, which is true. The row mapping comes back with that route (ADR 3567).
- **Section 5.1 alone**, each for its reason: `program-unreadable` (the embedding row already
  refuses the same stream on its own reading), `font-file-subtype-disagrees` (the filing row reads
  the `/Subtype` against the bytes itself), the `ToUnicode` classes (section 6.2.11.7.2 exempts
  whole kinds of font, which a repair's place cannot tell), `cmap-damaged`, `differences-malformed`
  and `encoding-name-unknown` (no row of this part states their grammar).
- **Section 6.2.11.2's own row stays unchecked**: the log is what this reader repaired, not a
  validator of sfnt, CFF and Type 1, and an implemented row would claim the second.
- A unit test holds every `pdf_font` class to a mapping and every mapped row to the table: the font
  half of ADR 3477's closing gate.

## Consequences

- A file whose rendered font needed its `loca` rewritten fails section 5.1, and the finding names
  the class and the program stream.
