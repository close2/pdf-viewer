# 3749 — The seventh pass reads thirteen held files and logs four readings

Session 2262, the silent-repair campaign's census burn-down, seventh pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3574, ADR 3621, ADR 3710, ADR 3738; ISO 32000-2 §8.9.7,
§8.11.4.2, §8.11.4.4, §12.9, §12.10.4; ISO 19162 section 9.3.4; IOGP Guidance Note 7-2.

## Context

ADR 3738 left 224 sites held, the largest files `inline_image.rs` at five and eleven at four, with
`xmp.rs` (two) and `script_sites.rs` (one) held for a channel and a class. Each site was read
against the text its module cites.

## Decision

- **An inline image's `/L` that is no length** (§8.9.7: `/L` "shall be present on all inline
  images"): `scan` reads it, and a value that is no non-negative integer is noted as
  `model/inline-image-data-extent` whichever route then finds `EI`. Before, a negative `/L` whose
  §8.9.3 arithmetic happened to land on `EI` was read past in silence. The class's text and the
  converter's answer now name the case.
- **`model/optional-content-entry-unusable`** (new, §8.11.4): Table 98's required `/D` absent or no
  dictionary (read with every group on, at the file), and a group's `/Usage` or one of its
  dictionaries of another type (read as absent, at the group).
- **`model/measurement-entry-unusable`** (new, §12.9): a number format's `/D` that is no positive
  integer (read at Table 268's 100 or 16), and a point data dictionary's required `/Names` or
  `/XPTS` that is no array (read as empty).
- **`model/wkt-false-origin-absent`** (new, §12.10.4): ISO 19162 section 9.3.4 leaves a method's
  parameters to the method, and Guidance Note 7-2 lists a false easting and northing among the
  parameters that define every method `resolve` admits. A string stating either not is noted by
  `from_wkt_noting`; `Projection::forward` and `inverse` evaluate that reading, and say so.
- Each new class breaches section 5.1 alone. None is inside content; the whole-file rewrite carries
  the entry as stated and the output's own check refuses by the class's name, as for ADR 3669's
  group `/Name`.
- **Marked**, each read back: `combine.rs`'s four (a decoded buffer's sample, a conversion that
  cannot fail, a grid bound, a capacity hint); `content/image.rs`'s four (the walk ahead decides only
  which decodes start early, and the interpreter reads the same entries and logs them); `aform`'s
  three declines and ADR 1578 section 5's zero for a field stating no number; `label.rs`'s caller
  range and UAX #9 level 0 (`Paragraphs::new` is `None` exactly where every level is 0);
  `published.rs`'s compiled-in tables; `drawing.rs`'s two declines (a hinting program that does not
  start draws unhinted; an unreadable glyph count answers `false`, which silences nothing);
  `inline_image.rs`'s `u32` widening and its depth prediction (the decoder logs the depth).

## Consequences

- `HELD` 224 → 176: all thirteen named files to zero (ADRs 3750 to 3752 carry three of them).
- Not done: a TrueType program whose instructions do not start is drawn unhinted with nothing
  logged; where the program needs its interpreter to assemble a glyph that is a loss a class could
  name, and the font loader that holds the document would have to probe it.
