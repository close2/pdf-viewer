# 3798 — The EPSG registry is priced, and its licence is the owner's question

Session 2274, review 2271's D01. Status: **accepted**. Context: ADR 1586, `doc/questions/A865`,
`doc/questions/Q1095`; ISO 32000-2 §12.10.3 with Errata Collection 3 Issue #321, Table 270.

## Context

§12.10.3 as corrected: the system "shall be described in any of two well-established standards: as a
numeric EPSG reference code, or as a Well Known Text (WKT) string". A dictionary stating `/EPSG` alone
is a conforming file, and this program refuses it by its number because no registry is carried. ADR
1586 rested that on a corpus census in which no document states a code alone; A865 says the corpus
is not what counts. The 12.10.3 row said `implemented` while refusing.

## The price

- **Which part**: geographic 2D and 3D CRSs, the projected CRSs over them, their datums, ellipsoids,
  prime meridians, units and conversions. PROJ 9.9.0's `data/sql` carries EPSG v13.102 at a pinnable
  tag: 902 geographic, 5417 projected, 624 datums, 50 ellipsoids, 15 meridians (non-deprecated).
- **Size**: about 360 KB of compiled-in `static` data with no names (CLAUDE.md's "no parsed data at
  startup" holds: a table lookup by code); the WKT strings instead would be about 4.6 MB.
- **Methods**: the eight built cover 95.2 % of the projected systems; about 35 more cover the rest.
  1007 projected systems count in a unit other than the metre, 816 of them the U.S. survey foot.
- **Reproducibility**: a generator reading the pinned PROJ files, its output checked in and a test
  that regenerates and compares, as the Arlington tables are.
- **Licence**: IOGP's terms (revised 8 April 2016) permit subsets, with ownership acknowledged and
  values unchanged, but forbid "[d]istribution for profit" and oblige a distributor to pass the terms
  on. That is a second licence over part of an Apache-2.0 program, binding whoever redistributes it.

## Decision

Not built in this round: the size is sane and the generator is ordinary, but whether the program may
carry data under a non-commercial clause is the owner's to decide, and `Q1095` asks it with the three
options. Until it is answered the refusal stands, by its number, and the 12.10.3 row (and the 12.10
head above it) is `partial`, naming the code-alone form as the requirement owed and A1095 as what
moves it.

## Consequences

- If the owner takes option 1 or 2, a round builds the table and the first methods past the eight,
  each a row of its own; if option 3, the row becomes `departed` on A1095 with this price.
