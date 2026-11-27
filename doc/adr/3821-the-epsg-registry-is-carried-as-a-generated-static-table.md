# 3821 — The EPSG registry is carried as a generated `static` table

Session 2280, the owner's answer `doc/questions/A1095` (option 1). Status: **accepted**. Context: ADRs
1586, 3798; ISO 32000-2 §12.10.3 with Errata Collection 3 Issue #321, Tables 270 and 271.

## Decision

- **Source, pinned.** PROJ 9.9.0's release archive, `https://download.osgeo.org/proj/proj-9.9.0.tar.gz`,
  SHA-256 `791a0610547eeabb17006cfd49cdbd2034f3240f47ed5e88a1031811f4e2bcf3` (its published MD5
  `50ed234d73d68f7bf4378e9a020a40a7` agrees), whose `data/sql/metadata.sql` states EPSG v13.102 of
  2026-08-27. The release archive rather than GitHub's tag archive, whose bytes GitHub does not promise.
  The generator keeps it in a cache directory on the scratch disk, outside the tree, and it is never
  committed.
- **Generator.** `tools/epsg-table.py`: `--fetch` downloads and checks the digest and unpacks only
  `data/sql/*.sql`; the default run writes `crates/pdf-model/src/geospatial/epsg/dataset.rs`; `--check`
  writes nothing and fails on a difference. It refuses a source whose metadata names another EPSG
  version. It never runs at build time or launch. The ignored test
  `the_table_is_what_the_generator_writes_from_the_pinned_release` runs `--fetch` then `--check`; a
  planted byte made `--check` fail and regeneration restored it.
- **Subset.** Every EPSG-authority geographic 2D and 3D system (1018), every projected system over one
  (5780), and their closure: 700 datums and ensembles, 53 ellipsoids, 14 prime meridians, 2550
  conversions with every parameter, 24 units, 47 method names. Geocentric, vertical, compound and
  engineering systems are out: §12.10.3's `/GCS` is geographic or projected, and such a code is refused
  by its number.
- **Deprecated codes are kept** (about 7 % of the rows): a code names the system it named when a file
  was written, and a file written before a deprecation is no less conforming for it.
- **Names are kept for systems and datums**, not for ellipsoids and meridians: a system's name is what
  a host shows, and a datum's is how `datum.rs` tells two datums apart (WGS 84 by name, ADR 1958; the
  Dataset's `World Geodetic System 1984 ensemble` is added to its spellings). ADR 3798's 360 KB had no
  names.
- **Values unchanged** (IOGP's terms): each number is the SQL's text, a whole number given `.0`. A
  parameter in sexagesimal DMS (unit 9110, 3963 of the parameters) is converted where it is read.
- **Form.** Tuple-struct `static` slices sorted by code, searched by binary search: nothing is built
  until a code is looked up. One record per line, `#[rustfmt::skip]` on the module so a regeneration's
  diff is a line per changed record. The file is about 1.0 MB of source.
- **A datum transformation is not part of a code.** A system read from a code states none
  (`datum_shift: None`), as a WKT string without `TOWGS84` states none; the Dataset's many
  transformations per datum are a choice the code does not make.
- **Licence.** `LICENSE-EPSG` reproduces IOGP's terms (revised 8 April 2016) as read on 2026-11-27,
  words unchanged and Table 1's layout restated; `NOTICE` section 9 acknowledges IOGP's ownership and
  says the terms bind whoever passes the program on; `doc/third-party-data.md` records the source.

## What it costs, measured

The release `quorra` grows by 968 335 bytes of `size`'s total (text +662 607, data +303 280, bss
+2 448; the file by 967 136), against ADR 3798's 360 KB without names. The difference is the names
(about 200 KB) and the layout: a tuple struct of a `&str` and three `u32`s is 40 bytes, a parameter
24 with its padding, and every name and parameter slice is a pointer, which in a position-independent
binary is a load-time relocation — 10 122 more `R_X86_64_RELATIVE` entries (30 808 to 40 930), applied
by the loader before `main` over about 300 KB of `.data.rel.ro`. That is the one part of the table
not deferred to a lookup. `tools/state.sh --round 2280 launch` measured 6 documents, 36 figures
banded, 0 outside. If a band ever moves on it, indices into one string table and one parameter array
in place of pointers remove the relocations and about half the bytes; it is not done on a cost no
gate sees. `quorra-confined` is unchanged: the confined renderer does not reach §12.10.

## Two readings the Dataset forced

- **A scale factor stated in metres.** Four Transverse Mercator systems (conversions 17725 and 19880,
  the Fiji grids among them) give the scale factor at the natural origin unit 9001. A ratio in a unit
  whose factor is one is read as the ratio its number states, a choice written in `scale_value`; the
  alternative refused four valid systems for a record's inconsistency.
- **Degree minute second hemisphere** (unit 9108, a few deprecated geographic systems) has no factor:
  it is a way of writing degrees, so its factor is the degree's.

## Consequences

§12.10.3 is `implemented`: every geographic code of the table resolves to the types a WKT string
builds, and 4326, 27700 and 2278 are tested against their WKT to a nanodegree. A projected code over a
method not built is refused by its number and the method's name (`Refusal::EpsgMethod`), which is
§12.10.4's debt (ADR 3823). `Refusal::EpsgWithoutWkt` is gone; `EpsgNotCarried` and `EpsgMethod`
replace it.
