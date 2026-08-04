# §12.7.4.3's remaining edges

Status: partly reported, partly unreached.
Priority: 22
Corpus: 3 documents
Clauses: §12.7.4.3, §12.7.5.4, §9.6.5.2, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`

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

## Table 231 bit 24's `DoNotScroll`, which is a `shall` and is not implemented

Found by `doc/todo/01`'s second sweep in the three-hundred-and-fourth session, in §12.7.5.3's own
"Not read:" list, where it had been sitting behind the reason *"constrains typing"* — true when
written and false since the hundred-and-thirty-fifth session, which is when this program learned
to fill a field. **A flag that constrains typing binds a program that types.**

> If set, the field shall not scroll (horizontally for single-line fields, vertically for
> multiple-line fields) to accommodate more text than fits within its annotation rectangle. Once
> the field is full, no further text shall be accepted for interactive form filling; for
> non-interactive form filling, the filler should take care not to add more character than will
> visibly fit in the defined area.

Two sentences, and only the second binds a reader: a `shall` about *accepting* text. This program
accepts whatever `Edit::SetField` hands it.

**What it costs is a measurement rather than a flag.** `variable_text::lay_out` returns a content
stream and an `Owed`; it says nothing about how much of the value it placed, so there is no way to
ask "does this fit" without it reporting where it stopped. That is the change — `LaidOut` gaining
the byte offset the layout reached — and everything else follows from it: `ViewState::set_field`
lays the candidate value out for each of the field's widgets and keeps the longest prefix that
fits, which is what "no further text shall be accepted" means for a host that sends whole values
rather than keystrokes.

**And it is worth doing after `33`'s caret rather than before**, because a caret is what turns this
from a truncation into the behaviour the clause describes: a person typing into a full field sees
nothing happen, which is the point.

Corpus: unmeasured. The count of widgets setting bit 24 is one sweep and belongs in the round that
takes it.
