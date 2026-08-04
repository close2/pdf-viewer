# §12.7.4.3's remaining edges

Status: partly reported, partly unreached.
Priority: 22
Corpus: 4 documents
Clauses: §12.7.4.3, §12.7.5.4, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`

## A `/DA` font `/DR` does not define — **4 documents, from 7**

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

Still owed, and the four are what is left:

- `bug1865341.pdf` (`/Helv`, value *Załącznik*) and `freetext_no_appearance.pdf` (`/Helv`, a
  paragraph of Arabic) decline, because Helvetica has no glyph for their characters and an
  inference may not fall short. **These are the per-character-fallback question one clause over**
  — see `doc/todo/21`, where it has no witness of its own.
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
