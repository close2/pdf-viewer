# 1337 — A linearised file's first group is written whole

Session 1250. Status: accepted and **built**.
Context: `crates/pdf-syntax/src/linearize.rs` (`lay_out`, the module's section on object streams),
`crates/pdf-transform/tests/support/linearized.rs` (`faults`),
`crates/pdf-transform/tests/linearize.rs`
(`object_streams_in_a_linearised_file_meet_every_condition_f_3_1_states`), the F.3.1 ledger row.
Amends: ADR 1309's numbering of the first group, which packed parts 4 and 6 and numbered their
compressed objects before the hint stream. ADR 1309 is not edited.
Clauses: ISO 32000-2 §F.3.1, §F.3.4, §F.3.6, §7.5.8.

## 1. What the annex says about the order, sentence by sentence

- §F.3.1, the first group: "These objects shall be numbered sequentially, starting at the first
  object number after the last number of the second group. (The stream containing the hint tables,
  called a hint stream, may be numbered out of sequence; see F.3.6, "Hint streams (Parts 5 and
  10)"."
- §F.3.1, object streams: "Objects stored within object streams shall be given the highest range of
  object numbers within the main and first-page cross-reference sections."
- §F.3.4: "this crossreference table shall contain entries for the linearization parameter
  dictionary (at the beginning) and the primary hint stream (at the end)." And: "It shall consist of
  a single cross-reference subsection that has no free entries."
- §F.3.6: "The hint streams shall be assigned the last object numbers in the PDF file -that is,
  after the object number for the last object in the first page, including any objects stored
  within object streams. Their cross-reference table entries shall be at the end of the first-page
  cross-reference table. This object number assignment shall be independent of the physical
  locations of the hint streams in the PDF file."
- §7.5.8: Table 17's `/Index` "shall be sorted in ascending order by object number"; nothing in
  §7.5.8 orders a section's entries by type, so an uncompressed entry after compressed ones costs a
  §7.5.8 reader nothing.

## 2. The reading

F.3.6's "after" is about the **number**, not the position: its last sentence makes the number
"independent of the physical locations of the hint streams", and "including any objects stored
within object streams" is written for exactly this case. So the hint stream is numbered after the
first group's compressed objects, and qpdf's order (hint stream after part 4, before part 6;
`QPDFWriter.cc`, "Object number sequence") departs from F.3.6 and F.3.4 alike. F.3.1's "highest
range" holds beside it only through F.3.1's own parenthesis that the hint stream "may be numbered
out of sequence".

The annex therefore admits two files: a packed first group ending in the hint stream (ADR 1309),
and a first group with no compressed object. F.3.1 lets a linearised file hold object streams; it
does not require the first group to use them. **The second is chosen**, because under it all three
sentences hold read literally, none leaning on another's exemption, and it is the one every known
reader accepts. qpdf 12.4.1's `checkLinearization` warns "linearized file contains an uncompressed
object after a compressed one in a cross-reference stream" wherever a section has one, which is its
reading of F.3.1 without the parenthesis. That is evidence about the operative reading, as A131's
was, not the derivation.

## 3. The cost, measured

The first page's dictionaries stay uncompressed. On the ten corpus outputs ADR 1328 used, with the
default object streams: `freeculture` 2 315 089 to 2 314 917 bytes, `tracemonkey` 876 924 to
885 889, `TAMReview` 638 879 to 647 383, `alphatrans` 13 315 to 13 882, `issue1512r` 30 917 to
31 242, `sizes` 13 810 to 13 929, `annotation-highlight-without-appearance` 14 735 to 15 149, and
three unchanged. At most 4.3 per cent. `qpdf --check` exits 0 on six where it exited 3; the
warning is gone from all ten. `TAMReview` still warns about its source's unsorted name tree and
`issue15590` is still exit 2 on its source's `/Pages` naming a page.

## 4. What holds it

`faults` reads F.3.1's sentence over each whole section, the hint stream included, so a compressed
object anywhere in the first-page section is a fault; the F.3.1 test asserts the section holds none.
Calibrated by packing the first group again: three tests fail, one naming "in the first-page
section, object 38 is not compressed and follows compressed object 31".

## 5. Revisit

If an edition or a corrigendum says that F.3.1's range excludes the hint stream in so many words, or
a reader is found that refuses an uncompressed first group, the first group may be packed again.
