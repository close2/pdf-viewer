# 62 — The exemption no row stated: a named resource nothing references

Status: **taken**, session 1001 (ADR 1021), after ADR 0935 deferred it with an argument and ADR
0941 measured what was being deferred. What blocked it was named in this file for three sessions —
"a whole-file reachability answer available to every row, which is a change to `Examination`'s
contract rather than to any one predicate" — and that answer is now `crate::reach`.
Priority: **closed**; what is left of it is one sentence, §5 below, and it is owed by the base
standard rather than by this item. Companions: `doc/todo/01` (the ledger sweeps), `doc/todo/48`
(the copy of the standard).
Corpus: `doc/veraPDF-corpus`, all six targets. **The numbers are not written down here**: run
`cargo run --release -p pdf-archive --example unreferenced -- doc/veraPDF-corpus`, which prints
them per target and names every witness.
Clauses: ISO 19005-2 section 6.2.2's last sentence, ISO 19005-4 section 6.2.2's last sentence,
`TechNote 0010` A010; §7.8.3 for what a resources dictionary is, and `TechNote 0010` A003 for which
of them is *explicitly associated* with what.
Code: `crates/pdf-archive/src/reach.rs` (the walk, the population and the carve-out),
`crates/pdf-archive/src/lib.rs` (`judge`, where a failing requirement is narrowed),
`crates/pdf-archive/src/finding.rs` (`Findings::exempting`, `named_an_object`),
`crates/pdf-archive/tests/reach.rs` (the fixtures §4's fourth point asked for),
`crates/pdf-archive/examples/unreferenced.rs` (the census, now a printer over the method),
`crates/pdf-archive/tests/unwitnessed.rs` (a fail/pass pair for each row `examples/withdrawn.rs`
finds no corpus document failing — §7's other half, pinned where the corpus cannot rank it),
`crates/pdf-archive/src/clarification.rs` (A010's record).

## 1. What the two parts say, and what this crate does about it now

Both target parts close section 6.2.2 with the same exemption, in almost the same words: a named
resource that is present in a resources dictionary but whose name the associated content stream
does not reference is not used for rendering and is therefore exempt from the part's requirements.
They differ in one place, and that difference was the whole subject of this item:

| | what the exemption keeps | where that comes from |
|---|---|---|
| ISO 19005-2 | **nothing** as published; 6.1.2 to 6.1.13 once A010 is read | text; A010 |
| ISO 19005-4 | sections 6.1.6 to 6.1.9 | text, no clarification |

**Both now hold.** Half always had, by construction: the rows that read `crate::survey` never see
a resource nothing references, because the walk reaches a form `XObject` only through the `Do`
that names it, and a colour space only through the operator that selects it. The other half is
`crate::reach::Exempt` — the objects reachable *only* through a named resource entry the
associated content stream does not reference — narrowing every row whose clause falls outside the
applicable carve-out, which `reach::exemption_narrows` reads and `crate::check` applies.

## 2. How the population is computed, and the three things to know before trusting it

- **Association is A003's**, not the resources dictionary in force. A dictionary belongs to the
  stream that states it, to the page whose `/Contents` it governs, or to the glyph procedures of
  the Type 3 font that states it. A dictionary with no such owner — an `/AcroForm` `/DR` is the
  standing case, and a `/Type /Pages` node holding what its descendants inherit is the dangerous
  one — exempts nothing, because the clause's premise is an *associated content stream* and there
  is none. The measurement that ran from session 944 to session 1001 had this wrong for the
  second case and over-exempted; ADR 1021 §4 has the correction and what it moved.
- **Every name operand counts as a reference**, with no operator table, and a dictionary two
  owners share carries the union of their names. Both can only make the exempt set smaller, so the
  answer is a floor rather than a ceiling.
- **A bound refuses rather than truncates.** If either walk stops at `reach::EDGES` or
  `reach::DEPTH`, the exempt set is empty and `stopped_at()` names the limit, so every row judges
  every object exactly as it did before the module existed. Over `doc/veraPDF-corpus` no document
  reaches either.

## 3. What the measurement found, and why it decided the order

Session 944 measured this before it was implemented, and the finding is what made A010 a
precondition rather than a refinement. Every document whose **entire verdict** turns on an object
the exemption reaches fails under a clause the applicable carve-out **keeps**: the part 4
witnesses under sections 6.1.6.1 and 6.1.7, inside part 4's own published 6.1.6 to 6.1.9; the part
2 witnesses under sections 6.1.7.1 and 6.1.13, inside A010's 6.1.2 to 6.1.13.

Two consequences, and the second is why this file said "in the same commit or not at all":

- **Implementing the exemption cost the corpus nothing**, and session 1001 confirmed it from the
  other side by running the validator's corpus gate with the narrowing switched off and on: the
  same six columns on all six targets, `over` zero throughout. So the sweep could not rank this
  work — `CLAUDE.md`'s two denominators, a coverage question the robustness instrument is blind to.
- **Part 2's published sentence *without* A010 would have withdrawn real failures**, every one of
  them a `-fail-` document this crate agrees with the corpus about. A010 is what makes part 2's
  exemption safe, and the two landed together.

The direction of risk is asymmetric and worth restating, because it is not what a reader expects
of a rule that adds a permission: `over` cannot move, since the exemption only ever withdraws
failures. Every unit of risk is in `missed`, where a validator that has quietly stopped checking
something is indistinguishable from one that never checked it. That is why the population
under-exempts on all three counts above, why a bound refuses, and why §4's fixtures exist.

## 4. What taking it needed, and where each part landed

1. **A reachability answer on `Examination`**, not a predicate — `Examination::reaches` and
   `Examination::exempt`, ADR 1021 §2.
2. **The carve-out expressed by clause, not by row identifier** — `reach::exemption_narrows`,
   which reads a requirement's `Clauses` for the target's part.
3. **A010's second sentence**, an unreferenced resource shall still conform to the base standard —
   §5 below, and the reason section 5.1 is kept out of the narrowing for both parts.
4. **A test that pins the reach**, because the sweep will not —
   `crates/pdf-archive/tests/reach.rs`, seven fixtures: the path to a page, an object nothing
   names, a resource nothing invokes, a resource something invokes, a failure the exemption
   withdraws, a failure the carve-out keeps, and the two carve-outs' ranges.

## 5. What is left, and whose it is

A010's second sentence is the one part of this work that can *add* a failure, and the corpus has
no witness for it at all. It is honoured in shape — `reach::exemption_narrows` keeps section 5.1
out of the narrowing, so the exemption never withdraws a base-standard finding — and unenforced in
fact, because `conformance/adheres-to-the-base-standard` is `Check::Unchecked`, as the whole of
the base standard is here. **That is not this item's debt.** It is `doc/PLAN.md` §5a's conformance
ledger, which is where a claim about this program's reading of ISO 32000 belongs, and the row says
so in its own words.

## 6. What this item was not

It was not A010. A010 is one sentence of it — the carve-out — and the thing missing from this
crate was the published exemption both parts state. Recording it under A010's name is what let it
sit as a line on a note's owed list for three sessions while being, in fact, a requirement of the
standard that no row of this table stated. The general shape is worth keeping: **a clarification
whose subject is a population rather than a rule has no row to hang on**, which is why
`clarification::CLARIFICATIONS` has no entry for A010 and why the record is that module's prose.

## 7. What session 1007 added, and the two questions it separated

This item was closed by session 1001 and stays closed; what follows is the *audit* of what it
built, which the closing round could not do from inside the same commit.

**The reach is now measurable per requirement**, and not only per document:

```sh
cargo run --release -p pdf-archive --example withdrawn -- \
  doc/veraPDF-corpus doc/pdf.js doc/corpora/pdf20examples doc/corpora/pdf-differences \
  doc/corpora/pdfbox doc/corpora/format-corpus doc/corpora-own
cargo run --release -p pdf-archive --example withdrawn -- --threads 6 --max-mb 96 \
  --targets 2a,4 corpus-cache/openpreserve corpus-cache/tika-issue-tracker \
  corpus-cache/safedocs/cc-main-2021-31/*/
```

**Name the subdirectories rather than the corpus on a walk that long.** A report is printed per
root, so a walk stopped by a bound keeps what it has; session 1007 lost 87 000 documents' worth of
answer to one `RLIMIT_DATA` because it passed `corpus-cache` as a single root (ADR 1026 §5.2).

`examples/unreferenced.rs` counts the *population* — documents stating a named resource nothing
references, and the objects only such an entry reaches. `examples/withdrawn.rs` counts what that
population does to the **requirement table**: per row identifier, in how many documents the
predicate found a place, how many of those the exemption withdrew some or all of, and how many
stood. Its bounds are reported rather than applied in silence (unreadable, oversized, panicked,
slower than ten seconds, skipped by name), and it prints beside every narrowed row what
`crate::withdrawal` says that row's subclause is — so a row narrowed under a subclause the reading
calls *not a resource* is a contradiction the run itself raises.

**The per-subclause reading is `crates/pdf-archive/src/withdrawal.rs`.** `exemption_narrows` keeps
two carve-outs and narrows everything else with one `else` arm, which is what the two published
sentences say and which makes three different situations look identical: a subclause whose subject
*can* be a named resource, one whose subject is the file or a page so that the narrowing is a
fall-through that cannot fire, and one no document can fail at all because every requirement in it
binds a processor. Every bound subclause of both parts now says which of the four it is, with the
reason, and four tests hold the reading to `coverage`, to `exemption_narrows` and to the table.

**Two questions this separates, and they are `CLAUDE.md`'s two denominators again.** *Can* the
exemption reach a subclause is a question about the clause and the answer is in `withdrawal.rs`.
*Did* it reach one is a question about documents and the answer is the command above. A subclause
the first calls reachable and the second never witnesses is not an error in either — it is
`doc/questions/Q62`'s shape, and the round that reads one owes a look at the other.
