# 2789 — A CFF face is embedded with the fitting the window drew it at

Session 1989, a single archive round.
Status: accepted and built. In `pdf-font`: `cff_stretch.rs` (`with_outlines_scaled`, `Restatement`),
`stretch_cff.rs` (`cff_outlines`, `drawn_alike`) and `StretchError::NotRestated`. In `pdf-transform`: `archive/fonts/described.rs`
(`with_the_fitting` chooses the writer by outline format, `machine_face_proved` holds a written
fitting, `same_once_written` admits the line the writer does not state) and `archive/fonts.rs`.
Tests: `cff_stretch_tests.rs` (eight), `stretch_cff_tests.rs` (four), and
`tests/archive/machine_face.rs`'s `a_machines_cff_face_fitted_horizontally_is_embedded_with_its_fitting`.
Context: ADR 2769 (the same construction for a `glyf` face), ADR 2770 (which profile takes it),
ADR 2773 and 2774 (the machine's face embedded), ADR 2779 (a charstring run as the interpreter
runs it), ADR 0358 (the window's fitting), `doc/pdfa-blocking-issues.md` R09.
Clauses: ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.5, ISO 19005-4 sections 6.2.10.4.1 and
6.2.10.5; ISO 32000-2 §9.2.4, §9.6.2.1; Adobe Technical Note #5177 sections 2 (edge hints), 3.1,
3.2, 4.1 to 4.3, 4.7, Appendices A and B.

## Context

Where a file names a font it does not embed and this machine has a face of that name, the window
draws that face (ADR 0270), and where the file's widths are narrower it scales the outlines in x
(ADR 0358). Under `as-shown` the machine's face goes in (ADR 2773), but the fitting did not: ADR
2769's writer reads `glyf` points, and the URW faces are OpenType with a `CFF ` table. So the font
stopped on `substitute-stretch` — 18 documents at each part in round 1984's census. Run under every
profile at both parts, 14 of the 18 refused on that alone under `as-shown`; the other 4 (020747,
064034, 492484, 504076) refuse on content-stream and string rows first.

## Decision

1. **Each charstring is run as the interpreter runs it and stated again without calls.** An
   operand inside a shared subroutine can be an x for one caller and a y for another, so no
   in-place edit is right. Each point is taken absolutely, its x scaled about the origin and
   rounded to a whole unit, its y kept; deltas are stated between rounded points. `vstem` edges
   are scaled, an edge hint's −20 or −21 kept; `hstem` and every mask byte copied; `endchar`'s
   `adx` scaled; the four flex forms stated as `flex` with their depth; the width operand copied.
   An arithmetic, storage or reserved operator is refused by glyph, and the face then goes in
   unmodified with the loss named, as before.
2. **Whole units, and a line that rounds to no length is not stated.** This tree's reader draws a
   CFF coordinate at no pixel size as its whole part, so a 16.16 fraction would be read as its
   floor; a whole unit is read as written. Two points one unit apart can round to one point; the
   reader drops such a line, so the writer does not state it, and both proofs skip exactly that
   line: one no longer than the tolerance that does not move y.
3. **The OpenType wrapper follows.** Each left side bearing is scaled; `head`'s box and `hhea`'s
   extremes are recomputed from the outlines read back. Advances are copied, and
   `restate::with_widths` runs after the scaling as for an unscaled face. A bare CFF program, a
   `FontMatrix` other than the default, and `CFF2` are refused.
4. **Proved before it is written**: every glyph read back through the OpenType reader, the same
   commands, each y equal, each x within one unit of the em; every charstring width equal. Then
   the conversion's own proof holds each shown code to the window's outline.
5. **The profile choice is ADR 2770's**, not reopened: written where `substitute-stretch` is
   withheld, the face unmodified with the loss named where it is authorised.

## Consequences

The 14 documents convert under `as-shown` at both parts. All 35 URW faces take the fitting at
every glyph. The cost is size: the flattened charstrings are appended beside the original
`CharStrings` INDEX and subroutines, which stay in the table, about 55 to 85 KB per fitted face.
The compacting CFF writer R09 already lists for a subset would remove that.
