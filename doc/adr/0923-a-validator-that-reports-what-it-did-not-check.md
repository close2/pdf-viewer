# 0923 — A validator whose product is a citation, and which reports what it did not check

Session 940. Status: **accepted**. `crates/pdf-archive` decides whether a document conforms to a
part and level of ISO 19005 and says which requirement it fails, where, and — the part that
shapes everything — which requirements it did not look at.

## The four decisions

### 1. The requirements are data, and one table covers both owned parts

`python3 tools/pdfa-text.py --overlap` counts how much of ISO 19005-2's and ISO 19005-4's clause 6
is the same rule; most of it is. So a shared rule is one predicate with two citations
([`Clauses::both`]) rather than two implementations, and **a level is an applicability column
rather than a build**: ISO 19005-2 §5.3 and §5.4 state their own exemptions, and ISO 19005-4's
annexes modify clause 6 mostly by relaxing it. Six targets, one table.

That was the owner's question and the measurement answered it. What stays sequenced is
*completion* — PDF/A-4 first, because its base document is the edition this tree owns.

### 2. A requirement that is not checked is named in the verdict

`doc/questions/Q20`'s discipline, and it is structural rather than a convention: `Check` has an
`Unchecked(&'static str)` variant carrying the reason, `Report::render` prints the section even
when it is empty, and every report states "*n* of *m* requirements checked" beside the verdict.
`Verdict::Conforms` is deliberately not named `Conformant`.

Both parts end their identification subclause by saying the `pdfaid` properties do not themselves
determine conformance. A validator that answered a bare yes would be making the same mistake in
the other direction.

### 3. The errata are an input, not a footnote

`errata.rs` carries the approved PDF Association corrections by requirement id. Only entries
labelled `ISO approved` count — an open issue is a question, and adopting one would be
implementing a proposal. One of them, #314, **withdraws** a rule from part 4, so `table::binding`
removes it rather than letting it pass: a withdrawn rule that "passed" would still be counted in
the coverage figure.

Errata exist for ISO 19005-4 alone. TechNote 0009 and 0010, which clarify parts 1 to 3, both
return HTTP 403 and are therefore not implemented from — recorded where it costs something, in the
§6.6.2.3.3 adjudications.

### 4. The corpus reports; it does not gate

ADR 0922 has the rule. `tests/corpus.rs` compares this crate against the veraPDF corpus clause by
clause and prints five columns, of which `over` — a document its author built to conform that this
crate failed — is the one that matters. It is **zero on all six targets**, and the six
disagreements standing behind that number are adjudicated entries with the clause reasoning
written out, not tolerances.

`elsewhere` was added late and is worth its own sentence: several clauses state their requirement
by *delegating* to another, so a witness filed under §6.2.2 is correctly failed under §6.2.6, and
counting that as a miss was a defect in the harness rather than in the crate. Adding a duplicate
row under the referring clause would make a report state one fault twice.

## What measurement changed, three times

`CLAUDE.md` principle 2 asks that an optimisation be justified by a benchmark. Three beliefs about
this crate's cost were wrong, which is why `examples/cost.rs` exists:

| believed | measured |
|---|---|
| the repeated content survey | real, 19 s → 10.5 s |
| the repeated object walk | almost nothing; that walk is 346 ms once |
| the page-tree enumeration | 153 ms → 146 ms; the cost is `/Parent` inheritance |
| — | fifteen rules re-walking `/Annots`, 8.8 s of a 10 s report |

`Examination` is the answer: the document, the target, and shared work computed at most once and
only where something asks. It also collapsed the check signatures into one, which retired a
`PerTarget` variant added a day earlier — the rules that need the target are not rare enough to be
an exception.

## Consequences, including the uncomfortable one

- Genuine misses across the six targets fell from 688 to 64 in one session, with false positives
  at zero throughout.
- **Three `cmap`-inventory rules are exercised by no corpus document at all.** They rest on a unit
  test of the accessor and on reading the clause. That is worth saying plainly: the corpus ranks
  what documents contain, and a requirement nothing exercises is a requirement nothing ranks —
  `CLAUDE.md`'s two denominators, seen from the coverage side.
- The rules this crate cannot check are its most honest output. Each names what is missing: a
  reader `pdf-model` does not expose, a base standard this tree does not carry (ISO 32000-1:2008
  for part 2 — `doc/questions/Q49`), a text nobody here has read.
