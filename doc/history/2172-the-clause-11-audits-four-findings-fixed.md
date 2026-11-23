# Round 2172 — audit 2171's four transparency findings fixed

A fix round in the `gwg` worktree, taking `doc/reviews/2171-specification-audit-11.md`'s four
groups. All five audit tests pass unchanged; the manifest's `failures` is empty again.

## What changed

- **B01** (ADR 3389): an absent or `null` `/SMask` is `SoftMaskEntry::Unsaid` and leaves the mask
  in force (§8.4.5: "parameter values established in previous invocations persist until explicitly
  overridden"); `/None` alone removes it. Every other `ExtGState` entry was checked for the same
  reading and none has it.
- **B02** (ADR 3390): a page group's `/K true` runs the page — content stream and annotations — as
  the elements of an isolated knockout group. The close of a form's group became `composite_group`,
  which the page now shares; a knockout page is not divided across threads.
- **B03** (ADR 3391): `soft_mask::table_142_faults` reports a `/BC` that is not the group space's
  `n` numbers and a `/Luminosity` group with no `/CS`, as departures of the mask's group; the
  drawing is unchanged.
- **B04** (ADR 3392): `Unsupported::UnknownBlendMode` for a single `/BM` name outside Tables 134
  and 135, in a graphics state and in an annotation's Table 166 entry; the array form stays silent.
  `viewer-core` and `tests/corpus.rs` name the new variant.

## Backends, golden and ledger

B01's and B02's fixtures, drawn by a probe test deleted afterwards: `render-cpu`, `render-raster`
(RX 580, RADV POLARIS10) and `render-gpu` agree at the centre within one level — (255, 127, 127)
for the mask kept, (127, 127, 255) for the knockout page, (127, 63, 191) for its control.
`raster_golden` (test profile): held 974, moved 0. Rows 8.4.5, 11.4.7, 11.6.3, 11.6.4.3, 11.6.5.1
and 11.6.6 stay `implemented`, each re-read with its audit test and a note sentence added.

**Gates.** Over `-p pdf-model -p render-cpu -p render-raster -p render-gpu -p viewer-qt -p
viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0;
nextest `--lib --bins --tests` (`xvfb-run -a`) 4137 passed, 0 failed, `audit-diff.py` "failed 0,
expected 0"; rustdoc `-D warnings --document-private-items -p pdf-model -p viewer-core` exit 0;
`fuzz` `cargo check --bins` exit 0; `cargo test -p conformance --no-fail-fast` exit 0 on the rerun
(exit 101 first: a blockquote attributed to §11.4.7 rather than §11.6.6, and this record's gates).

## Habit

A construction a form's group already has is reached by the page through the same method, not a
copy: extract the close, then call it from both places.
