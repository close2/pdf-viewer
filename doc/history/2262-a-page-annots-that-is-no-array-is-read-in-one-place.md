# 2262 — A page /Annots that is no array is read in one place

Lane `gwg`, the silent-repair census burn-down, seventh pass (ADRs 3574, 3621, 3710, 3738). ADRs
3749 to 3752.

- **New classes**, each section 5.1 alone, answered by the rewrite and the output's own check:
  `model/page-annots-not-an-array` (ADR 3750; `annotation::page_annots` is now the one read of a
  page's `/Annots`, and the 33 readers in `pdf-model` go through it),
  `model/file-specification-entry-unusable` (ADR 3751; Table 43's `/F` where nothing names the
  file, Table 44's `/Params`), `model/xmp-reference-undefined` (ADR 3752; `unescape_noting`'s
  answer, logged by `Xmp::read` at the stream), `model/optional-content-entry-unusable`,
  `model/measurement-entry-unusable` and `model/wkt-false-origin-absent` (ADR 3749).
- **On an existing class**: an inline image `/L` that is no non-negative integer is noted as
  `model/inline-image-data-extent` even where §8.9.3's arithmetic then finds `EI`; before, it was
  read past in silence.
- **Marked**, each read back against its source: `combine.rs`, `content/image.rs`'s walk ahead,
  `aform`, `label.rs`, `published.rs`, `drawing.rs`, two of `inline_image.rs`. One reading was
  wider than the brief named: `unescape` also kept a bare ampersand with no `;`, now noted too.
- `HELD` 224 → 176: the thirteen named files to zero.
- Left, in order: the next held files from `repair_census.rs`; `/Annots` readers outside
  `pdf-model`; `unescape`'s answer for rich text, XFDF and XFA; a TrueType program whose
  instructions do not start, drawn unhinted unlogged.
- `tools/state.sh --round 2262 launch`: 36 figures banded, 2 outside, both main's below their
  floors since `3b5320ea` — `PDF20_AN001-BPC` `peak_anon_mib` 5.102 (7.0 .. 11.0), WTPDF 9.238
  (10.5 .. 14.0). No per-object check was added on a hot path: `content/image.rs` and
  `drawing.rs` changed comments only, `optional_content.rs` reads once per document and group,
  and the inline image's `/L` read moved from `data_extent` to `scan` rather than being added;
  so no instruction count was taken.
- Census with and without the pack: each run's twelve summary lines are 2259's, and no line names
  a class this round adds. veraPDF 1.30.3 over the 5523 outputs written as conforming: 40 not
  compliant, 12 unfinished, as 2259's.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6306 of 6312 — the manifest's five entries for these
crates (two §10.5, three of Annex D and F, lane 3's) and this record before its gates were written;
rustdoc of the four reader crates 0; fuzz 0; `cargo test -p conformance` 0.
