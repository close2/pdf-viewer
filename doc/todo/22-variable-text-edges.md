# §12.7.4.3's remaining edges

Status: partly reported, partly unreached.
Priority: 22
Corpus: 7 documents
Clauses: §12.7.4.3, §12.7.5.4, §9.7.6.2
Code: `crates/pdf-model/src/variable_text.rs`

## A `/DA` font `/DR` does not define — 7 documents

A malformed file rather than a clause gap: §12.7.4.3 requires the name to match a `/DR` entry.
Since ADR 0112 the value is laid out in a stand-in **where the stand-in can draw all of it**, and
the missing font is named. Four Arabic-valued documents decline, because a Latin stand-in drawing
their punctuation and nothing else is worse than a blank — the first version of that ADR drew six
dots on an otherwise empty page, which is why the rule is "a stand-in may not fall short".

## A composite `/DA` font, a list box, `/DS`, `/RV` — 0 documents

The rest of the clause's edges, none of them reached by any corpus document:

- a **composite** `/DA` font needs §9.7.6.2's codespace ranges inverted, to turn a character back
  into the code the font wants;
- §12.7.5.4 states which items of a **list box** are selected and nothing whatever about how that
  looks;
- `/DS` and `/RV` are XFA, which `CLAUDE.md` excludes.
