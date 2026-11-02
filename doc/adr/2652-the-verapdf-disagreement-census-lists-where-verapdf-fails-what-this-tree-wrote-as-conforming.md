# 2652 — The veraPDF disagreement census lists where veraPDF fails what this tree wrote as conforming

Session 1909, slot 9 of batch 131, a "clause" round (reading).
Status: implemented in `tools/verapdf-census.py`, `crates/pdf-transform/tests/archive_corpus.rs`
(`QUORRA_CENSUS_OUTPUTS`, `Kept`) and `tools/state.sh` (`section_archive`). No fix: every row below is a
finding for a later slot. ADR 2653 is unused.
Context: ADRs 2557, 2582, 2608, 2619 and 2632, each a checker gap found by putting one output through
veraPDF; ADR 0947 (an output is written only when it conforms on the second verdict); principle 5.

## 1. The instrument

The census of real producers' files (`the_shipped_profiles_list_what_real_producers_files_refuse_on`)
keeps every output it writes when `QUORRA_CENSUS_OUTPUTS` names an absent or empty directory, under
`<profile>/<flavour>/<corpus>/<path in the corpus>`, and beside each a `.unchecked.tsv` naming the
requirements this tree's validator says it did not check. A written output is one this tree passes:
with no departure in either shipped profile, `stands_as_departed` reduces to the second verdict's
`conforms`. `tools/verapdf-census.py` validates each at its own flavour, at most 25 files a veraPDF
process and three processes of 1500 MiB heap, a chunk that does not answer re-run one file at a time
under a time limit, and prints the rules veraPDF fails by rule and by document. A rule whose clause is
one at which this tree names an unchecked requirement is counted "checked in part": a clause holds
several requirements and the citation names only the clause, so that split says where to read first
and classifies nothing. `tools/state.sh archive` keeps the census's outputs in a scratch directory and
prints the tool's two count lines.

## 2. HEAD's run

Against HEAD's census, outputs kept (counts identical to HEAD's census line for line), veraPDF 1.30.3:
4997 outputs, 4850 compliant, 113 failing on 32 rules (17 clauses once ISO 19005-2's and -4's
numbering of one requirement are joined) over 35 documents, 34 outputs of 9 documents veraPDF did
not finish. 460 s under the lock at a load of 15 to 33, 4.11 GiB peak over the tree. The hypothesis
(under twenty rules, most on fonts and metadata) held by clause and not by rule: 18 of the 32 rules
are font rules, two are metadata.

Side the standard supports, a first reading: **veraPDF** (the file fails the clause; this tree's
checker has a gap), **ours** (the file meets it), **undecided** (named reading owed). P2 is ISO 19005-2
at 2b, P4 ISO 19005-4 at 4; both profiles unless said.

| rule (P2 / P4) | documents | what the output holds | side |
|---|---|---|---|
| 6.1.13-1 / — | 475121 | content-stream operands at the 64-bit integer limit, past 6.1.13's 32-bit bound | veraPDF |
| 6.1.13-11 / — | issue7872 | a `/MediaBox` of four indirect numbers, which veraPDF reads as 0 by 0 | ours, provisional (the referenced values unread) |
| 6.2.2-1 / 6.2.2-1 | 029016, calgray, calrgb | tokens veraPDF lexes as undefined operators in one content stream | undecided (whether the stream decodes alike in both) |
| — / 6.2.2-3 | T02-04-01_005, issue14438, issue17679, issue17679_2 | a font resource with no `/Subtype`, which veraPDF counts as undefined | undecided (defined by where it stands, ADR 2608; the dictionary fails 6.2.10.2, not 6.2.2) |
| 6.2.4.3-4 / 6.2.4.3-4 | T02-05-01_004, T02-05-01_008 | DeviceGray set with no output intent veraPDF finds | undecided (both are content streams missing an operator) |
| 6.2.6-1 / — | AIAA-2002-4016-606 (as-if-printed) | an image `/Intent` misspelling a standard name | veraPDF |
| 6.2.8.3-1 / 6.2.7.3-1 | issue19326 | a JPX image whose `cdef` types its second channel as opacity | ours (`doc/pdfa-blocking-issues/verapdf-jpeg2000-issue.md`) |
| 6.3.3-3 / 6.3.3-3 | issue15557 | a `Btn` widget whose `/N` maps both states to null, an empty dictionary by §7.3.7 | veraPDF, provisional |
| 6.6.2.3.1-1, -2 / — | PDFBOX-4532-reduced | `illustrator:` and `xmpTPg:HasVisibleTransparency` properties with no extension schema | veraPDF, provisional (ADR 1991's predefined set to be re-read) |
| 6.2.11.2-3 / 6.2.10.2-3 | issue1350 | text shown under a font veraPDF resolves to none | undecided (which clause a null font entry fails) |
| 6.2.11.2-4, -5, -6 / 6.2.10.2-4, -5, -6 | 0000490 | unembedded TrueType Arial with no `FirstChar`, `LastChar` or `Widths`, which ISO 32000-1 requires outside the standard 14 | veraPDF |
| 6.2.11.4.1-1 / 6.2.10.4.1-1 | bug1068432, bug1308536, bug860632, bug866395, extgstate, issue14462_reduced, issue4630 (and 477139 at P4) | subset Type 1 fonts veraPDF finds no program for | veraPDF, provisional (slot 1's cut programs; 6.2.11.2's program conformance is unchecked here) |
| 6.2.11.4.1-2 / 6.2.10.4.1-2 | Type3Test, Type3WordSpacing, bug920426, issue14881, issue7769 | a code the Type 3 encoding maps to no glyph procedure | undecided (what a Type 3 code with no glyph name references) |
| 6.2.11.4.2-1 / — | facct2025-final168, issue2462 | a Type 1 `CharSet` veraPDF finds unlike the program | undecided (the differing glyph unread) |
| 6.2.11.5-1 / 6.2.10.5-1 | AIAA-2005-6173-712, issue14881, issue3566, issue7769, issue7901 | a TrueType glyph width unlike the `Widths` entry by hundreds | undecided (which glyph the code selects, §9.6.5.4) |
| 6.2.11.8-1 / 6.2.10.9-1 | bug920426 | Identity-H codes whose CIDs map to glyph 0 | veraPDF, provisional (ADR 2582 reads a shown CID 0 as `.notdef`; these CIDs are not 0 and still select glyph 0) |
| — / 6.2.10.7-1 | issue3566 | a code veraPDF gives Unicode 0, U+FEFF or U+FFFE | undecided (whether a `ToUnicode` states it) |

Seven clauses read for veraPDF, two for this tree, eight undecided. veraPDF did not finish nine
documents' outputs: a validation exception on T02-02_007, bug1050040 and poppler-67295-0, an
unparsable stream on issue15590 and issue21436, a parse exception on T02-02_002, T02-03_001,
T02-03_002 and Pages-tree-refs — most of them page trees this tree rebuilt. None is counted either way.

Two disagreements other rounds of batch 131 found outside this census, evidence to weigh in batch 132:

- ISO 19005-2 6.2.2, the opposite direction (round 1902): veraPDF does not report the run
  `33600.272.58` as an operator, where this tree's fence does. §7.3.3 governs: the run is no number
  by its syntax, so it is an operator token and an undefined one; side **ours**, provisional.
- 6.2.11.4.1 on `178360` (round 1901): veraPDF fails `ICLJGJ+Glypha`, an unfiltered 1009-byte `Type1C`
  program whose header states minor version 0x20. The header is valid by TN 5176 section 6, but the
  program holds no `0x00` byte anywhere and its Name INDEX counts 8193 names: a whole-file corruption
  no loader reads. Side **veraPDF**; this tree's checker now fails a program it cannot load (ADR 2636
  decision 5) and the document is freed.

## 3. The plant

`helloworld.pdf`'s as-shown 2b output, copied with `pdfaid:conformance` set to `Z`, was put among the
outputs: the tool listed it alone at P2 6.6.4-3, counted "checked in part", because this tree names
6.6.4's amendment requirement as unchecked while it checks the conformance value at the same clause.
It is excluded from the counts above.
