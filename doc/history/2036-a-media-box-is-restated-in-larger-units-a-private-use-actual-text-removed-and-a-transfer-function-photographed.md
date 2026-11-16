# 2036 — A media box is restated in larger units, a Private Use ActualText removed, and a transfer function photographed

Session 2036, an archive round in the audit worktree, on `doc/pdfa-blocking-issues.md` R03's finer remedies.
Rows 7.7.3.3, 10.5, 14.9.4 and 14.11.2.1 (notes); ADRs 2888 and 2889; no question.

**The slice.** The brief's three, in its order, each built whole: a MediaBox outside part 2's range, a Private Use
`ActualText` (part 4), a transfer function.

**The premise.** All three unbuilt and refused by name, effort the only obstacle. Q694 is the photograph's
widening (A694 item 1), not the `/UserUnit` remedy, and no later answer cites it. Item 1's premise held for one
document of two: `issue19517` (12 608 × 16 806) is past the range, `boundingBox_invalid`'s `[0 0 0 0]` is measured
by no unit and stays refused. Both `ActualText` documents state it on structure elements, not inline.

**Built.** `larger_units`: every box divided by the smallest power of two that fits, `/UserUnit` multiplied, the
content bracketed `q 1/k 0 0 1/k 0 0 cm … Q`; annotations, page patterns (§8.7.2), beads, viewports, placed
destinations and layout `BBox`es refused by name; mechanical. `actual_text`: the entry removed from each object,
loss `actual-text-private-use`, its value in the report; both shipped profiles `discard`. `fenced::transfer_pages`:
a page reaching a non-`Identity` `/TR` is photographed under `as-if-printed`, §10.5 having every processor apply it.

**Evidence.** veraPDF 1.30.3: `issue19517` compliant at 2b, 2u and 4, its 4 dpi render byte-identical to the
source's; `smaskdim` and `sakula` compliant at 4 under both profiles; `issue6931_reduced` at 2b and 4 under
`as-if-printed`, its logo red as the window draws it.

**Tests.** `tests/archive/finer_remedies.rs` (seven), `larger_units::tests` (one). **Plant**: `/UserUnit` not
multiplied, `transfer_pages` always `None`, the entry kept: four of the seven fail.

**Gates.** Over `pdf-transform`: `cargo fmt --check` exit 0; clippy `-D warnings --all-targets` exit 0; nextest
1205 of 1205 passed. `cd fuzz && cargo check --bins` exit 0. rustdoc `-D warnings --document-private-items` exit 0.
Conformance after `git add -N`: exit 0. Tier 2: `archive_corpus` exit 0 (8 of 8), `pdf-archive` corpus exit 0 (0
missed, 0 over); the census re-taken with the pack, exit 0, over pdf.js and corpora (1479 files). `corpus` and
`raster_golden` not run: no crate they build changed. `state.sh docs` exit 0.

**Left.** The four rows stay `implemented`. R03 lists 54 documents where it listed 57. Left of it here: a box
with a side of no length, the bracket's refusals (no census document reaches one), an inline Private Use
`ActualText` under `as-shown`, the transfer function under `as-shown`, and `/TR2` and a halftone's
`TransferFunction` (ADR 1964's owed). `state-every-loss` keeps `stop` for the new loss: its statement page has no
sentence for it.
