# 2228 — An index held on every route, and the sections a reader balances read by the checker

Lane 3. ADRs 3613, 3614, 3615.

- **§8.6.6.3** (no erratum names it). Round 2224's "an `Indexed` sample past `hival` painted
  black" is not so: every route reaches `pdf_colour`'s `entry_of`, which rounds and clamps, and
  the unpacker's per-depth table holds an entry for every level, so its `BLACK` default is
  unreachable (now said under a `not a repair:` marker). Held instead (ADR 3613): one document
  per route in `tools/conformance/test-support/indexed_past_hival.rs` — images at 8, 4, 2 and 16
  bits, a `/Decode` below zero, a `/Matte`, an inline image, `scn`/`SCN`, an uncoloured pattern's
  base, a free-form mesh, an axial shading's function, a luminosity mask — drawn by `render-cpu`
  (`colour_suite`, a test per route), `render-raster` under its three coverages, and
  `render-gpu`. All agree with the clause; no pixel moves, so no golden page moved.
- **050734** was photographed: the fence named pages 19 and 21 and both were taken. Section 5.1
  then found `model/marked-content-unbalanced` at page 8, logged by the photograph step's own
  interpretation after the fence had read the log. The checker's grammar now reads Table 352's
  and §7.8.2's balance off the tokens (ADR 3614), so the fence names page 8 too; the document
  converts under `as-if-printed`, veraPDF passes it.
- **issue17679_2**: its form's `123 /oc1 BDC … EMC` sections were refused because ADR 3558's
  write took the `BDC` alone and `model/property-list-unresolved` was no class a write answers.
  A written-out `BDC` now takes its balancing `EMC`, and the property-list class is answered with
  it (ADR 3615); it converts under `as-shown`, veraPDF passes it. **issue6342** stays refused by
  name: `model/operands-left-over` (no write answers it) and section 6.2.2's runs like `c08.`.
- Left: `model/operands-left-over` and the other interpreter-only classes into the grammar where
  the tokens decide them; a table shorter than `m × (hival + 1)` reads missing entries as 0.

**Gates.** Over `-p pdf-colour -p pdf-model -p render-cpu -p render-raster -p render-gpu -p
pdf-transform -p viewer-qt -p pdf-archive`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` 0 (5504 of 5504; the manifest is empty),
rustdoc 0, fuzz 0; `cargo test -p conformance` 0 after the record's gates and the `unpack.rs`
ratchet (13 to 12). Census, with and without the pack (the same movement): `050734`
converts under `as-if-printed` at 2b and 4; under `as-shown` `issue17679_2` converts and
`issue15629` (a stream closing one `BDC` twice) is newly refused by its class's name; `050734`
and `504076` are now refused there under `model/marked-content-unbalanced`. veraPDF 1.30.3 over
5495 outputs: 40 not compliant and 12 unfinished, the same list as round 2224's.
