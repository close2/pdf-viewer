# Round 2241 — audit 2239's fixes: a decimal carries, a slope turns with its axes, scripts are met

A fix round in the `audit` worktree for review 2239's four findings (§12.9, §12.11).

## What changed

- B01: the `/F` `D` arm of `measurement::fractional_text` now rounds once to `/D`'s precision, whole
  part included, and only then splits the value. So 2.999 to hundredths is `3`, or `3.00` with `/FD`.
- B02: `number_format` reads `/RD`'s empty string as the period, by `/RD`'s own sentence: "An empty
  string indicates that the default shall be used". It reads `/RT`, `/PS` and `/SS` by theirs:
  "An empty string indicates that no text shall be added".
- B03: `Viewport::along_measuring_axes` reverses each axis that Table 265's `/BBox` states high first.
  For an object's own measure it uses the drawn unit square's edges instead (ADR 3665).
  `Viewports::distance` and `traced` now measure oriented points. Only the slope is signed;
  distance, length, area and angle do not change under a reversal.
- D01: `Kind::unmet` is restated. `EnableJavaScripts` is met, because actions and the
  document-level tree run at the reader's level (ADR 3666). `Multimedia` stays unmet, since a
  form `XObject` clip is not presented. `Geospatial2D` stays unmet, since a system given only by
  its EPSG code, or by a method not carried, is refused. `Geospatial3D`, `RichMedia` and the 3D
  types each have their own reason. The penalty fixtures in `requirements.rs` and in two
  `viewer-core` tests now price `Multimedia`.
- The three audit tests pass. Their entries are removed from `expected-failures-current.json`, and
  2239's four findings are marked fixed.

## Ledger

`12.9.1`, `12.9.2` and `12.11.2` are `implemented` before and after. Their notes now state the
axes, the carry, `/RD` and the restated reasons, and the stale "eight met, seventeen not" is gone.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-host -p viewer-qt`: fmt 0 and clippy
`-D warnings --all-targets` 0 (first 1 and 101: a line break, an unboxed `Rectilinear`);
`pdf-sandbox` 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 100, 4230 run, 3 failed, exactly
the manifest's 2237 three (`audit-diff.py`: "failed 3, expected 3"; three `viewer-core` fixtures
pricing `EnableJavaScripts` failed first and now price `Multimedia`); rustdoc of the four 0;
conformance 0 (earlier: two unmarked default sites, this record). Fuzz not owed.

## Left

`pdf-model/src/media.rs`'s module comment still says nothing is played (stale against A946, A1006).
