# 3429 — A symbolic TrueType font's ignored /Encoding is a recorded recovery, below the code as a character

Session 2182. Status: accepted. Context: audit 2180 (B02, D04).

## Decision

1. **No step of §9.6.5.4 reads a symbolic font's `/Encoding`.** The clause: "When the font has no
   Encoding entry, or the font descriptor's Symbolic flag is set (in which case the Encoding entry
   is ignored), this shall occur" — the (3, 0) and (1, 0) steps. `truetype_code_table` builds the
   glyph-name table for the clause's second route from nothing for such a font, so no
   `/Differences` name decides a glyph above the reader-chosen tiers.
2. **Below the code offered as a character, and for a symbolic font only, the ignored entry's
   name is carried through the (3, 1) subtable, the (1, 0) subtable or `post`**, as the second
   route carries a nonsymbolic font's. The clause ends "a PDF processor may supply a mapping of
   its choosing", and this is one: a font the producer flagged symbolic and named glyphs for
   anyway (a `/Differences` naming code 1 `/A` over a (3, 1)-only program) is not drawn blank.
   Every code it answers is in `reader_chosen`, so ISO 19005's 6.2.11.6 sees it.
3. **The names a symbolic font keeps for §9.10.2's second method are exactly the ones step 2
   selected a glyph by** — "if the font is a simple font and the glyph selection algorithm (see
   9.6.5) uses a glyph name". Every other slot is empty, so text extraction and
   `selected_glyph_name` do not read an entry the selection did not use.
4. **The `UnsupportedEncoding` arm is gone.** `base_encoding` answers no base encoding for a name
   Table 109 does not permit, and every permitted name has a table, so the arm could not fire; a
   nonsymbolic font propagates the error, and a symbolic font, whose entry the clause ignores,
   loses only the recovery of step 2 if it ever did.

## Rejected

- **No recovery at all.** Spec-pure, and it draws `.notdef` for every code a symbolic font named
  through `/Differences` at a position that is no character in its (3, 1) subtable — a shape
  common enough that the processor's choice is better spent on it than on nothing.
- **The recovery above the code as a character.** That is the pre-2182 behaviour the audit
  found: an ignored entry deciding a glyph the clause's own fallback would decide otherwise.
