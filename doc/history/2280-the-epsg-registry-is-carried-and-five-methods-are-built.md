# 2280 — The EPSG registry is carried, and five projection methods are built

Lane 3, the owner's answer A1095 (option 1). ADRs 3821–3823; no question asked.

**A1095's `Owes:`, item by item.**
- *A separate, reproducibly generated data file from a pinned PROJ tag*: `tools/epsg-table.py` writes
  `crates/pdf-model/src/geospatial/epsg/dataset.rs` from PROJ 9.9.0's release archive (SHA-256
  `791a0610…e2bcf3`, EPSG v13.102): 1018 geographic, 5780 projected, 700 datums, 53 ellipsoids, 14
  meridians, 2550 conversions, 24 units, 47 method names, deprecated codes kept (ADR 3821). The ignored
  test regenerates and compares byte for byte; a planted byte failed `--check`.
- *Under IOGP's terms, with NOTICE and LICENSE-EPSG beside the Apache-2.0 licence*: `LICENSE-EPSG`
  (the terms as read on 2026-11-27), `NOTICE` section 9, `doc/third-party-data.md`.
- *Numeric EPSG lookup for §12.10.3*: `geospatial::epsg::system` builds the WKT path's types;
  `Refusal::EpsgNotCarried` and `Refusal::EpsgMethod` replace `EpsgWithoutWkt`. 4326, 27700 and 2278
  agree with their WKT to 1e-9 degree; every code of the table builds or is refused by its method.
- *The methods past the eight as their own rows*: built Cassini-Soldner, Lambert azimuthal equal
  area, polar stereographic A, B and C, each on its Guidance Note 7-2 worked example through the
  example's EPSG code (ADR 3822); the other 34 are `doc/todo/79`'s list, because the ledger has one row
  per numbered subclause (ADR 3823).

**Two Dataset readings**: four systems state a scale factor in metres, read as the ratio; unit 9108
is degrees. WGS 84's ensemble name joins `datum.rs`'s spellings.

**Ledger.** 12.10.3 `partial` → `implemented`; 12.10.4 stays `partial` (185 of 5780 systems, 34
methods); 12.10 aggregate `partial`.

**Launch and size.** `tools/state.sh --round 2280 launch`: 6 documents, 36 figures banded, 0
outside. Release `quorra` +968 335 bytes (`size`), 10 122 more load-time relocations, priced in ADR
3821; `quorra-confined` unchanged.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-host -p viewer-confined -p viewer-qt -p
conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib
--bins --tests` (`xvfb-run -a`) 4905 of 4910, the four clause 13 audit tests round 2279 owns
(`expected-failures-current.json`'s four, unchanged) and this record before its gates were written;
the ignored regeneration test 0; rustdoc of `pdf-model` 0; fuzz `cargo check --bins` 0;
`cargo test -p conformance` 0.
