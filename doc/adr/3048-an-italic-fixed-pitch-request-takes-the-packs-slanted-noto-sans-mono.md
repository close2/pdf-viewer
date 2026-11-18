# 3048 — An italic fixed-pitch request takes the pack's slanted Noto Sans Mono

Session 2087. Status: **accepted and built.** In `tools/lgc-pack.py`: `FACES`' slant and `build`'s shear,
`PACK_VERSION` 3; in `pdf-transform`: `LgcFace::MonoOblique` and `MonoBoldOblique`, their digests, and
`for_request`. Context: ADRs 2630 and 2934 (the pack; 2934 section 3 left this unanswered), 2938 (the fourteen stated
by name alone take it), 2162 (`machine-face`), 1441 (a machine face's distance puts slope before width). Clauses: ISO
32000-2 §9.8.2 (Table 121); the standard fourteen's published metrics (`data/standard-fonts/afm`).

## Context

A `Courier-Oblique` showing a letter the shipped fixed face lacks (Greek, Cyrillic, Latin Extended-A) had its page
photographed: the pack holds Noto Sans Mono, which google/fonts publishes in no italic, and `for_request` answered an
italic fixed-pitch request with nothing.

## What the text asks, and what it leaves

Table 121's `Italic` flag: "Glyphs have dominant vertical strokes that are slanted", and `FixedPitch`: "All glyphs
have the same width". The standard names no face for either; the pack's face is already a stand-in under the loss
`machine-face`. An upright face does not meet the `Italic` flag, so answering with it would state a font the
descriptor's own flag contradicts.

## The options

- **The upright face** (ADR 1441's nearest face): contradicts the flag, as above. Not taken.
- **Another family's italic monospace** (Source Code Pro, IBM Plex Mono, Cousine): a second design inside one
  pack, a second notice, a new input to fetch and hold by digest. Not needed, so not taken.
- **The text matrix**, as the brief asked of other synthesised styles: no style is synthesised anywhere in this
  tree (the viewer picks the nearest installed face, ADR 1441), and the archive edits no content stream (ADR 0816),
  so there is no text matrix for it to set.
- **Taken: a slanted face the pack's script makes.** The upright instance's outlines are sheared, each point moved
  right by its height times tan 12°, advances unchanged, the CFF `ItalicAngle` −12 and `FontBBox` the sheared box.
  Twelve degrees is the `ItalicAngle` that `Courier-Oblique.afm` and `Courier-BoldOblique.afm` state for the
  standard fixed-pitch obliques, the faces such a request most often names. It is the same family, licence and
  notice (the OFL reserves no name for Noto, and permits Modified Versions); the report names it as "slanted by this
  program", because no designer drew it.

The owner is not needed: no font file enters the repository, the script builds and CI publishes the pack as before,
and the two new digests are carried in `lgc_pack.rs`. A `Courier-Oblique` page is drawn as text in a face nobody
published, where it used to be a photograph — the choice this ADR records.

## Proof

Two builds wrote the same bytes, and the six existing faces kept their digests. `lgc_pack.rs`'s
`a_built_pack_holds_the_programs_whose_digests_this_file_carries` passes against the build;
`tests/archive/lgc_pack.rs`'s `an_italic_fixed_pitch_request_takes_the_packs_slanted_mono_face` converts a
`Courier-Oblique` and a `Courier-BoldOblique` stated by name alone showing a-ogonek, n-acute and z-dot-above, each
code reaching the slanted face's own glyph at 600; veraPDF 1.30.3 passes both at 2b. Planted: `for_request` giving
`None` for an italic fixed request again fails it and the unit test.
