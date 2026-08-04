# §12.7.4.3's remaining edges

Status: partly reported, partly unreached.
Priority: 22
Corpus: 4 documents
Clauses: §12.7.4.3, §12.7.5.4, §9.6.5.2, §9.7.6.2
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
  paragraph of Arabic) decline, and an inference may not fall short. **The reason was written down
  wrong here and in the report, and the two-hundred-and-eighty-third session corrected both**: it
  is not that "Helvetica has no glyph for their characters". For `bug1865341.pdf` the missing set
  is **one character, `ą`** — Liberation Sans has `aogonek` and so does every Helvetica clone, and
  `ł` is not missing at all because Adobe's `StandardEncoding` happens to include `lslash`. What
  is missing is a **code**: a simple font reaches a glyph only through §9.6.5.2's encodings, and
  neither `StandardEncoding` nor `WinAnsiEncoding` has a Polish *ogonek*. The report says so now,
  naming both halves.

  So closing this one needs a way to address a compiled-in face by *character* rather than by
  code, and the standard states exactly two: a `/Differences` array naming `aogonek`, which needs
  a character-to-glyph-name table this tree does not have and whose obvious source is GPL
  (`doc/HANDOVER.md` §1's trap), or an invented `/Type0` font with `/Encoding /Identity-H` and a
  `CIDFontType2` descendant, where the code *is* the glyph — which `resolve_font` would have to
  build and which the guard at `addresses_characters` currently refuses. The second needs no
  vendored data and is the one to take.

  `freetext_no_appearance.pdf` is **not** closed by either: no Helvetica has Arabic, so that one
  is `doc/todo/21`'s per-character fallback and stays refused.
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
