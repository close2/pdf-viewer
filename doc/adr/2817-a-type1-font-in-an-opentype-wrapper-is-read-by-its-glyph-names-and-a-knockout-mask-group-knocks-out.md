# 2817 — A Type 1 font in an OpenType wrapper is read by its glyph names, and a knockout mask group knocks out

Round 2000, the audit-fix round (group MC, model core).
Status: **accepted and built** in `pdf-font` (`loading/opening.rs`: `type1_cff_in_opentype`) and `pdf-model`
(`content/transparency/soft_mask.rs`: `knock_out_mask_group`).
It answers two findings of `doc/reviews/1983-specification-audit.md` under the owner's instruction of 2026-11-11 to fix
all of them. Qualifies ADR 2508's `knockout: false` boundary; leaves ADR 0015 and ADR 0050 standing within the scopes
the audit found them justified in.
Clauses: ISO 32000-2 §9.6.5.2, §9.6.5.4, §9.9.1, §11.4.6, §11.5.3, §11.6.5.1.

## 1. §9.9.1 picks the route by the font dictionary and the table, not by the wrapper

§9.9.1 lists "[t]he process of finding glyph descriptions in OpenType fonts" by case, and its first bullet is "For Type
1 fonts using "CFF " tables, the process shall be as described in 9.6.5.2, "Encodings for Type 1 fonts"". Every
`/FontFile3 /Subtype /OpenType` program reached `truetype_code_table`, which is §9.6.5.4's — the list's second bullet,
"For TrueType fonts using "glyf" tables". A `Type1` (or `MMType1`) font dictionary over an `OTTO` whose `CFF ` table is
name-keyed now takes the bare-CFF route: the `/Encoding`'s glyph names looked up in the CFF charset, `.notdef` the
program's own, and the glyph index that answers is the one the wrapper draws. The witness is
`opentype_selection_audit`'s: a `post` of version 3 with no names, a Macintosh `cmap` that offers the other glyph at the
same byte, and a `two.sups` that suffix-stripping would turn into `two`.

ADR 0015 (the TrueType route) and ADR 0050 (exact-name lookup) are unchanged: each still governs the font it was
written for, and neither said the TrueType route was right for a Type 1 font.

## 2. A soft mask's knockout group knocks out where a painted one would

§11.5.3, of the group a mask is derived from: "G may be any kind of group - isolated or not, knockout or not - producing
various effects on the C result in each case." ADR 2508 kept `knockout: false` for every mask group because
`pdf_render::SoftMask` carries no knockout flag, and reported the group. The flag is not needed on the mask: its list
can hold a `Command::Group` that carries it, which every backend already composites. So a knockout mask group is put
through `knockout_construction`, the painted group's own, and where that draws the elements on transparency — an
isolated group, or one whose elements all paint Normal, whose backdrop then cancels (§11.4.4 NOTE 3) — the mask's list
becomes one isolated knockout `Command::Group` holding the result, and the report is not made.

Three cases keep the report, each because a mask has nothing to hand the construction: the own-backdrop construction
(a non-isolated group whose elements blend; the mask's backdrop is `/BC`, which ADR 2508 states only for a
non-knockout group), the construction that moves a blend mode to the `Do` (a mask has no `Do`), and a mask whose group
composites in four components, whose black half would need the same rewrite beside it.
