# 1281 — The baseline's last two parts, a Flate copy in the stated space, and issue1905's cause

Batch forty-three. Row §7.4.9 stays `partial`, its residue re-stated with the texts' availability.
ADRs 1399, 1400.

## Built
- **M.9.2.6 and M.9.2.7** (ADR 1399): the header reader records box spans, the JP2 Header box's
  contents, the first Codestream Header and Compositing Layer Header boxes' contents (Colour Group
  children and Cross-Reference `Rtyp`s included) and each Fragment List's holder. The validator
  reports a first-layer cross-referenced fragment not wholly before its codestream's data, and a
  box type the JP2 Header holds found again in the first layer's headers. 1273's Colour-Group
  fixture now draws the M.9.2.7 finding. `tests/jpx_baseline.rs` 13 → 16 tests.
- **Device colour in the codestream** (ADR 1399): the survey records the device space an image
  stating no `ColorSpace` effectively uses (enumerated CMYK; no obliged specification → channel
  count, §7.4.9), and the six 6.2.4.3 rows judge it; the row is delegated like its sibling.
- **Transcode remedy** (ADR 1400): `archive/transcode.rs`, `Rewrite::Jpeg2000TranscodedToFlate`,
  in-place `preserve`, shapes `stated-colour-space` / `data-colour-space`, report rows,
  `--remedy-sites` sentence, keep-everything profile row, mitigations 4.5. Fixture: a CalRGB image
  with an off-list `colr` fails the rule alone, is refused unconfigured, converts, validates clean,
  and the Flate samples equal the source's decoded ones.

## issue1905 (helper agent, scratch exports, separate target dirs, md5-distinct binaries)
Cause: ADR 1389's `Encoder::compute_takes` (`raster/crates/raster-gpu/src/encode/fill.rs`) sends
fills that wind more than two values to the scratch lane; the page's 29 large fills left the
compute lane, whose accumulator charged ~4 more bytes a pixel. 0c9f97ff refused at 272158852;
2f571015 draws at 252594693 of 268435456; 2f571015 with that one condition reverted refuses
byte-identically (whole page 527497181 with the budget check off). Sentence edited into
`crates/render-raster/tests/corpus.rs` (1280's file, one paragraph).

## Texts (2026-09-29)
PIMA 7667 sold (IS&T USD 25, ANSI); CIE 131 sold, superseded by CIE 159; a TC draft is not the
text. Non-D50 Lab needs ITU-T T.4 Annex E (free), not yet read.

## Left
SMaskInData ≠ 0 under the transcode; re-classing bit-depth/channel-count/CIE Jab for the stated
shape; T.4 Annex E; the thirteen one-level-off codestreams.
