# 62 — The exemption no row states: a named resource nothing references

Status: **measured and deferred**, session 944 (ADR 0941), which is what ADR 0935 left owed when it
deferred `TechNote 0010` A010 with an argument and no number. The size is now a command's output
rather than a guess, and the measurement changed the shape of the work: see §3.
Priority: 50-band, by ADR 0935's own reason — this is blocked on an infrastructure this crate does
not have, a whole-file reachability answer available to every row, which is a change to
`Examination`'s contract rather than to any one predicate. Companions: `doc/todo/01` (the ledger
sweeps), `doc/todo/48` (the copy of the standard).
Corpus: `doc/veraPDF-corpus`, all six targets. **The number is not written down here**: run
`cargo run --release -p pdf-archive --example unreferenced -- doc/veraPDF-corpus`, which prints it
per target and names every witness.
Clauses: ISO 19005-2 section 6.2.2's last sentence, ISO 19005-4 section 6.2.2's last sentence,
`TechNote 0010` A010; §7.8.3 for what a resources dictionary is, and `TechNote 0010` A003 for which
of them is *explicitly associated* with what.
Code: `crates/pdf-archive/examples/unreferenced.rs` (the measurement),
`crates/pdf-archive/src/clarification.rs` (A010's entry), `crates/pdf-archive/src/examination.rs`
(`objects`, the population the exemption would narrow), `crates/pdf-archive/src/survey.rs` (the
half that already holds by construction).

## 1. What the two parts say, and what this crate does about it

Both target parts close section 6.2.2 with the same exemption, in almost the same words: a named
resource that is present in a resources dictionary but whose name the associated content stream
does not reference is not used for rendering and is therefore exempt from the part's requirements.
They differ in one place and the difference is the whole subject of this item:

| | what the exemption keeps | where that comes from |
|---|---|---|
| ISO 19005-2 | **nothing** as published; 6.1.2 to 6.1.13 once A010 is read | text; A010 |
| ISO 19005-4 | sections 6.1.6 to 6.1.9 | text, no clarification |

**No row of this crate states either one.** Half of the exemption holds here by construction and
always has: the rows that read `crate::survey` never see a resource nothing references, because the
walk reaches a form `XObject` only through the `Do` that names it, and a colour space only through
the operator that selects it. The other half does not. `Examination::objects` is every object a
cross-reference section names, and the rows built on it — the filter rows, the font rows, the image
rows, the string and name limits — judge an unreferenced resource exactly like a used one.

## 2. The measurement, and how it was made

`examples/unreferenced.rs` computes, per document, the objects reachable **only** through a named
resource entry the associated content stream does not reference — a reachability question over the
whole file, which is the thing ADR 0935 says no per-row predicate can answer — and then asks which
of the report's failures land on nothing else.

Three properties of it are worth knowing before its output is read:

- **Association is A003's**, not the resources dictionary in force. A dictionary belongs to the
  stream that states it, to the page whose `/Contents` it governs, or to the glyph procedures of the
  Type 3 font that states it. A dictionary with no such owner — an `/AcroForm` `/DR` is the standing
  case — exempts nothing, because the clause's premise is an *associated content stream* and there
  is none.
- **Every name operand counts as a reference**, with no operator table. That can only make the
  exempt set smaller, so the output is a floor rather than a ceiling.
- **It reports a row only where the whole row is exempt** and the finding list is not a prefix, and
  it says separately whether the document fails anything else — because a document that fails a
  second row on a used object does not change verdict however the exemption is read.

## 3. What it found, and why it reverses the order the work was going to be done in

Every document the tool names is one whose **entire verdict** turns on an object the exemption
reaches — none of them fails anything else. And every one of them fails under a clause the
applicable carve-out **keeps**: the part 4 witnesses under sections 6.1.6.1 and 6.1.7, inside part
4's own published 6.1.6 to 6.1.9; the part 2 witnesses under sections 6.1.7.1 and 6.1.13, inside
A010's 6.1.2 to 6.1.13.

Two consequences, and the second is the one that matters:

- **Implementing the exemption as it stands today costs the corpus nothing.** Part 4's published
  carve-out and part 2's under A010 retain every failure the corpus currently ranks. So the sweep
  cannot rank this work either — `CLAUDE.md`'s two denominators again, a coverage question the
  robustness instrument is blind to.
- **Implementing part 2's published sentence *without* A010 would withdraw real failures**, every
  one of them a `-fail-` document this crate agrees with the corpus about today. ADR 0935 recorded
  A010 as a narrowing to apply after the exemption it modifies, and reasoned that taking A010 first
  would be "implementing nothing". The measurement says the opposite: **A010 is what makes part 2's
  exemption safe, and the two have to land in the same commit or not at all.**

The direction of risk is therefore asymmetric and worth stating plainly, because it is not what a
reader expects of a rule that adds a permission. `over` cannot move: the exemption only ever
withdraws failures, and `over` is already zero on all six targets. Every unit of risk is in
`missed`, where a validator that has quietly stopped checking something is indistinguishable from
one that never checked it.

## 4. What taking it needs

1. **A reachability answer on `Examination`**, not a predicate. Something of the shape *is every
   route to this object an unreferenced named resource entry* — which the example computes today
   and which a row cannot, because a row sees one object and the question is about the file.
2. **The carve-out expressed by clause, not by row identifier.** Part 2 keeps sections 6.1.2 to
   6.1.13 and part 4 sections 6.1.6 to 6.1.9, so the filter belongs where a requirement's clause is
   known — `Clauses`, or the place a judgement is assembled — rather than inside the rows.
3. **A010's second sentence, which is a requirement rather than an exemption**: an unreferenced
   resource shall still conform to the base standard. That is the one part of this work that can
   *add* a failure, and it is the one part the corpus has no witness for at all.
4. **A test that pins the reach**, because the sweep will not. ADR 0933 and ADR 0935 both had to do
   this for the same reason, and both said so.

## 5. What this item is not

It is not A010. A010 is one sentence of it — the carve-out — and the thing missing from this crate
is the published exemption both parts state. Recording it under A010's name is what let it sit as a
line on a note's owed list for three sessions while being, in fact, a requirement of the standard
that no row of this table states.
