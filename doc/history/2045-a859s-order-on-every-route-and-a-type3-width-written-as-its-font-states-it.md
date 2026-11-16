# 2045 — A859's order on every route, and a Type 3 width written as its font states it

Session 2045, a single ledger round in the audit worktree. Rows 9.6.2.1, 9.8.1 and 9.6.4; owner answers A868 and
A865; ADRs 2906 and 2907.

**The slice.** The brief's two items in order: A868 (A859's order on the embedded-program and metrics-only routes,
three audit tests sanctioned), then A865 (a Type 3 `d0`/`d1` `wx` restated from `/Widths` under a named loss).

**The premise.** Both unbuilt and effort only once the owner answered: ADR 2898 held the two routes for Q868, and
ADR 2847 photographed the Type 3 row behind ADR 0816's fence for Q865.

**Built.** `simple_widths` takes, outside the fourteen, an embedded program's own advance (a Type 1 `hsbw` too),
then the resembling metrics, then `/MissingWidth`; `published_metrics` derives the request for every such font;
`StatedMetrics::read` gives a metrics-only font the metrics; `SuppliedWidths::from_program` counts the first step
and the metrics-only route reports it. The three audit tests now assert 0.5 em, 667 and the report. The archive
followed by construction (an embedded program now takes `layout-from-this-viewer`). `type3_widths.rs` writes a
procedure's first operand as the `/Widths` entry, copies a procedure reached at a second width, gives a font a
`/CharProcs` naming the copy and, where one font's codes split, a suffixed name in its `/Differences`; the loss
`type3-width-as-drawn`, history, report; `as-shown` and `as-if-printed` grant it, `state-every-loss` keeps the
photograph.

**Tests.** pdf-font: the audit test, a Type 1 `hsbw` test; pdf-model: the two audit tests; pdf-transform: one
window-and-archive test for an embedded program, five in `type3_width_as_drawn.rs`, the photograph test moved to
`state-every-loss`. veraPDF 1.30.3 passes the four fixture outputs and `issue19634` and `issue19971` under both
profiles (486 and 845 widths written, none photographed). **Plant**: the program step dropped fails both pdf-font
tests and the archive test; the in-place write dropped fails `a_procedure_every_font_needs_one_width_for_is_written_in_place`.

**Gates.** Over `pdf-font`, `pdf-model`, `pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
nextest 4274 of 4274 passed; rustdoc `-D warnings --document-private-items` exit 0; `cd fuzz && cargo check --bins`
exit 0. Conformance after `git add -N`: exit 101 on this record's gates paragraph, then exit 0. Tier 2: `corpus`
101 as HEAD's (the same pair joined; no page moved); `raster_golden` 101 as HEAD's 23 first pages once six reports
(`issue5470`, `issue6069`, `issue6106`, `issue6108`, `issue2391-2`, `issue18099_reduced`, §9.6.2.1's sentence) were
re-held; `pdf-archive` `corpus` 0 (rerun after a thin-LTO failure); `archive_corpus` 0; census 0, `rank.py`'s table
re-taken (R03 52 / 51 under `as-shown`); `state.sh docs` exit 0.

**Left of the rows.** 9.6.2.1 and 9.8.1 stay `implemented`; 9.6.4 stays `implemented`. `state-every-loss` states no
line for the new loss yet. Our validator missed a Type 3 code with no glyph name that veraPDF failed (6.2.10.4.1).
