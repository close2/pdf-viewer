# 2259 — A Type 3 font reads as every simple font, and an XFA event fires on click

Lane `gwg`, the silent-repair census burn-down, sixth pass (ADRs 3574, 3621, 3709, 3710). ADRs
3737 and 3738.

- **`type3.rs`** (ADR 3737): Table 110's `/FirstChar`, `/LastChar` and `/Widths` are read in one
  place and logged once as `font/widths-malformed` where not as stated, a short array included;
  `/Differences` logs `font/differences-malformed`; `/ToUnicode` is read by `pdf_font::to_unicode`
  (now public), so `/UseCMap` is followed and its three classes logged. Defect: a code past one
  byte took code 0's name's text.
- **`xfa/template.rs`** (ADR 3738): `Template::read_noting` returns
  `model/xfa-template-attribute-unusable` (§12.7.3; an `occur` bound or digits limit that is no
  integer, or below `-1`), recorded by `xfa.rs` at the packet; section 5.1 alone, answered by the
  rewrite. Defect: page 703 underlines `click` (read in the PDF; the markdown drops underlines),
  page 597 makes underlined the default, and an event with no `activity` was never raised.
- **Logged on existing classes**: a choice field's unusable `/I` and a non-dictionary widget
  `/Parent` (`field-entry-unusable`); an annotation `/P` naming no page, now read as absent so the
  `/Annots` walk finds the page instead of answering page one (`annotation-entry-unusable`).
- **Marked**, each read back: public-key `/V` (a first marker said s3 and s4 refuse a 0; they read
  `/V` 1's key, so it is a comment naming `ENCRYPTION_ENTRY`) and `/CF` declines; bounds in
  `unpack.rs`, `bidi.rs`, `joining.rs`; writers in `type1_to_cff.rs`; `cff_reals.rs`'s declines;
  `stretch_cff.rs` and `cff_stretch.rs`, which rewrite a face this program ships.
- `HELD` 287 → 224. Held: `xmp.rs` 2 (a numeric character reference that does not parse, kept
  verbatim by `unescape`), `script_sites.rs` 1 (a page `/Annots` that is no array).
- Left, in order: `inline_image.rs` 5 (round 2258), the eleven files at 4 from
  `optional_content.rs`; an XMP channel for `unescape`; a class for a page `/Annots` no array.
- `tools/state.sh --round 2259 launch`, twice: 36 figures banded, 2 outside, both below their
  floors — `PDF20_AN001-BPC` `peak_anon_mib` 5.770 and 4.879 (7.0 .. 11.0), WTPDF 8.672 and 9.023
  (10.5 .. 14.0). Main's post-merge train at `3b5320ea` reads the same two (5.695, 9.285), and at
  `1986ad69` none, so the fall is main's, not this round's; nothing here runs per glyph.
- Census with and without the pack: each run's twelve summary lines are 2252's, and no line names
  a class this round logs. veraPDF 1.30.3 over the 5523 outputs written as conforming: 40 not
  compliant, 12 unfinished, as 2252's.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6292 of 6296 — the manifest's two §10.5 audit
entries and two of this round's (a blockquote cited under §9.6.5.3, this record's missing gates),
both fixed and passing under `cargo test -p conformance`; rustdoc of the four reader crates 0;
fuzz 0; `cargo test -p conformance` 0.
