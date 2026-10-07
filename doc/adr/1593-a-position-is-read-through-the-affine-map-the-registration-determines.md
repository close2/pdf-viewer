# 1593 — A position is read through the affine map the registration determines, and written in decimal degrees

Session 1378. Status: **accepted** and **built**. Builds on ADRs 1191 (the measuring query), 1586
and 1587 (`pdf_model::geospatial`); decides nothing `doc/questions/Q271` asks, whose refusal it shows.
Context: ISO 32000-2 §12.10.2 (Table 269), §12.10.4, §8.3; IOGP Guidance Note 7-2 for the degrees
`pdf_model::geospatial` returns.
Code: `crates/viewer-core/src/located.rs` (`Located`, `locate`, `fitted`), `crates/viewer-core/src/query.rs`
(`Answer::Measured`), `crates/viewer-host/src/measuring.rs` (`degrees`, `said`),
`crates/viewer-confined/src/protocol.rs` (`encode_located`), `crates/viewer-ui/src/bin/quorra-confined/typing.rs`
(`measure_at`). Tests: `crates/viewer-core/src/located.rs`, `crates/viewer-core/tests/measuring.rs`,
`crates/viewer-host/src/measuring.rs`; `tools/drive-windows.sh` step `38-located`.

## 1. Which point, and which route

The position is the measured path's **last** point, the one a person just put down. Outside Table
269's `/Bounds` — "the bounds of an area for which geospatial transformations are valid" — no
position is given and the sentence says so. Inside it: `/PCSM` where Table 269 gives it priority
(a projected `/GCS` beside a matrix), handed the point in default user space, because "the
transformation from XObject position coordinates" is applied to a page's viewport positioned there;
otherwise `Geospatial::registration_geographic`, which returns a geographic map's `/GPTS` as stated
and refuses a projected one by name — the census's every projected map, whose `/GPTS` are degrees
(Q271), or one whose method the tree does not evaluate. A refusal's own sentence is what the window
shows.

## 2. Between registration points: affine, fitted, and its departure said

§12.10 states the registration points and no function between them. §12.10.4 permits "direct mapping
between geographic coordinates and PDF object coordinates" for small areas without saying what form
it has; the one function Table 269 does state is `/PCSM`, a matrix, and every mapping between
coordinate spaces elsewhere in the standard is §8.3's affine one. So latitude and longitude are each
read as an affine function of the unit square, fitted by least squares to every pair the file states.
Four corners of a north-up rectangle of degrees — the one geographic map among the curated documents,
`bug1146106.pdf` — lie on such a map exactly. Where the file's points lie on none, the largest
distance between a stated point and the fit, in degrees, is said beside the position; no threshold
turns it into a refusal, because a threshold would be a number of this program's choosing and the
person is better served by the number itself. Fewer than three pairs, or pairs on one line, determine
no map and are refused. Not rejected: a bilinear or projective map through four corners (exact for
any quadrilateral, but a form no clause names and undefined for three or five points), and
interpolation in the projected space (Q271's question for the maps that need it).

## 3. How it is written

Table 269's `/DCS` row: "[f]ormatting the displayed representation of these values is controlled by
the interactive PDF processor". This program writes **decimal degrees to six places with the
hemisphere's letter** — `13.574120° S, 171.193325° E`. Decimal, because the clause's own example of a
display system is one "corresponding to values reported by a GPS device", and those report decimal
degrees, as the WKT's `UNIT["Degree"]` states them; six places, because a millionth of a degree is a
tenth of a metre, finer than any map's registration, so the last place is never the reading's error;
letters rather than signs, because a minus is the character of a coordinate a person misreads.
Degrees, minutes and seconds were the alternative: as legible, but two decimal seconds to reach the
same precision and a form no GPS readout or WKT unit shares. `/DCS`'s position is written beside the
file's own where the file names one, or its refusal (another datum, a projected display system).
`/PDU`'s angular unit names how *angles* are displayed and is not applied to positions.
