# 1244 — The four documents read whole against the tree, and raster's last `§` literals

Instruments slot, batch thirty-seven. No ledger status moved; no ADR. Span `4a97ea7a..HEAD`.

- **`doc/PLAN.md`, 6.** Section 1 said no window opens a file chooser (Ctrl + O is in all three,
  ADR 1275); sections 1 and 6 missed `ring` under `viewer_host::submit` (ADR 1291, Q130). Section
  5a gains the frontier map and the crate's other gates (ADRs 1273, 1274, 1313).
- **`doc/crate-map.md`, 13 in 11 rows.** `linearize.rs`, `colourants.rs`, `xfdf/annotations.rs`,
  `viewer_host::{documents, restriction, clock, submit}`, `quorra/arrivals.rs`, `encode/overprint.rs`
  and three `archive/` modules are named. It said `<annots>` was counted, not read (ADR 1297);
  that both other backends refuse a non-isolated group under a mode (`render-raster` draws it,
  ADR 1307); and that only tests reach `Plan::Archive` (`quorra-transform archive` does).
- **`doc/state-of-play.md`, 7.** "No window draws the path it is measuring" contradicted ADR 1264's
  paragraph. Knockout groups (ADRs 1301, 1305, 1306), the front-document rule (ADR 1303) and the
  staged blending conversion and matte (ADRs 1267, 1268) were missing; three sentences of history.
- **`doc/HANDOVER.md`, 4.** `batch.sh commit`; a row for a round writing a whole file (todo/57,
  64, RFC 0002); the XFDF text held beside ISO 19444-1.

## Sweep counts (before → after)

`pointers`: 310 → 314 absent, the four new ones siblings' `A72` paths; hits in the four documents
2 → 2, both false (an uncommitted `A100`, the `scratchpad/r` template). `overtaken` 69 and `unread`
153 over 68 rows unchanged; neither reads the four. `retired` over the five retired nouns finds them
only in ADRs. The history regex 3 → 0. `--bin cited`: `65 with no row`, `11 … under raster/` →
`54` and `0`.

**Raster literals.** Thirty-three string-literal lines in ten files read "brief section N" or
"`<document>` section N"; `command.rs`'s `§11` is `§11.3`. `every_case_cites_a_source` took a `§` as a source and now also
takes "`.md` section". The listing hid four more (one line per clause-file pair).

**Frontier map.** `doc/todo/65`: the §11.4.7/§11.5.3 bullet led with `MAX_PRESSES`, which both
notes say keeps neither open; the questions paragraph says what 2026-09-22's answers built, Q130 and Q131 open;
bucket 4 has two rows, not four.

**`tools/state.sh navigation`** (in `quick`) runs the three counting sweeps and the history regex.

**Left**: §11.4.3's note says §11.4.4 is `partial` (it is `implemented`); §8.9.6.2's bullet asks
for the premultiplication ADR 1287 built (round 1243's row).
