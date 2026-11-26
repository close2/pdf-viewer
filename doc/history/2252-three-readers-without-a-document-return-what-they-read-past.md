# 2252 — Three readers without a document return what they read past, or decline

Lane `gwg`, the silent-repair census burn-down, fifth pass (ADRs 3574, 3621, 3691). ADRs 3709 and
3710.

- **The three readers** (ADR 3709). `markup::parse_noting` returns `model/rich-text-malformed`
  (§12.7.4.3: a `/RV` or `/RC` that is not XML) and `model/rich-text-attribute-unusable` (an `ol
  start`, `li value` or `xfa:spec` read as absent); `for_field` and `for_free_text` record them, a
  `/V` that only looks like markup records the second alone, an imported `/RV` nothing.
  `system::from_wkt_noting` returns `model/wkt-name-absent`, `model/wkt-identifier-unusable` and
  `model/wkt-auxiliary-sphere-type-absent` (§12.10.3, §12.10.4); `measurement.rs` records them
  where it reads `/WKT`. A nameless unit is no longer called `metre`. All five map to section 5.1
  alone; the converter carries each string as stated and the output's own check names it.
- **JPEG 2000 declines**, by the module's own contract: a short `cref` or `creg` box is
  `HeaderError::ShortBox`; `Collection::count` is `Option<u16>` (a public type: fuzz checked).
- **Two defects found by reading**: a Latin, Greek or Roman list tag outside its range had no
  tag (ADR 1660's fallback is `decimal`; `ListStyle::tag` returns a `String`); an absent XFA
  `occur max` was read as 1 rather than as `min`, calling `<occur min="3"/>` dynamic.
- **Next ten files** (ADR 3710): two Sonnet helpers marked declines, bounds and writers; every
  marker was read back here and one withdrawn (an absent event `activity`: the syntax lists `click`
  first). `mask.rs` records `image-depth-absent` on the colour-key and grid routes; a stored
  appearance with no `/BBox` records `form-bbox-unusable`; `collection::extract` names
  `COLLECTION_CUT_TO_ONE_FACE`.
- `HELD` 360 → 287.
- Left, in order: `cff_stretch.rs` 4 (an FDSelect that places no glyph, no `/FDArray` offset),
  `xfa/template.rs` 5 (the template reader holds no document; `activity`'s default to read),
  `rich_text/layout.rs` 3, `collection.rs` 3, `annotation.rs` 2 (ADR 3710 lists each); then
  `type3.rs` 7, left to the lane editing Type 3 drawing, and the next held files.
- **Census**, with and without the pack: no line names a new class or a short JPEG 2000 box, and
  the conforming counts are 2247's; the other lines differ from 2247's by the rounds merged since
  (fewer `the-fence` refusals on undefined operators). veraPDF 1.30.3 over the 5523 outputs
  written as conforming: 40 not compliant and 12 unfinished, as round 2247's.
- `tools/state.sh --round 2252 launch`: 6 documents, 36 figures banded, 0 outside.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6273 of 6276, the three failures the manifest's
§10.5 audit entries (`audit-diff.py`: "failed 3, expected 3"); rustdoc of `pdf-model`, `pdf-font`
and `pdf-archive` 0; fuzz 0; `cargo test -p conformance` 0.
