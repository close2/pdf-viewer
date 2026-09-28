# 1378 — `CONTRADICTED_DEVICE_CMYK_CONVERSION` re-read against the tree after ADRs 1153 and 1253

Status: accepted. Session 1270.
Context: `crates/pdf-model/tests/oracle.rs` (`CONTRADICTED_DEVICE_CMYK_CONVERSION`),
`crates/pdf-colour/src/colour.rs` (`CMYK_CORNERS`).
Clauses: ISO 32000-2 §10.4.2.1, §10.4.2.5, §10.3.2.

## What was asked, and what the group actually argues

The brief put the group's argument as §10.4.2.5's conversion against the references'. It is not:
no renderer on these pages uses §10.4.2.5 (on `transparent.pdf` it would draw the bottle black), and
§10.4.2.1 ranks that formula as the less-capable processor's "crude approximations" below §10.3.
All five renderers are on §10.3's route; the group is two *source assumptions* §10.3.2's NOTE
licenses — ADR 0009's process-ink corners (ours, and `poppler` to a level) against a SWOP/CGATS
press (`mupdf`, `ghostscript`, `hayro`). The note now quotes §10.4.2.1 and says so.

## Is the argument still each page's reason

Yes, page by page, on this run's panels. ADR 1153's shipped profile is the archival converter's
output intent and changes no rendering; ADR 1253's black point compensation acts on the ICC route,
which a document naming no press never reaches; the §10.8.3 separation work did not move
`function_based_shading_cmyk.pdf` page 1's `/Separation` square. The oracle's figures for the five:
2.70/10.68/17.25%/.9959, 5.15/19.47/29.19%/.9956, 7.30/18.04/37.25%/.9942, 0.64/3.01/3.34%/.9956,
0.31/6.70/1.29%/.9998 — four identical to ADR 0510's, `transparent.pdf`'s moved at the silhouette
only. The samples reproduce exactly: `postscript_type4_many_outputs.pdf` (100, 25) ours
(127, 214, 247), `poppler` (128, 214, 247), `mupdf` (109, 207, 246), `ghostscript` (108, 207, 246),
`hayro` (109, 206, 246); the bottle ours and `poppler` (28, 32, 40), `mupdf` (25, 34, 45),
`ghostscript` (25, 35, 46), `hayro` (26, 33, 45). No page leaves the group.

Corrected in the note: the "three files", "four pages" and "states no destination" sentences, two
history parentheticals, and `transparent.pdf`'s gate figures and their decomposition, each from this
run. The 802→800 figure and ADR 0510's ablated column are left named for their runs (trap 55).
