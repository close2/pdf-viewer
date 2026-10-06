# 1586 — The crawl states every system in WKT, and no registry is carried

Status: accepted. Session 1375. Carries out the first obligation of the owner's answer
`doc/questions/A171` (the census over the crawl, the build's first commit) and records the fork it
decides; ADR 1587 is the build's accuracy budget. Amends nothing; ADR 1191's reading of §12.10 as
data stands, and this is the step it left.
Context: ISO 32000-2 §12.10.2–§12.10.4, Tables 269–271; ADRs 0405, 1191, 1574; trap 112.
Code: `crates/pdf-model/examples/geospatial_census.rs` (new), `crates/pdf-model/src/geospatial/`.

## 1. The instrument

The census reads every file of `corpus-cache` and the curated corpora — 90 763 — behind the
heavy-walk lock under `tools/bounded.sh --tree 12`, in 153 s at a peak of 8.8 GiB. A byte search
for `/GCS`, `/DCS`, `/WKT`, `/EPSG`, `/Measure`, `/GEO`, `/LGIDict` and `/ObjStm` comes first; any
hit sends the file to the parse, `/ObjStm` because no needle sees into a compressed object stream.
What is counted is a structure, not a name (ADR 0405): a dictionary whose `/GCS` is a dictionary,
which Table 269 requires of a geospatial measure. Its last table is this tree's own reader asked of
every `/GCS`, so the count of documents served is the build's and not an estimate.

## 2. What it found

| corpus | files | Table 269 measures | by `/WKT` | by `/EPSG` | `/EPSG` alone | projected `/GCS` | `/LGIDict` |
|---|---|---|---|---|---|---|---|
| SafeDocs crawl | 65 944 | 154 | 154 | 58 | 0 | 148 | 4 |
| Tika issue tracker | 23 075 | 14 | 14 | 1 | 0 | 8 | 7 |
| openpreserve | 267 | 1 | 1 | 0 | 0 | 1 | 0 |
| pdf.js test files | 974 | 1 | 1 | 0 | 0 | 0 | 0 |
| doc/corpora | 503 | 1 | 1 | 0 | 0 | 1 | 0 |

- **Every system that states `/EPSG` states `/WKT` in the same dictionary**; no document names a
  system by code alone. Seven state `/EPSG 0`.
- **Every WKT string is the older form** ISO 19162 Annex C documents: 158 documents `PROJCS`, 17
  `GEOGCS`, none ISO 19162's own `PROJCRS`; four carry ESRI's unknown-system GUID instead of WKT.
- **Methods, by document**: Transverse Mercator 85, Lambert Conformal Conic 61, Mercator 6, Albers
  4, `Double_Stereographic` 3, `Mercator_Auxiliary_Sphere` 3, Hotine oblique Mercator 4 (two
  variants), Krovak 1, vertical near-side perspective 1. **Codes**: 25832 (41 documents), 3857 and
  4326 (2 each), and 26912, 28350, 28992, 32631, 32724, 3878, 5650 once each.
- **Every one of the 158 projected `/GCS` states its `/GPTS` as degrees, and none states `/PCSM`**
  — the finding `doc/questions/Q271` asks about (section 4).

## 3. The fork

Table 270's `/WKT` carries the method and every parameter inline; `/EPSG` names them in a registry.
With no document stating a code alone, **the registry is not carried at all**: no table, no IOGP
dataset terms, no revision tail. `CoordinateSystem::reference_system` reads the WKT where both are
stated and refuses a code alone by its number. The older WKT form is read because it is what the
files carry, and ISO 19162's own form beside it because §12.10.3 names that standard; the methods
built are the census's — Transverse Mercator, both Lambert variants, Mercator A and B, the
Pseudo-Mercator (ESRI's auxiliary sphere type 0 among its spellings), Albers and the oblique
stereographic. Hotine, Krovak and the vertical perspective are refused by name.

## 4. What the census changed beyond the fork

The projection was asked for to turn a projected `/GPTS` into latitudes, and no file needs that:
every producer writes the latitudes already, against Table 269's sentence that a projected
system's array holds eastings and northings. Converting them as the table says would put each map
at its grid's origin. `Geospatial::registration_geographic` therefore refuses a projected set whose
every pair is shaped as degrees, by name, and `doc/questions/Q271` asks the owner whether to read
them as degrees as a departure. What the inverse serves today is `/PCSM`'s leg — no census file
states one — and every system a host meets in the file's own reading; what serves the 158 maps is
Q271's answer and the forward projection beside each inverse.

## 5. Re-running it

`find -L corpus-cache doc/corpora -name '*.pdf'` and the pdf.js directory into a list, then
`RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --tree 12 -- <binary> @list`.
The crawl is a 2021 public-web sample, so a zero there is strong evidence and not proof.
