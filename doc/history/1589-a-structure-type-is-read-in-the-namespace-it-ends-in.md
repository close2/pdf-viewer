# 1589 — A structure type is read in the namespace it ends in, and the default one is PDF 1.7's

Slot 5 of batch ninety-three, 2026-10-19, a clause round. ADR 2014; no question.

**Premise and hypothesis.** The premise held: `Tree::standard_role` followed §14.8.6.2's walk and
then read the name in PDF 2.0's vocabulary whatever the namespace, so Annex M's eleven PDF 1.7 types
were never standard with no `/NS`, though the §14.8.6.1 row said `implemented`. The hypothesis held
in its first half and not its second: not a dozen documents but 1 305 tagged pages carried them —
`Art` 1 182, `Reference` 653, `TOCI` 552, `TOC` 60, `Note` 34, `Code` 28, `BlockQuote` 8 elements as
a host is told them — and the copy's logical order gains no paragraph break: 17 contents pages go from
52 block ends to 443, but the readback had already broken their lines, so the copy differs on 10 pages
only by each contents line's trailing space.

**Built.** `StandardType` gains the eleven with ISO 32000-1's categories (grouping for Table 333's
six, inline for Table 338's five); `StandardNamespace`; `StandardType::read_in` and `defined_in`
(Annex M's lists; `since_pdf_2_0` counts `Hn` past six); `Tree::standard_role` reads an explicit
standard namespace in its own vocabulary and no namespace in PDF 1.7's, a PDF 2.0 type with no `/NS`
read as PDF 2.0's by choice (ADR 2014 section 3: `tagged_copy`'s `Artifact` fixture and the part 2
leniency already recovered, and the corpus has none). `viewer-accessibility` maps the eleven;
`pdf_archive::standard_structure_element` is now `standard_role`'s answer and its own copy of Annex
M's list goes (slot 1's file, `-U0` hunks). The census prints three new counts by name.

**Rows.** §14.8.6.1 `implemented` → `implemented`, its note now true: "When a namespace is not
explicitly specified for a given structure element or attribute, it shall be assumed to be within
this default standard structure namespace." §14.8.6.2 `implemented` → `implemented`, note extended.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-model -p
viewer-accessibility -p viewer-core -p pdf-archive --all-targets`: exit 0. `nextest` of those four:
2 852 passed, 0 failed. `cargo test -p conformance`: 440 passed, 0 failed. `tools/state.sh docs`: exit
0. In the export (HEAD plus this round's four files): `t3-accessibility` exit 0 — elements a host is
told no standard namespace defines 2 548 on 1 310 tagged pages → 31 on 5 (MathML's and two foreign
names), PDF 2.0 types read with no namespace 0, every other count identical, no ratchet moved;
`t3-selection` exit 0 (it copies nothing); `t3-text_extract` (`--clock`) exit 0, 4 passed; the six
arms, cpu, gpu and compute at 1× and 4×: 968 and 965 page digests each, every one equal to
HEAD's arms'. Planted: the old reading back in `standard_role` fails
`a_name_is_read_in_the_vocabulary_of_the_namespace_it_ends_in`.

**Unfinished.** `Private`'s descendants are still presented; a host reads the role name alone, so a
foreign namespace's homonym maps as a standard type (ADR 2014 section 5, `viewer-core`'s). Duration:
9 100 s.
