# 3757 — The eighth pass reads eighteen held files and logs seven readings

Session 2264, the silent-repair campaign's census burn-down, eighth pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3574, ADR 3621, ADR 3749, ADR 3750; ISO 32000-2 §8.9.5.1,
§8.9.5.2, §11.6.4.3, §12.3.5, §12.5.2, §12.5.6.14, §12.7.4.3, §14.8.5, §14.11.2.2; XFA 3.3
chapters 17 and 19.

## Context

ADR 3749 left 176 sites held, eighteen files at three each. Each site was read against the text
its module cites.

## Decision

- **New classes**, each section 5.1 alone and outside content, so the whole-file rewrite carries the
  entry as stated and the output's own check refuses by the class's name (as ADR 3669's):
  - `model/collection-entry-unusable` (§12.3.5): Table 158's `/Position` outside 0 to 100 (read at
    the nearer end) or of another type (absent), and Table 159's required folder `/ID` that is no
    non-negative integer (read as 0).
  - `model/structure-attribute-unusable` (§14.8.5): a stated `/RowSpan` or `/ColSpan` that is no
    integer of at least 1 (read as 1), a `/WritingMode` none of Table 378's names (read as `LrTb`).
    The comment that called the second "a reading of the cell rather than a shrug" is retired:
    the value "shall be one of the following", and another is a value read past.
  - `model/box-colour-info-unusable` (§14.11.2.2): every case `guidelines::of` already said in
    `malformed` is now logged too.
  - `model/soft-mask-in-data-unusable` (§8.9.5.1): an `/SMaskInData` that is no integer, read as 0.
    An integer Table 87 does not list is **not** a repair: §11.6.4.3 makes the embedded mask govern
    where the entry "is present with a value other than 0", which is what the reader already did,
    and what the survey's Annex Q test counts.
  - `model/xfa-locale-name-absent` (§12.7.3, XFA 3.3 page 954, "This value must be supplied"): a
    `locale` with no `name` is not kept — before, it was kept under the empty name, which no picture
    clause asks for — and `LocaleSet::read` says so to the reader that holds the document (ADR
    2230's shape), which logs it at the packet's stream.
- **On existing classes**: a popup's `/Parent` that is no dictionary (read as parentless) and a
  `/C` number outside Table 166's 0.0 to 1.0 (read at the nearer end) are
  `model/annotation-entry-unusable`; a `/DA` whose `Tf` states no size (§12.7.4.3: "its two
  operands, font and size") is auto-sized as zero is, and logged as `model/field-entry-unusable`
  by `DefaultAppearance::size_read`, which all three layouts now read the size through.
- **A defect**: XFA 3.3 pages 947, 961 and 965 make `med` the default `name` of a `datePattern`,
  `numberPattern` and `timePattern`. An unnamed pattern was kept under the empty name and never
  found; it is now the medium one.
- **Marked**, each read against its source: the three catalog declines of `xfa.rs` and two of
  `destination.rs`; §8.4.1's clipping of a cap and a join code; §8.9.5.2's adjustment to the
  nearest allowed value; page 527's "the picture clause has no effect"; page 759's null string;
  page 153's `en_US` ambient locale; the banded JPEG decode's declines; the installed and
  stand-in faces of `pdf-font` (no document's program); the run table's own indices.
- **Stretch**: `annotation::page_annots` is public as `pdf_model::page_annots`, and the page
  `/Annots` readers of `pdf-archive` (survey, reach, examination) and `viewer-core` read through
  it, so the survey logs `model/page-annots-not-an-array` before the fence reads the log.

## Consequences

- `HELD` 176 → 122.
- Left: `pdf-transform`'s page `/Annots` readers, most beside a writer that must carry the entry
  as stated; a `/DA` `Tf` size that is negative is auto-sized without a class.
