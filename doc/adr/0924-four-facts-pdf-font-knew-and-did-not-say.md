# 0924 — Four facts `pdf-font` knew and did not say

Session 940. Status: **accepted**. Recorded under the project owner's standing rule for this
tranche: a change to a shared crate made for `pdf-archive`'s sake is written down as an ADR when
its cost to the viewer is small, and asked as a `doc/questions/Q*` when it is not.

## The clauses that asked

ISO 19005-2 §6.2.11.7 has two rules, and neither could be answered from `pdf-font`'s surface:

- **§6.2.11.7.2, last sentence** — every Unicode value a `/ToUnicode` `CMap` states must be
  greater than zero and be neither U+FEFF nor U+FFFE. That is a statement about the map's whole
  **value set**.
- **§6.2.11.7.2, second exemption** — a Type 1 or Type 3 font is excused from carrying a
  `/ToUnicode` at all when every glyph name it references is in the Adobe Glyph List or the
  Symbol set. That is a statement about the **name** an encoding selected.

§6.2.11.7.3 needs the first of those again, from the other end: which codes map *into* the
Private Use Area.

**§6.2.11.4.2 asked the same kind of question about a different object**, and it is the same
shape of gap. A font descriptor's `/CharSet` string and its `/CIDSet` stream each claim to list
what the embedded **program** contains — the clause is explicit that the claim covers glyphs the
file never uses — and `LoadedFont` answered only per code. A reader that can say what a code
reaches cannot check a claim about the glyphs nothing reaches, which is exactly the case such a
claim gets wrong.

## Why each was unreachable, which is the interesting half

**`ToUnicode` answered per code and enumerated nothing.** So the predicate asked it about every
code in the font's code space — 256 for a simple font, 65 536 for a composite one, under a
document-wide probe budget. That was exact for no font: ISO 32000-2 §9.7.6.2 lets a character
code be one to four bytes, so the space to sweep is four billion wide and any bound on it omits
the tail in silence. A value stated for a three-byte code went unreported, and nothing said so.

**The glyph name was hidden by a clause doing its job.** `LoadedFont::text` and
`LoadedFont::naming_gap` both answer "what does this code mean", and §9.10.2 ends by permitting a
processor to choose a character where its three methods fail. `pdf-font` takes that permission
(`text_from_program`), so a Type 1 code whose built-in encoding names it `integraldisplay` comes
back as `"Z"` — the character its *code* would be in ASCII — and `naming_gap` returns `None`.
Every route to the name ran through a reading that had already replaced it. Two corpus witnesses
turn on exactly this, and the tree passed both by accident.

## What was added

Both are read-only accessors over state the crate already held. Nothing new is computed at load
time and nothing new is stored.

- `pdf_font::tounicode::Mapping` and `ToUnicode::mappings()` — an iterator over the statements
  the `CMap` made, **in the form it made them**: a `beginbfchar` entry as `Single { code, text }`,
  a `beginbfrange` entry as `Span { low, high, first }`. The span is not expanded, for the reason
  the module already refuses to expand it in storage: one `<0000> <FFFF>` line would otherwise
  become sixty-five thousand items, and a four-byte one considerably worse. A caller asking
  whether a forbidden value or a private-use block falls inside a span does it with three
  comparisons.
- `LoadedFont::selected_glyph_name(code)` — a method that already existed, made public. It is
  the table §9.6.5's glyph selection used, before §9.10.2 got to it.
- `LoadedFont::program_glyph_names()` — the names a name-keyed program's charset assigns, for a
  bare Type 1 or a Type 1C CFF. `None` for a substituted font, for an sfnt (which keys by index)
  and for a CID-keyed CFF (whose charset assigns CIDs).
- `LoadedFont::program_character_identifiers()` — the CIDs a CID-keyed CFF assigns, or, for an
  sfnt under §9.7.4.2's default identity `/CIDToGIDMap`, the glyph indices it holds. `None` for a
  substituted font and for a name-keyed program.

The last two parse the program when asked rather than at load, which is the same bargain
`program_cmap_subtables` already struck: a validator asks once per font, a viewer never asks.

## The cost, judged honestly

**Small, and the reason it is small is that both are lazy in the strictest sense: they compute
nothing until called, and the viewer never calls them.**

- `mappings()` allocates one `Vec` of borrowed references to walk the `/UseCMap` chain — a chain
  that is nearly always one long — and yields borrows of data the map already owns. It is not on
  any extraction path; `append` and `char_for` are unchanged, and they are what a page render and
  a text readback use.
- `selected_glyph_name` is a slice index. Making it public changed no call site and no behaviour.
- Neither changes what `ToUnicode` or `LoadedFont` *holds*. That is the line the owner's rule
  draws, and it is the case where the answer would have been a question instead: an accessor that
  made a struct carry a new eagerly-built index, or that put work on a hot path, is a cost the
  viewer pays for a validator's benefit and is not ours to decide alone.

`crates/pdf-model/tests/text_extraction.rs` — this tree's readback against `pdftotext` over 974
documents — is the gate that would have shown a regression, and it did not move.

## The consequence in `pdf-archive`

`fonts/to-unicode-values-are-usable` now reads the values off instead of sweeping a guessed code
space, so it covers three- and four-byte codes and needs no probe budget.
`fonts/actual-text-covers-private-use-characters` moved off `Unchecked`: it can now say which
shown codes map into the area, and reports the case where the file states no `ActualText`
anywhere — the one case that needs no `BDC`/`EMC` span analysis to be certain of. The other half
of that clause is still owed and is named in the predicate's own documentation rather than
implied to be met. `fonts/charset-lists-every-glyph-in-the-program` and
`fonts/cidset-lists-every-cid-in-the-program` moved off `Unchecked` outright.

## What this does not license

An accessor added to a core crate for a validator's sake is still a core crate's surface, and it
is judged by what it costs *the viewer*. The test is not "is `pdf-archive` easier now"; it is
"does a page still open as fast, and does the readback still say the same thing". Where the
answer to either is unknown, the change is a question rather than an ADR.
