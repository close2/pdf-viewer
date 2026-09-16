# 1124 — The reachable knockout member is drawn; the gap is the one alpha the clause forbids

2026-09-16. Files: `crates/render-cpu/tests/group_constructions.rs` (one calibration test),
`doc/conformance/ledger.toml` (§11.4.3, §11.4.4, §11.4.6 readings + one test path), this record. No
backend `src` edit owed; no ADR. Every other path in `git status` (`image.rs`, `ccitt_bound.rs`,
`page_labels.rs`, `optimize.rs`, the 1125–1129 records) is a sibling's, on disjoint rows and files.

## The question: is the knockout-member residue a backend gap? No — it is already drawn.

The contract's residue — a knockout group whose member is non-opaque or a nested group, "reported
rather than composited" — is, measured, not reported: it is drawn wherever the member's shape can
be *stated*. A non-opaque solid draws as bare coverage; a non-opaque image or shading states its
shape from `SampleAlpha`/`Shading::opaque` (ADR 1017); an isolated nested group states its shape as
itself (ADR 0234, ADR 0554); a blending member takes the own-backdrop form (ADR 0327, ADR 1009). It
is 1089's pairing and 1093's backdrop removal *reached*, not a gap. **Census** (`group_shape_census`,
963 opened first pages of `doc/pdf.js` + 2 `corpora-own`, under the lock): 33 knockout groups on 18
pages; 5 hold a stated shape on 5 pages; 0 on the group's own backdrop; **0 refusals**. The gap has
no first-page witness. What stays `partial` is the case §11.4.6 itself says one alpha cannot carry —
a stencil under its own `/SMask` (`SampleAlpha::Both`), a non-isolated group as an element, `/AIS`
both ways — priced at a second raster per command in ADR 1022 §5.

## Calibrated (trap 13), because a corpus clean of the gap is a sentence about the corpus

`a_translucent_knockout_element_shows_the_initial_backdrop_not_the_element_below`: an opaque red
lower and a `ca ½` blue upper, all Normal, in an isolated `/K` group over a green page. §11.4.6
composites the upper with the group's initial backdrop, so it knocks the lower out: the overlap
draws `½ blue + ½ green = (0, 127, 127)`, the page through the upper. With `knockout` cleared it
draws `(128, 0, 128)` = `½ blue + ½ red`, the lower read through. The number discriminates knockout
from the accumulated backdrop.

## Ledger and gates

§11.4.6 gains the fixture and the reading; §11.4.4 records the residue is a genuine model gap, not a
buildable combination; §11.4.3 records the collapse `initial_backdrop` feeds draws it — all stay
`partial`, nothing built. `render-cpu` clippy `-D warnings` and fmt clean, 148 tests pass.
`raster_golden`: exit 0, held 974, moved **0** — output byte-identical, so the oracle cannot move.
`render-raster --test corpus`: exit 0, 958 compared, 943 agree, 8 differ, 7 refused, 16 not
comparable. `pdf-transform --test gate`: exit 0, 82.9 pages/s (floor 40). `oracle`: exit 0, 1962
pages in 53.8 s, 46 not comparable, head unchanged. `conformance`: ledger agrees. `batch.sh check`:
clean. Any batch red is a sibling's.
