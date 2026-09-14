# 1040 — A `shall` behind a silence, drawn; and a window a watermark never had

Ledger slot of batch 1038–1043, five siblings in one worktree. Contract: §12.5's thirteen `partial`
rows after 1034. **Two closed, three narrowed, one new module, one census, thirteen calibrated tests,
one ADR. No corpus pixel moves.**

| row | was | now | what moved it |
|---|---|---|---|
| §12.5.6.8 Square and circle | `partial` | **`implemented`** | Table 169's cloudy `/BE` drawn, `pdf_model::cloud` |
| §12.5.6.9 Polygon and polyline | `partial` | **`implemented`** | the same, on `/Vertices` and on `/Path`; a polyline's `/BE` not read |
| §12.5.6.22 Watermark | `partial` | `partial`, one debt | "shall have no popup window nor other interactive elements" read; the printing half remains |
| §12.5.4 Border styles | `partial` | `partial`, one debt | the cloud sentence retired; Table 168's B and I remain |
| §12.5.2 Annotation dictionaries | `partial` | `partial`, two entries | `/AF` and `/Lang` for an annotation; `/M` was on the list six hundred sessions after it was read |

**1. The two rows 1034 left were on the wrong side of `appearance.rs`'s own line.** The module
refuses an appearance whose clause does not say what it looks like, and the cloud sat there beside
§12.5.6.12's stamp legends. Read whole, §12.5.4 introduces `/BE` with a `shall` — "an effect that
shall be applied to the border" — and Table 181 says it again; Table 169's `should`s only describe
the look. That is ADR 0192's construction, and ADR 1057 chooses the artwork once: semicircles on
chords of the line's own path, cusps a radius inside the inscribed shape, edges divided evenly so
corners are cusps, two points plus two per unit of `/I` and never under the line's width, round
joins — the last two from looking (trap 1): the first render showed a miter spike at every cusp
and scallops thinner than a six-point stroke, and no test had seen either.

**2. The corpus cannot rank it, counted rather than assumed.** `examples/cloudy_border_census`: two
annotations under `doc/` state `/BE /S /C`, both squares in `AnnotationTypes.pdf`, both with an `/AP`,
so the fixtures are hand-built (trap 8); the two are evidence of scale only (ADR 1057 §2).

**3. §12.5.6.22's sentence bound nothing**: a `/Popup` on a watermark opened a window and
`annotation_at` found its rectangle. `annotation::interacts` and `popup` now read it. §12.5's own
note still said §12.5.6.3's state was read by nothing, six rounds after 1034 read it; retired.

**Calibration (trap 13).** Each of the seven behaviour tests was run with its rule removed — `cloudy()`
returning `None`, `is_watermark` short-circuited in three places — and all seven failed.

**Gates.** Tier 1 whole and `raster_golden`, exit codes in the round's report. `raster_golden`'s one
mover, `bug1721218_reduced.pdf`, is "list only" and the file has no `/Annots` at all, so nothing in
this round's diff reaches it: a sibling's interpreter change, named rather than explained (trap 37).
Five siblings edited `pdf-model` throughout, so the workspace lines measure their trees as well as
this one, and a sibling's rustfmt moved this round's anchors twice.
