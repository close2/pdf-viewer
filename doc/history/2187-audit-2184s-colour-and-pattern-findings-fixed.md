# Round 2187 — audit 2184's colour and pattern findings fixed

A fix round in the `gwg` worktree, taking audit 2184's five groups. All seven audit tests pass, one
with an exact fixture (below); the manifest's `failures` is empty and 2184's six findings `fixed`.

## What changed

- **B04** (ADR 3450): a `Pattern` space read where it carries no colour is refused and said, never
  drawn black — `shading::kind_of` refuses a shading's `/Pattern` space (Table 77),
  `parse_indexed` a `Pattern` base (§8.6.6.3), and `scn` given numbers and no name in a `Pattern`
  space keeps the colour in force under the new `Unsupported::PatternOperands` (§8.6.8).
- **B02** (ADR 3452): the `/N` space of a profile without `/Alternate` is an `IccAlternate` too.
- **B03** (ADR 3449, answering ADR 1241 section 5): `cmyk_tints` and `reverts_to_cmyk` gain an
  `Indexed` arm over the entry `entry_of` selects. The audit test compared byte `E6` with `0.9`,
  one level apart in red; its fixture now states `CC` and `0.8`, which are the same number.
- **B01, D01**: `/BG`, `/BG2`, `/UCR`, `/UCR2` behind `!self.uncoloured`, the comment restated.
  **D02** (ADR 3451): a zero or absent `/XStep` or `/YStep` refuses the pattern by name under
  `Unsupported::Shading`; the `/BBox` fallback and `lattice::cell_extent` are gone.

## Backends, golden and ledger

B03's and B04's fixtures by a probe deleted afterwards: `render-cpu` and `render-raster` (RX 580)
agree exactly — (7, 18, 35) for both overprint fixtures, white for the three B04 refusals.
`render-gpu` agrees on B04 within one level and refuses B03's `DeviceCMYK` page group, as it
does every four-component blending space (§11.4.7). `raster_golden`: held 973, moved 1 —
`operator_list_cycle.pdf` p1, "list only … and the reports": its pattern states no step (D02),
no pixel moved; updated through the walk. Rows 8.7.4.3, 8.6.6.3, 8.6.8, 8.6.5.5, 8.6.7 and
8.7.3.1 stay `implemented`, each re-read and given its sentence.

**Gates.** Over `-p pdf-colour -p pdf-model -p render-cpu -p render-raster -p render-gpu -p
viewer-qt -p viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox
--bins` exit 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 4378 passed, 0 failed,
`audit-diff.py` "failed 0, expected 0"; rustdoc `-D warnings --document-private-items` of
`pdf-colour`, `pdf-model`, `viewer-core` exit 0; `fuzz` `cargo check --bins` exit 0;
`cargo test -p conformance` exit 0.

## Habit

A fixture that compares a table byte with a decimal compares two numbers: pick a byte whose
fraction is the decimal (`CC` is 0.8, `E6` is not 0.9) before reading a one-level miss as a defect.
