# 2773 — The machine's face the window drew is embedded where its licence permits

Session 1981, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts.rs` (`FaceAuthority::Machine`,
`Face::Machine`, `machine_face`, `machine_face_named`, `face_for`'s new argument, `row_for`),
`archive/fonts/described.rs` (`restated_kind`'s `machine` argument, `machine_face_proved`,
`held_descriptor`).
Tests: `crates/pdf-transform/tests/archive/machine_face.rs` (two), and
`tests/archive.rs`'s `the_unrestated_route_over_a_machines_face_is_the_loss_machine_face`.
Context: ADR 0270 (the window widens to a machine's face), ADR 2162 and ADR 2279 (the loss
`machine-face`), ADR 2490 and `doc/pdfa-blocking-issues.md` R09 (a), ADR 2769 (the same shape for
a shipped face's fitting), ADR 1425 and ADR 2548 (`pdf_font::embed`'s reading of `fsType`). ADR
2774 says which profile takes it.
Clauses: ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1; ISO 32000-2 §9.9.1,
§9.9's Table 124, §9.6.5; ISO/IEC 14496-22's `OS/2` table (`fsType`).

## Context

Both parts require the program of every rendered font in the file, and admit only a program that
may legally be embedded for unlimited, universal rendering. Where a file names a font it does not
embed, the window draws a face it ships, or, where the file's encoding names characters that face
lacks or the file names a face this machine has installed, a face of this machine's (ADR 0270). The
conversion embedded the shipped face in every case, and where the window had drawn a machine's face
that is the loss `machine-face`, which `as-shown` withholds. After round 1979 the shipped census
refused 80 documents at each part under `as-shown` on R09, and `machine-face` is in every one.

On this VM the faces involved are, by a probe of each rendered font's `substitute_program`: for
simple fonts, URW's `NimbusRoman-*`, `NimbusSans-*`, `NimbusSansNarrow-*` and `NimbusMonoPS-*`
(OpenType with a name-keyed `CFF ` table, `fsType` 0x0004, preview-and-print); for composite
fonts, `DroidSansFallback` (`glyf`, `fsType` 0x0008, editable) and three Nimbus faces. Installed
but not drawn by any of the 76 documents: DejaVu and Noto Mono (`fsType` 0). No collection is drawn.

## Decision

1. **The face embedded is the face drawn, by its bytes.** `machine_face` takes the program from
   the font as the window loaded it (`LoadedFont::substitute_program`), never from a second search
   of the machine, and builds its code table by the route the window built its own
   (`supplied_face` and the window both take `substitute_code_table`). A collection is not taken.
2. **The licence is the face's own `fsType`, and only that.** Installable, editable and
   preview-and-print embedding are admitted; restricted-licence embedding and a bitmap-only face are
   not (`pdf_font::embed::may_be_embedded`). Preview-and-print is admitted because §9.9.1 names
   exactly that permission — "A font program may allow embedding for the sole purpose of viewing
   and printing the document but not for creating new or modified text that uses the font" — and
   makes it the use every embedded program gets "[i]n the absence of explicit information to the
   contrary". Rendering is viewing and printing, on any system and without limit, which is what the
   PDF/A sentence asks. A face refused here leaves the shipped face and the loss `machine-face` by
   name, so `as-shown` keeps its stop on it.
3. **The routes are the shipped face's, with the machine's program.** Table 124 decides the key:
   an OpenType face carrying a name-keyed `CFF ` table goes under a `Type1` dictionary as
   `/FontFile3 /OpenType`, so a `TrueType` dictionary drawn from a Nimbus face is restated as
   `Type1` exactly as ADR 2147 restates one given a shipped CFF face, its codes' names stated where
   it had no encoding. Advances are restated to the file's `/Widths` where they differ (section
   6.2.11.5), as for any face. A fitted machine face keeps the fitting only where it is `glyf`
   (ADR 2769's writer); a fitted CFF face names `substitute-stretch`, as a shipped one does.
4. **Proved before it is written.** On both routes the converted font is written into a document of
   its own, loaded, and held code by code to the window's outline and advance (`proved`); a code the
   embedded font would reach another glyph for, by §9.6.5's route into the embedded program rather
   than the substitute's, is refused by name.
5. **The face is embedded whole.** §9.9.2 makes a subset optional, and neither part requires one.
   A name-keyed CFF subset that keeps glyph indices needs a compacting CFF writer this tree does not
   have (`cff::with_charstrings` appends and leaves the old INDEX inert). The cost: one Nimbus face
   is 82–106 KB before Flate. Named as what is left of R09.
6. The report names the face, its PostScript name and its `fsType` permission, as
   `FaceAuthority::Machine` ("machine"), and the file's `xmpMM:History` says whose face it was.

## Consequences

`as-shown` converts the documents whose only refusal was `machine-face` on a simple font. **A
different machine converts differently**: which face the window draws is a fact of the machine
(ADR 0270), so the output of `as-shown` is the page as the converting machine showed it, and a
census on a machine with other faces counts other documents. That is what `as-shown` means, not a
defect. On this VM the shipped census frees 25 documents at 2b and 24 at 4 under `as-shown`, and 19
at each part under `no-data-loss`; R09's row goes from 80 to 58. What is left of R09 (a): 25
documents whose Nimbus face states the width of a shown glyph inside a subroutine, which
`pdf_font::restate` does not rewrite (9 of NimbusSans-Regular's 855 glyphs, 12 of
NimbusRoman-Bold's), refused by name; a fitted machine CFF face, now named `substitute-stretch`; the
standard fonts stated by name alone (`described`) and the composite routes (`rekeyed`, the CJK pack),
which still embed a shipped or pack face over the machine's and keep the loss; and the subset.
