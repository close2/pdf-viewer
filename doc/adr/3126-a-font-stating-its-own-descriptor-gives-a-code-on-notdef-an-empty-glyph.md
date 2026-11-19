# 3126 — A font stating its own descriptor gives a code on `.notdef` an empty glyph

Session 2106, an archive round in the `lane3` worktree, beside ADR 3125. Status: accepted and
built. In `pdf-transform`: `archive/fonts.rs` (`face_widths` returns the codes on `.notdef` where
the route allows them; the descriptor route writes them), `archive/fonts/empty.rs`
(`named_proved`, `encoding_entry`, `NOTDEF_BESIDE_A_DESCRIPTOR` narrowed),
`archive/fonts/described.rs` (`same_glyph` admits an empty glyph against a fitted face). Tests:
`crates/pdf-transform/tests/archive/descriptor_empty_glyphs.rs`. Context: ADR 2458 item 5 (the
descriptor route's refusal, "not built yet"), ADRs 2147 and 2278 (a dictionary restated as the
face's kind), ADR 2769 (the fitting), `doc/questions/A48`.
Clauses: ISO 32000-2 §9.6.5.1, §9.6.5.4, §9.9 Table 124; ISO 19005-2 sections 6.2.11.4.1, 6.2.11.8;
ISO 19005-4 sections 6.2.10.4.1, 6.2.10.9.

## Context

A font that states a descriptor and no program takes the route that embeds a face into the file's
own descriptor. A code its encoding sends to `.notdef` was refused there by
`NOTDEF_BESIDE_A_DESCRIPTOR`, though the same code in a font stated by name alone had an empty
glyph since ADR 2458. Four of R08's five documents stopped on such fonts: `TimesNewRoman` and
`Arial` under `TrueType` dictionaries (Liberation faces), `TimesNewRoman,Italic` restated as
`Type1` for a Foxit face, and Bembo and New Aster under `Type1` dictionaries (Foxit faces).

## Decision

1. **Under a `glyf` face** — shipped, or a machine's `glyf` face — with the dictionary kept or
   restated as `TrueType`, ADR 3125's symbolic font; the restatement's own proof, which reads the
   non-symbolic encoding the font no longer states, gives way to ADR 3125's.
2. **Under a shipped Compact Font Format face** — the dictionary kept as `Type1` or restated as
   one — ADR 2458's construction: one empty glyph per width added to the face, named outside the
   Adobe Glyph List, and the encoding restated as `/Differences` naming every code's glyph and each
   such code's empty one. `named_proved` reads the font as written and holds every shown code to
   the face's glyph, or to a glyph drawing nothing, at its advance.
3. **An empty glyph against a fitted face.** The proof of a restated dictionary compared a code
   the window drew nothing for with the converted outline only where the face was drawn unfitted;
   a face drawn at a scale (ADR 0358) refused an empty glyph as drawn differently. It now admits
   one with no outline on both fitted branches, as it did on the unfitted one.
4. **Still refused, with the narrowed sentence**: beside a code Annex D encodes twice (ADR 2723's
   renamed encoding), under a face of the Latin-Greek-Cyrillic pack, under a machine's OpenType CFF
   face, and for a font the window drew as nothing.

## Consequences

`455746` and `210260` convert at 2b and part 4 under `as-if-printed` and pass veraPDF 1.30.3; with
ADR 3125, `150277` converts at part 4. `219789`'s remaining deferred font and the section 6.1.13
values of `150277` and `219789` are the next sentences in R08's and R03's rows.
