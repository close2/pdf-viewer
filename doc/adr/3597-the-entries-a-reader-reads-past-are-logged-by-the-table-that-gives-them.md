# 3597 — The entries a reader reads past are logged by the table that gives them

Session 2224, fix round 6 of the silent-repair campaign. Status: **accepted**. Context: ADR 3477,
ADR 3505, ADR 3521, ADR 3541, ADR 3561, ADR 3565, ADR 3574 (the census and its `HELD` ratchet),
ADR 3575 (T05); ISO 32000-2 §7.5.7, §7.7.2, §8.4.5, §8.10.2, §9.6.2.1, §9.7.5.3, §10.6.5.2,
§11.3.7.2, §12.2, §12.5.2, §12.6.4, §12.7.

## Context

Round 2218's census left 704 defaults in 159 files neither beside a class nor marked. Its helpers
read the sites as repairs in `xref.rs`, the decoders, the font metrics and maps, and `pdf-model`'s
annotations, appearances, actions, forms and viewer preferences. ADR 3575 left T05's remainder: a
halftone's unreadable `/TransferFunction`, `/CA` and `/ca` clamped, and a form's `/Resources` of the
wrong type.

## Decision

- **One class per table family, not per key.** A dictionary entry present and of a type or value
  its table does not give it, read as absent or as the nearer end of its range, is logged as the
  family's class: `model/catalog-display-entry-unusable` (Table 29's `/PageMode`, `/PageLayout`,
  `/ViewerPreferences`), `model/viewer-preference-unusable` (Table 147),
  `model/action-entry-unusable` (§12.6.4's tables), `model/annotation-entry-unusable` (Table 166's
  `/F`, `/C`, `/IC`, `/CA`, `/ca`, `/Border`, Table 168's `/W` and `/D`, Table 192's `/MK`),
  `model/field-entry-unusable` (`/AcroForm`, `/DR`, Table 234's `/TI` and `/I`, any inherited
  count). The key is in the site's comment; a validator needs the departure and the table.
- **T05's remainder**: `model/form-resources-not-a-dictionary`, `model/alpha-constant-out-of-range`
  (§11.3.7.2: "All of the shape and opacity inputs shall have values in the range 0.0 to 1.0
  (inclusive)"), `model/halftone-unreadable` and `model/halftone-transfer-function-unreadable`,
  logged at the content site of the stream that set them, as ADR 3575's are. `font/wmode-unusable`
  for Table 118's `/WMode`; a `/Widths` with no integer `/FirstChar` is `font/widths-malformed`; an
  object stream's `/N`, `/First` or header read past while enumerating members is
  `syntax/object-stream-header`, as when it is expanded.
- **What is not a repair** stays as written: an absent entry's default is the table's, and
  Table 147's `/PrintScaling` reads "If this entry has an unrecognised value, AppDefault shall be
  used", so no class is logged there.
- **Placement.** Catalog entries at the catalog's object; the action, annotation and field readers
  are handed dictionaries, not objects, so their classes are placed at the file.
- **Cost.** Every recording sits on a branch a conforming file does not take, behind a test the
  reader already made or one `is_null`; the action, annotation and field recorders are `#[cold]`
  (ADR 3561).
- **Mapping and the converter.** All ten classes are section 5.1's alone. The six outside content
  are carried by the whole-file rewrite as the file states them, so each is left to the output's
  own check, which refuses by the class's name (ADR 3521's second bullet). The four T05 classes
  are logged at `Site::Content` and, like ADR 3575's, reach the log only where the interpreter ran;
  where they do, the page is photographed under `as-if-printed` (ADR 3557) and refused under the
  fence by the general sentence elsewhere.

## Consequences

- The census's held sites fall from 704 to the count `repair_census.rs`'s `HELD` table sums.
- Left: `sfnt::horizontal_metric`'s truncated `hmtx` (its callers hold no document), the
  surrogate in `text_string` and `tounicode`, the image unpackers' short data, `forms_data`,
  `appearance.rs`'s remaining entries, and an `Indexed` sample past `hival`, which §8.6.6.3 says
  "shall be adjusted to the nearest value within that range" and `image/unpack.rs` paints black: a
  pixel change, owed its own round.
