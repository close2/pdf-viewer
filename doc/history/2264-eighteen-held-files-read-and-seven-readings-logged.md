# 2264 — Eighteen held files read, and seven readings logged

Lane `gwg`, the silent-repair census burn-down, eighth pass (ADRs 3574, 3621, 3749, 3750). ADR
3757.

- **New classes**, each section 5.1 alone, answered by the whole-file rewrite and the output's own
  check: `model/collection-entry-unusable` (Table 158's `/Position`, Table 159's `/ID`),
  `model/structure-attribute-unusable` (a span below 1, a `/WritingMode` Table 378 does not name),
  `model/box-colour-info-unusable` (what `guidelines::of` already said), `model/soft-mask-in-data-unusable`
  (no integer; §11.6.4.3 makes any other integer the mask's), `model/xfa-locale-name-absent`
  (page 954's "must be supplied", returned by `LocaleSet::read` and logged at the stream).
- **On existing classes**: a popup's `/Parent` that is no dictionary and a `/C` past 1.0
  (`annotation-entry-unusable`); a `/DA` `Tf` with no size, auto-sized (`field-entry-unusable`,
  through `DefaultAppearance::size_read` in all three layouts).
- **A defect**: an XFA date, time or number pattern stating no `name` was kept under the empty
  name and never found; pages 947, 961 and 965 make it `med`.
- **Marked**, each read against its source as written (no helper wrote one): catalog declines,
  §8.4.1's cap and join clipping, §8.9.5.2's nearest allowed value, XFA pages 527, 759 and 153,
  the banded JPEG declines, `pdf-font`'s installed and stand-in faces, the run table's indices.
- **Stretch**: `pdf_model::page_annots` is public; `pdf-archive`'s survey, reach and examination
  and three `viewer-core` readers read a page's `/Annots` through it.
- `HELD` 176 → 122: the eighteen named files to zero.
- Left, in order: the next held files from `repair_census.rs`; `pdf-transform`'s `/Annots` readers;
  a negative `/DA` size auto-sized without a class; `unescape`'s answer for rich text, XFDF, XFA.
- `tools/state.sh --round 2264 launch`: 36 figures banded, 2 outside, both main's below their
  floors since `3b5320ea` — `PDF20_AN001-BPC` `peak_anon_mib` 5.785 (7.0 .. 11.0), WTPDF 8.645
  (10.5 .. 14.0). No per-object check was added on a hot path: `content.rs`, `runs.rs`,
  `shaping/face.rs` and `restart.rs` changed comments only, so no instruction count was taken.
- Census with and without the pack: each run's twelve summary lines are 2262's, and no line names
  a class this round logs. veraPDF 1.30.3 over the 5523 outputs written as conforming: 40 not
  compliant, 12 unfinished, as 2262's.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-core -p viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 6993 of 6996 — the
manifest's three entries for these crates (two §10.5, one of Annex D, lane 3's); rustdoc of the
five library crates touched 0; fuzz 0; `cargo test -p conformance` 0.
