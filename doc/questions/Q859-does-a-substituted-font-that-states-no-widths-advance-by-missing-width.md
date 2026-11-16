# Q859 — Does a substituted simple font that states no `/Widths` advance by `/MissingWidth`?

Source: round 2032, on ADR 2870's consequence and the brief's item 2. Nothing waits on the answer: the window keeps
the layout it has, and `crates/pdf-font/src/metrics.rs::a_substitute_with_no_widths_keeps_the_resembling_standard_metrics`
pins it until you answer.

## What was found

- **The tree today.** A simple font outside §9.6.2.2's fourteen that states no `/Widths` advances by one of three
  readings, by the route it took. Drawn from a substitute (a font naming no program, ADR 0007, and since ADR 2870 a
  program present and unreadable): the published metrics of the standard font its name resembles (Helvetica's for
  `/Arial`), then the face's own advance for a glyph outside the standard set. An embedded program that parses,
  and a refused one drawn from metrics alone (ADR 1094): Table 120's `/MissingWidth` for every code, default 0.
  The brief read the first as "the face's widths"; it is the standard font's published table in nearly every code.
- **The clauses.** Table 109 makes `/Widths` "Required; optional in PDF 1.0-1.7 for the standard 14 fonts", and
  §9.6.2.2 calls those "14 Type 1 fonts". Table 120 gives `/MissingWidth` as "[t]he width to use for character codes
  whose widths are not specified in a font dictionary's Widths array", "Default value: 0". Read literally, every
  code of a dictionary with no array is such a code. But such a dictionary is missing an entry the standard requires,
  and the standard describes valid files; whether Table 120's default is what an absent required array means is a
  reading, not a sentence.
- **What the literal reading draws.** Built and looked at (trap 1): `issue5470.pdf` (`/Arial`, `TrueType`,
  `/FirstChar 32 /LastChar 255`, a descriptor, no `/Widths`, no `/MissingWidth`) draws its whole line
  "Issue 5470 - the text should be red" as one pile of glyphs at the line's start, with no report. It was the one
  `raster_golden` page that moved. A font with neither a descriptor nor `/Widths` (A687's ten documents) has no
  `/MissingWidth` entry at all for the sentence to read; the round left those at the resembling metrics in its build.

## The question

For a simple font outside the fourteen that states no `/Widths`, what lays its codes out?

- **(a)** Table 120's `/MissingWidth`, stated or 0, for every code, whatever route draws the font. One reading for
  all three routes; `issue5470`'s line becomes one pile, and should say so out loud as the file's defect.
- **(b)** The best statement the file makes about its glyphs' widths, in an order written down as a choice: an
  embedded program's own advances (Table 109's "consistent with the actual widths given in the font program"), else
  the resembling standard font's published metrics, and `/MissingWidth` only for a code neither answers. The three
  routes agree, and a page missing its `/Widths` keeps a readable line.
- **(c)** Leave it: substitutes by the resembling metrics, embedded and metrics-only by `/MissingWidth`.

## Recommendation

**(b)**, with the choice stated in an ADR as a choice in the file's defect rather than as a clause obeyed, and the
archive's `layout-from-this-viewer` loss (A687) extended to every font it covers, since what it writes is this
reader's layout. (a) is the most literal, and it trades a readable page for a statement the file never made.

## Meanwhile

The window draws as before (c). ADR 2880 (a substitute's code that the file takes to no glyph, reported per code)
was built in the same round and does not depend on this answer.
