# §12.7.4.3's remaining edges

Status: partly reported, partly unreached. **Table 231's `DoNotScroll` closed in the
three-hundred-and-thirty-eighth session** (ADR 0197); what it left is one query's shape.
Priority: 22
Corpus: 3 documents
Clauses: §12.7.4.3, §12.7.5.3, §12.7.5.4, §9.6.5.2, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`, `crates/pdf-model/src/view.rs`

## A `/DA` font `/DR` does not define — **3 documents, from 7**

A malformed file rather than a clause gap: §12.7.4.3 requires the name to match a `/DR` entry.
Since ADR 0112 the value is laid out in a stand-in **where the stand-in can draw all of it**, and
the missing font is named. The rule is asymmetric on purpose: a Latin stand-in drawing a paragraph
of Arabic's punctuation and nothing else is worse than a blank, and the first version of that ADR
drew six dots on an otherwise empty page.

**Five of the seven named one of §9.6.2.2's fourteen and nobody had noticed** — the
two-hundred-and-fifty-eighth session. `/Helv`, `/HeBo`, `/TiRo`, `/ZaDb` and their ten siblings
are the four-letter abbreviations of the standard 14 and there is no fifteenth, so the table is a
*bijection* with the clause's own list rather than a habit read off a corpus. A `/DA` naming one
therefore names a font this binary carries, and the value is drawn in the face the name means: a
documented choice about a malformed file, and one that buys ADR 0133's own argument — those pages
reproduce where no fonts are installed.

Still owed, and the three are what is left:

- **`bug1865341.pdf` is closed** (two-hundred-and-eighty-fourth session, ADR 0184). Its value is
  *Załącznik* and its missing set was **one character**, `ą` — not a glyph any Helvetica lacks but
  a **code** neither §9.6.5.2 encoding has. A font this module invents may state its own encoding,
  so it does: `/Encoding << /Differences [1 /aogonek] >>`, with the name from the Adobe Glyph List
  that `read-fonts` already carries. Kept only when it reaches strictly more characters, which is
  what leaves the next entry refused.
- `freetext_no_appearance.pdf` (`/Helv`, a paragraph of Arabic) still declines, and the
  `/Differences` route cannot help it: the AGL names those characters `afii…` and no Helvetica has
  the glyphs. **That one is `doc/todo/21`'s per-character fallback**, where it is the only witness
  left.

- `poppler-395-0-fuzzed.pdf` names `/Rufscript` and `issue19389.pdf` names `/F1`: neither denotes
  anything, so both stand in and both say so, which is the case the table is narrow enough to
  leave alone.

## A composite `/DA` font, a list box, `/DS`, `/RV` — 0 documents

The rest of the clause's edges, none of them reached by any corpus document:

- a **composite** `/DA` font needs §9.7.6.2's codespace ranges inverted, to turn a character back
  into the code the font wants;
- §12.7.5.4 states which items of a **list box** are selected and nothing whatever about how that
  looks;
- `/DS` and `/RV` are XFA, which `CLAUDE.md` excludes.

## ~~Table 231 bit 24's `DoNotScroll`~~ — **done in the three-hundred-and-thirty-eighth session**

ADR 0197. `LaidOut::overflows` answers whether the value needs more room than the box gives, on the
axis the clause names — horizontally for a single line, vertically for several, and a count of
Table 232's cells for a comb — and `ViewState::set_field` takes the longest prefix that fits, over
every widget of the field, shortest wins. **260 corpus widgets over 8 documents set it**, which
`examples/field_flag_census` measured in that round and which makes it the most-stated of every
type-specific flag in Tables 229, 231 and 233; four of the twenty have no witness at all.

### What it left owed: a host cannot read back what was accepted

`Query::FieldAt` answers with §14.9.3's two names and **no value**, so a host with a text box of its
own has no way to learn that a field took less than it sent. Nothing is wrong today — `viewer-ui`
sends no `Edit::SetField` at all — and it becomes wrong the moment a host types. The fix is the
familiar shape from ADRs 0166 and 0167: where a host needs a thing a variant does not carry, the
variant changes and every consumer fails to compile.

**And a caret is the other half**, which is `33`'s: a truncation is what this looks like to a host
that sends whole values, and *nothing happening as you type* is what the clause describes.

## A field's baseline reads the same two entries with a weaker guard — **new in the three-hundred-and-seventy-eighth session**

`Metrics::read` here and `pdf_font::vertical_extent` both build a line from Table 120's `/Ascent`
and `/Descent`, and since ADR 0216 only the second of them asks whether the pair could be a
measurement. This one's guard is `ascent > 0 && descent < 0`, which is stricter than the ordering
ADR 0216 replaced and still accepts `/Ascent 4000 /Descent -1140` — a face `PDFJS-9279-reduced.pdf`
states — and still refuses the 42 font dictionaries that write a `/Descent` without its sign, where
the em-relative fallback stands in and the baseline lands somewhere the file did not ask for.

**It was left alone deliberately**: this one *draws*, so sharing the band would move pixels, and the
round that took ADR 0216 measured none. What it owes is small and known — call `measured_extent`,
keep `DEFAULT_ASCENT`/`DEFAULT_DESCENT` as the fallback for a pair it rejects, and re-run the
oracle and `variable_text`'s own ink gates before and after. `examples/font_metric_census` already
counts the population.
