# 1010 — The frontier read to zero, a sentence carried on a ground the subclause does not state, and the edition sweep that became a test

Session 989. Status: **accepted**. It takes the sentence-level audit of `crates/pdf-archive` over
the last forty-one subclauses of both owned parts and leaves no subclause with text in it unread;
gives `coverage.rs` the variant two rounds said the second row of its kind should buy; and turns
session 975's base-standard sweep — eight numbers found by hand and a sentence saying the rest
agree — into a table held against every `§` in the crate by a test.

Context: `crates/pdf-archive/src/coverage.rs`, `src/editions.rs` (new), `src/table/metadata.rs`,
`src/target.rs`, `examples/frontier.rs`; ISO 19005-2 clauses 6.6 to 6.11, ISO 19005-4 clauses 6.7
to 6.15; ISO 32000-1:2008 against ISO 32000-2 across every number this crate cites; ADRs 0972,
0981, 0986, 1003.

## 1. The last forty-one subclauses, and the one sentence with no row

`cargo run -q -p pdf-archive --example frontier` printed forty-one subclauses with no
sentence-level reading at the start of the round — both parts' metadata, logical structure,
embedded files, optional content, presentations and `Requirements` subclauses, and ISO 19005-4's
6.13 to 6.15 — and prints `none` now. The figures are the example's: 176 subclauses read against
135, 571 normative sentences against 435, and the example lists what carries each.

Depth was the instruction and depth is what the pass paid for: **a `Scoping`, `Restated` or
`StatesNoRequirement` verdict is a claim about every sentence of a subclause**, and ADR 0981 found
two of those hiding a processor obligation apiece. So every subclause got a verdict per normative
sentence, the eleven recorded at subclause level as stating no requirement included. Three of the
verdicts that were not `By` are worth the argument, because each could have gone another way:

- **ISO 19005-2 section 6.6.2.3.3 has a fourth normative sentence between its tables, and no row
  carried it.** The subclause was `Bound` — three rows cite it — and the sentence sits between
  Table 4 and Table 5: a `pdfaProperty:valueType` names a value type the XMP Specification
  defines or a custom one the same extension schema defines. `extension_schema_container_fields`
  holds the field present and spelled with its prefix and reads nothing of what it names, which
  the round checked in the predicate rather than in its doc comment. It is
  `metadata/extension-property-value-types-are-defined`, `Check::Unchecked`, part 2 only, with a
  reason that prices it exactly: two vocabularies this crate does not hold *as names* — the XMP
  Specification's value types, which `PREDEFINED` records per property as a shape and a lexical
  form rather than by the type's name, and the schema's own `pdfaType:type` entries — both
  countable from documents this tree holds. veraPDF checks it (its `6.6.2.3.3` tests 8 and 17),
  which is evidence the sentence is checkable and nothing more. It is the only new requirement in
  a hundred and thirty-six sentences, and it is the shape ADR 0981 section 3.1 named: a `Bound`
  subclause is exactly as able to hide a sentence as a silent one.

- **Both parts' optional-content subclauses say that each element of `Configs` defines a single
  variant, and the verdict is `Restated` at section 5.1 rather than a row.** Read as a rule on the
  file, what could break it is an element that is not a configuration dictionary — which the base
  standard's own Table 100 already forbids and `conformance/adheres-to-the-base-standard` carries
  — since one configuration dictionary is one variant by the subclause's own definition two
  sentences earlier. A row here would have been a second row for a type rule. veraPDF has none for
  the sentence either, which raises confidence in the reading without being its ground.

- **Both parts' NOTE 4, that the font rules reach every font in any optional content whether or
  not it is rendered, is recorded as `Scoping` even though a NOTE is not normative**, because it is
  a claim about a population this crate builds and the claim is checkable: `table::fonts` visits
  every font dictionary the cross-reference table reaches, and `crate::survey` walks every
  marked-content sequence without asking its group's state. A round that later teaches the survey
  to skip hidden content would be changing what this sentence records, and now there is a
  sentence to find.

Two things the pass confirmed rather than found are also results. ISO 19005-4 section 6.7.2.1
asks one thing of a packet — well-formedness as ISO 16684-1 defines it — where ISO 19005-2 section
6.6.2.1 asks two, conformance to the XMP Specification and well-formedness; `table::metadata`'s
module comment argued that split in session 946 and the sentence count now bears it out, with the
data-model and describe-one-resource rows binding part 2 alone. And part 4's `pdfaid:rev` and
E-or-F conformance property map sentence for sentence onto the five identification rows, with
nothing left over.

## 2. `Carried::Clarified`, and a third ground nobody had named

ADR 0986 section 5 met a row that binds a part on a clarification rather than on the part's own
text — `graphics/named-resources-are-defined`, PDF/A-2, `TechNote 0010` A002 — and recorded it as
a `Carried::By` whose sentence opened "not part 2's own sentence", saying the second such row
should buy a variant. ADR 1003 section 6 met the second — ISO 19005-4 section 6.3.3's
have-an-appearance row, which rests on that subclause's NOTE 1 — and declined to buy it, on the
argument that a NOTE is not a clarification. Both were right about the shape and this round found
the third, which settles the design: `embedded-files/associated-file-media-type` carries
ISO 32000-2 §14.13.2's rule under ISO 19005-4 section 6.9, on nothing but section 5.1's delegation
— no resolution, no NOTE, and the subclause neither states the rule nor points at it.

So the variant is `Carried::Clarified { by, ground }`, and `Ground` is what the three have in
common and where they differ: `Resolution(item)` for a clarification `crate::clarification`
carries, `Note(_)` for a NOTE of the subclause that says where the rule is, and
`BaseStandard(_)` for a clause of the base standard the part's clause 5 makes binding with
nothing in the subclause pointing at it. A `By` is a claim that the subclause's own words say so
— the claim a reader checks against their copy — and the whole reason for the variant is that a
disclaimer inside the one field a reader compares with the standard is the wrong place for it.

Two tests hold it. `a_clarified_sentence_rests_on_the_ground_it_names` asks
`clarification::clarifying` for every row under a `Resolution` and fails if the item differs, so
the reading and the clarification table cannot drift apart; and it refuses any `By` whose sentence
begins "not ", which is the shape the variant retired. The three existing tests that walked
`Carried::By` now walk both variants through one `rows_named` helper, so a `Clarified` row is held
to citing its clause for its part exactly as a `By` row is.

## 3. The base-standard sweep, finished, and held by a test

ADR 0986 section 4 resolved every `§` in this crate in both editions' heading lists by hand, found
eight numbers that resolve in ISO 32000-1:2008 to a different subclause, wrote each file's share
into that file's module comment, and closed each table with "everything else this file cites was
checked and agrees" — a claim about a sweep nobody could re-run, which is `CLAUDE.md`'s definition
of a fact that should have been a command. `crate::editions` is the command.

`SHIFTS` is one table for the crate: every cited ISO 32000-2 number whose counterpart in ISO
32000-1:2008 is not the same number under the same title, in three shapes — `Numbered`,
`NumberedAndRetitled`, `Absent` — with the 2008 number's own subject named wherever the number
exists there. `every_citation_resolves_in_the_edition_a_part_two_file_adheres_to` reads both
conversions' headings, scans every `§` in `src/`, `examples/` and `tests/`, and holds the table
both ways: a citation not in it must resolve in 2008 to a heading of the same title, an entry in
it must still be needed, and a `§` after the spelled-out `ISO 32000-1` fails outright (ADR 0997
section 2's rule, enforced for the one crate that owns both editions' concerns). It fails by name
with both editions' titles printed.

**Run over the crate, the mechanical sweep found nineteen shifted numbers where the hand sweep had
found eight.** The eleven it added, none of which contradicts session 975's result:

| ISO 32000-2 | in ISO 32000-1:2008 | why the hand sweep did not list it |
|---|---|---|
| §9.9.1 | 9.9 itself, unnumbered | a *General* subclause the 2008 edition does not number |
| §10.5 | 10.4 | cited once, in a reason rather than a row; 2008's 10.5 is *Halftones* |
| §12.8.3.4, §12.8.5 | absent | PDF 2.0's CAdES and document timestamps; 2008's 12.8.5 is *Legal content attestations* |
| §12.10 | absent | PDF 2.0's geospatial features; 2008's 12.10 is *Document requirements* |
| §13.6.3 | 13.6.3 | not shifted at all: a parser miss, because the title opens with a digit |
| §14.13.1, §14.13.2 | absent | PDF 2.0's associated files |
| §Q.2 | absent | ISO 19005-2 states the method itself, which ADR 0972 already said |
| §5 | — | `target.rs`'s "one of §5's three levels", which is ISO 19005-2's clause 5 and read as ISO 32000-2's *Version designations* — the residue ADR 0997 section 3 named, now spelled out |

Every row that binds a part 2 target cites, for the shifted ranges, a number in the table; the
`Absent` rows are all PDF 2.0 subjects cited from part 4's own subclauses, and each entry says so.
The per-file tables in `table::fonts`, `table::graphics` and `survey` stay where a reader meets
the numbers, with one sentence apiece pointing at the test that now keeps them true.

## 4. Two things the instrument taught while being built

- **A test that scans sources for citations scans its own source.** Its first run failed on `§14`
  — a bare clause number the scanner's own comment used as an example — and its second on
  `§12.11`, which the table's own `subject` string used to name what 2008's 12.10 is about.
  Excluding the file would have blinded the test to a real citation in it, so the prose was
  rewritten to keep its examples out of its population. The general form: an instrument whose
  population is *text* reads its own explanation of itself, and the explanation is written after
  the instrument is, by somebody thinking about the rule rather than the scan.
- **A converted standard with no heading markers cannot be parsed by shape alone.** The 2008
  conversion prints headings as plain lines, and a body line opening with a bare number is a table
  row far more often than a clause — "14 PNG prediction (on encoding, PNG Paeth on all rows)" is
  Table 8's, and it is what the parser first returned for clause 14. Top-level clauses are taken
  from the contents page, whose dot leaders mark it, and dotted numbers from the body with a title
  that opens the way a title does. Trap 1's rule about instruments, one directory over: the parser
  was right about 741 headings and wrong about the one the test happened to ask for.

## 5. What did not change

- **`over` is 0 on all six targets**, and the one standing miss (`6-6-2-3-3-t03-fail-b`, errata
  A029) is unchanged. The one row added is `Check::Unchecked`, so no document's verdict can move.
- **`crates/pdf-transform/src/archive/decision.rs` and `tests/archive_unconsidered.txt` are
  untouched**: no row was promoted to `Check::Implemented`, so nothing entered
  `pdf_transform::archive::unconsidered()`.
- **One requirement identifier added**, `metadata/extension-property-value-types-are-defined`;
  none removed, re-keyed or promoted. Two rows changed no identifier and no check, only the
  sentence of the reading that names them.
- `crates/pdf-model/src/der.rs` and `signature.rs` were read and not edited. The single-signer
  row's blocker stands as ADR 1003 section 7 priced it — *DER-encoded* is what `pdf_model::der`
  deliberately does not enforce — and this round adds nothing to that price.
