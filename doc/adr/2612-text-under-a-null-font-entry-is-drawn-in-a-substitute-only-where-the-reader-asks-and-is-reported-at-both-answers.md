# 2612 — Text under a null `/Font` entry is drawn in a substitute only where the reader asks, and is reported at both answers

Session 1889, Slot 9 of batch one hundred and twenty-nine, 2026-10-29.
Status: implemented — A738 builds A35's option. Context: `doc/questions/A35` (Q35's recommendation
approved), `doc/questions/A738` ("owed, as a reader setting off by default"), ADR 0893 (the null font
dictionary and why the standard offers nothing to fill it with), ADR 0779 (the two kinds of absence),
ADR 2550 (the show says the sentence), ADR 1228 and ADR 2440 (the two reader preferences whose shape
this copies). Code: `crates/pdf-model/src/content/font.rs` (`substitutes_for`, `null_font_substitute`,
`Absent::said`, `FontKey::NullSubstitute`), `crates/pdf-model/src/content/text/show.rs`,
`crates/pdf-model/src/view/settings.rs`, `crates/viewer-core/src/command.rs`
(`Command::NullFontSubstitute`), `crates/viewer-confined/src/protocol/command.rs`.

## 1. The clauses

§9.3.1's Table 103 makes `Tf`'s operand "the name of a font resource in the Font subdictionary of the
current resource dictionary", and Table 34 makes `/Font` a dictionary "that maps resource names to font
dictionaries". §7.3.9: "An indirect object reference (see 7.3.10, "Indirect objects") to a nonexistent
object shall be treated the same as a null object." So an entry naming an object the file lost is the
null object, no font dictionary, and §9.5's substitution (every input of which is an entry of the
font dictionary) has nothing to read. ADR 0893 recorded the case as an absence; Q35 asked whether a
reader should guess anyway, and A35 approved the guess as a thing the reader asks for. A738 makes it owed.

## 2. Decided

1. **The setting is the reader's, off by default, in the place a host supplies.**
   `ViewState::set_null_font_substitute(bool)` holds it and `pdf-model` never sets it;
   `viewer_core::Command::NullFontSubstitute(bool)` carries it into every open document and every one
   opened afterwards, as `Command::Separations` does, and it crosses the confined worker's protocol as
   tag 46. A host that never sends it changes nothing: such text draws nothing, as before.
2. **Only the null object is substituted.** The entry must be stated and resolve to `null`, whether by
   `Tf`'s resource name or Table 57's `/Font` reference. An entry that names something present and not
   a font dictionary (an integer, a stream) is another fault, and a name the resources do not state is
   §7.8.3's absence (ADR 0779); neither is what A35 or A738 asked about, so neither moves.
3. **The face is §9.6.2.2's Helvetica, at that face's own advances.** §9.6.2.2 requires every
   processor to have the fourteen, and §9.6.2.1 (the paragraph after Table 109) says processors "shall
   provide glyph widths and font descriptor data for those standard fonts for use in processing PDF
   files when the entries are absent" — those widths are the defaults this program holds for a font
   dictionary that states none, so the guess is laid out by them (`Helvetica.afm`: `A` is 667). The
   upright sans-serif is the member that presumes least about style. Neither the face nor the advances
   is derived from the file; both are this program's choice, by principle 5's rule for a silence.
   **Q35's "/MissingWidth-style default advances" read as the face's defaults rather than one constant
   for every code**: Table 120's `/MissingWidth` defaults to 0, which would stack every glyph, and any
   other single figure would be a number with no source.
4. **The report is kept at both answers.** The show says ADR 2550's sentence whole in either case, so
   the population that sentence counts is unchanged; under the setting it adds "its text is drawn in
   Helvetica at that face's own advances, as this reader asked", because a substituted line is the
   page's least checkable output (Q35's table: a legible letter on one document, black bars on another).
5. **Cached once a page, under its own key.** `FontKey::NullSubstitute` holds the face in the page's
   cache, so a page selecting the name on every line loads it once, and a `Tf` under any other name
   keeps its cached path unchanged; the cache that outlives the page never holds it.

## 3. What was rejected

Substituting under the entry's own object key: the cached path would then have to resolve every
cached font's entry again to know whether it owes the sentence, the per-`Tf` copy ADR 2550's note on
`font` removed. A constant advance: see 2.3. A setting in `pdf-model` decided by inference from the
codes (ASCII or not): that is the product judgement A35 gave to the reader.
