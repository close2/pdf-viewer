# 1375 — The crawl states every system in WKT, and a projected point comes back

Ledger slot of batch fifty-eight, on the owner's answer A171. ADRs 1586 and 1587; question Q271.

**The census** (`crates/pdf-model/examples/geospatial_census.rs`, the build's first piece): all
90 763 files of `corpus-cache` and the curated corpora, byte needles then a parse, behind the lock
under `bounded.sh --tree 12`, 122 s at a peak of 8.4 GiB.

| corpus | files | Table 269 | `/WKT` | `/EPSG` | `/EPSG` alone | projected |
|---|---|---|---|---|---|---|
| SafeDocs crawl | 65 944 | 154 | 154 | 58 | 0 | 148 |
| Tika issue tracker | 23 075 | 14 | 14 | 1 | 0 | 8 |
| openpreserve, pdf.js, doc/corpora | 267, 974, 503 | 1, 1, 1 | 1, 1, 1 | 0 | 0 | 1, 0, 1 |

Every WKT is the older form (158 `PROJCS`, 17 `GEOGCS`, 4 ESRI GUIDs, no `PROJCRS`). Methods by
document: Transverse Mercator 85, Lambert 61, Mercator 6, Albers 4, double stereographic 3, ESRI
auxiliary sphere 3, Hotine 4, Krovak 1, vertical perspective 1. Codes: 25832 ×41, then ten others.

**The fork.** No document names a system by code alone, so no EPSG registry is carried: a code
alone is refused by its number, WKT is read where both are stated (ADR 1586).

**The build** (`crates/pdf-model/src/geospatial/`): ISO 19162 section 6's string form; the older
`PROJCS` form and ISO 19162's `PROJCRS`; eight inverse methods by Guidance Note 7-2, each held to
its worked example within 0.0005″ (ADR 1587); `/PCSM`'s leg to degrees; `/DCS` on the same datum.

**What the census changed.** All 158 projected maps write `/GPTS` as degrees, none states `/PCSM`;
Table 269 says eastings and northings. Such a set is refused by name and Q271 asks whether to read
it as degrees (recommended: a departure). Final reading by the tree itself: 151 refused as degree-shaped, 16 geographic read, 9 refused by method or malformed WKT, 4 GUIDs.

**Rows.** §12.10 and §12.10.2 stay `partial`, each reason narrowed to Q271 and a `/DCS` on another
datum; §12.10.3's and §12.10.4's notes say the WKT is now read. `doc/todo/65`'s bullet rewritten.
Texts: IOGP 373-7-2 and OGC 18-010r11 at `/home/AI/specs/`, Markdown in `doc/md/`, cited never quoted.

**For the next UI round.** A measure tool or hover can show `Geospatial::geographic_position` and
`display_position`; the forward projection beside each inverse is what serves the 158 maps.

**Gates.** `rustfmt --check` on my six files exit 0; `cargo clippy -p pdf-model --all-targets -D warnings` exit 0 in a HEAD export with my files, exit 101 in the worktree only on siblings' `tests/aform.rs` and `tests/script_corpus.rs`; `cargo test -p pdf-model` exit 0, 1862 passed; `cargo test -p conformance` 382 passed, 1 failed (`records`, on 1376's unfinished Gates line); `the_frontier_map` passed.
