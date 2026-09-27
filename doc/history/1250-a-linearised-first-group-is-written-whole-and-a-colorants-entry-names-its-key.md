# Session 1250 — a linearised file's first group is written whole, and a `/Colorants` entry speaks for its key

Batch thirty-eight, parallel. Contract: two readings left by rounds 1245 and 1240.

**Part A (ADR 1337, amending ADR 1309's first-group numbering).** Read F.3.1, F.3.4, F.3.6 and §7.5.8
whole. F.3.6 numbers the hint stream last, "including any objects stored within object streams",
"independent of the physical locations", and F.3.4 puts its entry "at the end"; F.3.1's "highest
range" then holds only through its own parenthesis that the hint stream "may be numbered out of
sequence". §7.5.8 orders a section by number only. qpdf's writer numbers its hint stream before
part 6, against F.3.6 and F.3.4, and its checker reads F.3.1 without the parenthesis. The annex
admits a packed first group ending in the hint stream and a first group with no compressed object;
the second satisfies every sentence literally and every known reader, so `lay_out` packs only the
second group. `faults` reads F.3.1 over each whole section, hint stream included. Ten corpus
outputs: the qpdf warning is gone from all ten; six go from exit 3 to 0; cost at most 4.3 per cent
of bytes (`tracemonkey` +9 kB). Calibrated by packing the first group again: three tests fail.

**Part B (ADR 1338).** The brief's premise did not hold: "ColorantTable" in the tree is §10.8.3 step
a)'s, which is Table 402's `/DestOutputProfileRef` entry (a referenced profile's colourant names),
not Table 70's `/Colorants`, and it cannot be built from: it names colourants whose appearance lives
in a profile the renderer cannot fetch. `/Colorants` was already read by both readers. What was
missing was Table 70's "The key shall match the colourant name given in that colour space": any
`Separation` (spot curves) or any space at all (`nchannel_separations`) was taken. Now an entry
counts only as the `Separation` of its key; otherwise it is absent and each reader falls back as
for a missing entry. Fixtures: §8.6.6.5 EXAMPLE 2 with functions written out, EXAMPLE 5 (mixing
hints), and the mismatch in both readers; each key-rule test calibrated. Only under the simulation,
which is off by default, so no drawn page moves and `raster_golden` was not run.

**Rows.** F.3.1 stays `implemented`, note re-read and the qpdf sentence replaced. §8.6.6.5 stays
`implemented`, note gains the key rule, `colourants.rs` joins its code, four tests join its list.
`doc/todo/57`, `doc/todo/23`, `doc/state-of-play.md` say what is.

**Gates.** fmt exit 0; nextest (pdf-syntax, pdf-colour, pdf-transform, pdf-model) 2477/2477 exit
0; clippy clean on this round's files (neighbours' `viewer-core` too_many_lines at the time);
conformance: one failure, §10.7.4's `render-cpu` test name, a neighbour's. Behind the lock: on_disk,
transform gate, optimize_corpus (1918 linearised, 959 with object streams, 0 Annex F faults, 0 drew
differently): each exit 0.
