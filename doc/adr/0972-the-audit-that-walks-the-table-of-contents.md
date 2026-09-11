# 0972 — The audit that walks the table of contents, and the annex nobody had read

Session 965. Status: **accepted**. It adds a structural audit of both owned parts to
`crates/pdf-archive`, and twenty-one rows for requirements that had no row at all — including the
whole of ISO 19005-2's normative Annex B and the method its normative Annex A states.

## 1. The blind spot two sweeps could not have found

Sessions 958 and 961 (ADRs 0956 and 0964) swept every `Check::Unchecked` and `Check::Processor`
reason in the table and found, between them, four stale claims out of forty-three. Both sweeps read
the rows **that exist**. The owner's standing correction points the other way:

> remember that the corpus is just a nice feedback but not the target. The target is the complete
> spec.

A requirement with no row is invisible to every instrument this crate has. It is not `Unchecked`,
so no sweep of reasons reaches it. It fails no corpus document, so `over` and `missed` are silent
about it. It is in no denominator, so the coverage census cannot count what it is missing from.
**That is a strictly larger blind spot than the one the reason sweeps work in**, and the only way
to see into it is to walk the standard's own table of contents rather than the table's.

That walk was made this session, subclause by subclause, over ISO 19005-2 clause 5, clause 6 and
Annexes A and B, and ISO 19005-4 clause 5, clause 6 and Annexes A and B — every heading of both
documents that states a requirement. Its result is `crates/pdf-archive/src/coverage.rs`.

## 2. What the audit is, and the one thing it deliberately is not

`coverage.rs` holds one `Subclause` per subclause of each part: its number, this crate's own
sentence saying what it is about, and one of five verdicts.

| verdict | means |
|---|---|
| `Bound` | at least one row of the table cites this subclause for this part |
| `Container` | a heading whose own text states nothing; its children carry the rules |
| `StatesNoRequirement` | a permission, a recommendation or informative prose — nothing a file or a processor could fail |
| `Scoping` | it says how the *other* requirements are read, and the table honours it in its shape rather than in a row |
| `Restated` | it repeats a rule the table already carries under another clause of the same part |

Five tests hold the audit and the table to each other. Two run in both directions — every `Bound`
subclause is cited by some row, and no subclause of the other four kinds is — so a row deleted or
re-keyed fails the build by name, and so does a later round writing a row against a subclause
somebody once recorded as silent. A third checks that every clause a row cites has an entry here at
all. A fourth checks that the two parts' entries stay in two contiguous runs, which is the
reviewability property the requirement table states for itself and never enforced. The fifth reads
the headings of `doc/pdfa/` where the machine running the tests has those files and asserts that
every clause 6 heading of each part is listed — and **skips, printing why, where they are absent**,
because those texts are licensed to a single reader (`doc/questions/A16`) and are not in the
repository. Nothing of their text is read, kept or printed; only the numbers of their headings.

**It is a subclause-level instrument and not a sentence-level one, and that is stated in the module
comment rather than left to be discovered.** A subclause with four normative sentences of which
three have rows is `Bound` here exactly as one with four out of four is. Sentence-level coverage
cannot be committed to this tree at all — it would have to enumerate the standard's sentences — so
what is available is the coarser claim, *checked*, plus a round's sentence-level reading recorded
in prose. Section 3 is that reading.

**A partial audit with a stated frontier would have been worth having; this one has no frontier
inside clause 5, clause 6 or the normative annexes.** Clauses 1 to 4 are scope, normative
references, terms and notation, and state no requirement; they are the one boundary, and the module
comment says so.

## 3. What the walk found: twenty-one requirements with no row

Every one of these is a normative sentence of a part this crate claims to check, and not one of
them had a row before this session. They are grouped by why they had been missed, because the
shapes differ and the shapes are the transferable part.

### 3.1 A whole normative annex, and a method the table depended on

**ISO 19005-2 Annex A** states, normatively, the method a conforming reader uses to decide whether
a page contains transparency. **ISO 19005-2 Annex B** states nine requirements about signing and
validating. Between them: **no citation anywhere in the table**, and nothing in the crate that
mentioned either. Part 4's annexes had rows (`A.2`, `B.2.1`, `B.2.2`, `B.2.3`); part 2's had none,
and nobody had noticed that the asymmetry was ours rather than the standards'.

Annex A is the sharper of the two, because **the table already depended on it**.
`graphics/a-transparent-page-has-a-blending-space` asks its question of "every page containing
transparency", and which pages those are is decided by `crate::survey` — whose doc comments
attributed the method to **ISO 32000-2 Annex Q**. Annex Q is PDF/A-4's base standard's. PDF/A-2's
base standard is ISO 32000-1:2008, which states **no such method at all**; what states it for a
PDF/A-2 file is ISO 19005-2's own Annex A. The two texts give the same steps and the same four
graphics-state conditions, so no verdict moves — which is exactly the shape ADR 0964 section 2
found one clause over, in the ICC edition rule, and which that ADR called invisible to every gate
because the answer was right. It is invisible for the same reason here. The comments are corrected
and a row now carries the method with the distinction written into its reason.

New rows: `graphics/transparency-determined-by-the-parts-own-method` (Annex A),
`signatures/digest-covers-the-whole-file`, `signatures/signature-is-a-single-signer-cms-object`,
`signatures/revocation-information-is-a-signed-attribute`, `signatures/signature-handlers-available`
(Annex B.1), `signatures/signatures-validated-as-the-annex-describes` (Annex B.2).

### 3.2 Clause 5, which is where a part says what conformance *is*

Two rows cited clause 5 already, so the clause looked attended to. It states three more things.

- **ISO 19005-4 section 5.1: a conforming file shall not use a feature the base standard describes
  as deprecated.** A requirement on the file, on no list, and a large one — ISO 32000-2 deprecates a
  great deal. `conformance/no-deprecated-features` is `Unchecked`, and its reason names the route
  and the reason the route does not close it: `pdf_spec`'s Arlington-derived model carries
  `deprecated_in` per *key*, while the clause's subject is a *feature*, and a deprecated filter, an
  action or a security handler is not one key. Reporting the keys alone would under-report by an
  amount nobody has measured, and calling that the clause would be this crate deciding what a
  feature is.
- **ISO 19005-2 section 5.1: the header's version number shall not be used in deciding
  conformance.** Addressed to whoever judges the file — which is this crate. It is honoured today
  and now says so.
- **ISO 19005-2 section 5.5 and ISO 19005-4 section 5.2** are the subclauses that make every other
  `Check::Processor` row binding on a program, and are the only place either part says what a
  conforming reader *is*. `conformance/processor-behaviour`.

### 3.3 Twelve processor obligations inside clauses the table already cited

These are the ones the subclause-level instrument would *not* have found on its own, and they came
out of the sentence-level pass. Each sits in a subclause with rows, so the subclause was `Bound`
and the sentence was not.

- `file-structure/undescribed-data-never-renders` (both, section 6.1.1) — the standing instruction
  about everything the two standards do not describe. Its subclause had no row of any kind.
- `file-structure/unreferenced-objects-never-influence-rendering` (both, section 6.1.4) — the
  exemption for an object no cross-reference section names, and the rule for a processor that reads
  one anyway. The exemption half is one this crate *obeys in its own populations*, which
  `table/file_structure.rs` already said and no row recorded.
- `file-structure/linearization-permitted` (both) — its subclause had no row either.
- `graphics/destination-profile-alternate-ignored` (both, section 6.2.3) — distinct from the
  ICCBased `Alternate` rule one subclause along, which is about a colour space's profile rather than
  the output intent's.
- `graphics/jpeg2000-best-colour-space-specification-used` (both) — the processor half of a sentence
  whose file half was already a row.
- `graphics/blend-modes-processed-as-the-base-standard-defines` (both).
- `annotations/appearance-rendered-without-the-other-entries` (part 2 only — part 4 states no
  equivalent sentence).
- `signatures/signatures-use-signature-fields` (both) and
  `signatures/signing-does-not-break-conformance` (both), from the digital signature subclause that
  had exactly one row against three sentences.
- `signatures/timestamped-file-follows-the-base-standard` (ISO 19005-4 section 6.5.3) — a subclause
  with two `shall`s and no row at all.
- `actions/a-processor-that-declines-scripts-says-so` and
  `actions/on-instantiate-script-only-on-explicit-user-action` (ISO 19005-4 Annex B.3).

### 3.4 What the walk found to be correctly absent, which is also a result

Eleven subclauses of part 2 and seven of part 4 state no requirement a file could fail, and having
that *recorded* is the point: a later round that writes a row against one of them fails a test
instead of quietly widening the table. They are the `should`-only subclauses of part 2's logical
structure (artefact marking, alternate descriptions, non-textual annotations, replacement text,
abbreviation expansions), both parts' namespace-prefix tables, part 4's validation recommendations,
its geospatial and measurement permissions, and its whole account of logical structure — which has
no `shall` in it, because part 4 has no conformance levels for one to be conditioned on.

Two more are worth naming because they look like gaps and are not. **ISO 19005-2 section 6.1.5
restates section 6.6.3's rule** about the document information dictionary, and one rule gets one
row. **ISO 19005-4 section 6.2.10.4.2 states no requirement**, where part 2's section 6.2.11.4.2
states two: the `CharSet` and `CIDSet` rules are part 2's alone, and the table already had that
right.

## 4. Why every new row is `Processor` or `Unchecked`, deliberately

Fifteen of the twenty-one are `Check::Processor` and six are `Check::Unchecked`. None is a
predicate, and that is a decision rather than an economy.

- **A new predicate can move `over`.** The column that matters is this crate failing a document its
  author built to conform, and a round that adds a check and a verdict in the same breath cannot
  tell a finding from a regression.
- **A new predicate is a new row of the converter's census.** `pdf_transform::archive::unconsidered`
  computes its list from requirements whose check is a predicate a document can fail; `Processor`
  and `Unchecked` rows never enter it. So this change leaves
  `crates/pdf-transform/tests/archive_unconsidered.txt` empty and untouched — checked by running
  that crate's `archive` test — while a predicate would have owed the neighbouring crate a decision
  row in the same commit.

Two of the six `Unchecked` rows are one predicate away, and their reasons say so exactly, which is
what `doc/questions/A20` asks of a reason. `pdf_model::signature::Signature::coverage` already
returns `Coverage::WholeFile` for precisely the `/ByteRange` ISO 19005-2 Annex B.1 requires, and
`pdf_model::cms::SignedData` already states `signers` and the entries of `certificates`, which is
that annex's single-signer-and-a-certificate sentence. Closing either is a round with the
converter's decision table open beside it.

## 5. The habit this leaves, which is the general form of section 1

**An audit of the rules that exist cannot find a rule that does not.** Every instrument this crate
had — the reason sweeps, the corpus harness, the coverage census, the identifier and clause tests —
takes the table as its population. A gap in the table is outside all of their denominators at once,
and no amount of running them harder reaches it. The only instrument that can is one whose
population is the **standard's table of contents**, and the shape that makes it durable is a
committed list held to the table in both directions by a test.

The same argument applies wherever this project holds a requirement table against a document it
does not own: the conformance ledger's relationship to ISO 32000-2 is the same relationship, and
`tools/conformance` already walks the standard's clause numbers for exactly this reason. What was
missing here was the walk, not the idea.
