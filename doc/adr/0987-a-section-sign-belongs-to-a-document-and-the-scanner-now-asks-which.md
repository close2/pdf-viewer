# 0987 — A section sign belongs to a document, and the scanner now asks which

Session 977. Status: **accepted**. It teaches `conformance::citation` the three things a `§` can
be in this tree, and reclassifies the 142 that were being checked against the wrong document.

## 1. The defect, measured

`doc/todo/56` stated, until this round, the rule the whole project is written around:

> `§` in this tree means *a clause of ISO 32000-2* and nothing else — that is what makes every one
> of them checkable

ADR 0997 section 4 has what it says now, and why the narrowing was owed.

ADR 0984 section 6 reported that the gate does not enforce it, and priced it at "roughly a hundred
and thirty". The number off a run of the changed scanner, over `crates/`, `tools/` and `fuzz/`:

```
142 `§` naming a section of one of this project's own documents rather than a clause,
142 of which resolve against ISO 32000-2 and were counted as citations of it
```

Every one of the 142 resolved. That is the whole of the finding: not one of them failed
`every_citation_names_a_clause_that_exists`, because a section number of ours lands on a clause
number of theirs. The ledger's notes carry twelve more.

**Twenty-two of the 142 landed inside the ledger's own population** — `doc/todo/03` §9,
`doc/todo/03` §8, `doc/todo/03` §11, `doc/todo/02` §7, `doc/todo/02` §6, `doc/todo/11` §6 and
`doc/todo/58` §4 — so the coverage instrument was being fed citations of clauses 6, 7, 8, 9 and 11
that nobody had made. `ledger::check` walks `citations()` to decide which clauses the code cites;
a `partial` row for clause 9 could have been kept honest by a comment about a to-do list's ninth
section.

**A before and an after belong to one tree state or to neither**, which is why the number above is
the finding rather than a subtraction: four sibling rounds were writing citations into this working
tree throughout the session, and the gate's total moved between the round's first run and its last
for reasons that have nothing to do with this change. On the run that produced this ADR the gate
printed 15 349 citations; the same sources under the old reader would have printed 15 491, because
every one of the 142 resolved.

## 2. What the scanner could not see, and why each rule is the shape it is

The old `another_document` recognised one thing: an acronym and a plain number, or an upper-case
`FILE.md`, in the **word immediately before** the sign. Four families defeated it.

### A document wrapped in backticks is not a document further away

This tree writes `` `doc/todo/02` §2 ``, and the word in front of the sign was therefore
`` `doc/todo/02` `` — which is not a path, not a `.md` name and not an acronym. Ninety-six of the
142 name their document in front of the sign, and all but a handful wrap it in backticks. So a wrapper is removed before the word is read — a backtick, a quotation mark
or an emphasis marker at either end, and an *opening* bracket — and the same repair fixes
`` `RFC 3986` §5.2 `` on the findings side, which was equally invisible.

### But punctuation between the name and the sign is the sentence moving on

This is the half that makes the rule safe, and it is why the wrapper set is written out rather
than "trim punctuation":

- **A trailing comma, full stop or closing bracket.** `(ADR 0044), §12.5.6.5` is a citation of the
  standard with an ADR mentioned in front of it. This tree ends a parenthetical in front of a `§`
  constantly — `(ADR 0253 and ADR 0728.) §14.7.5.2`, `(ADR 0220). And whether §8.6.5.9` — and
  trimming punctuation blindly would have taken several hundred genuine clause citations out of
  the checked population *and left the gate green*, which is the same defect as the one being
  fixed, pointing the other way.
- **A possessive, which the tree proves twice.** `` `doc/conformance/ledger.toml`'s §8.7.4.1 row ``
  and `` `examples/absence_audit`'s §10.7.5 block `` both name a file that *discusses* a clause.
  `X §N` is a section of X; `X's §N` is X's treatment of the standard's §N. Both spellings are in
  the tree, ten lines apart in places, and only the apostrophe separates them.

### A letter-suffixed number is never a clause of ISO 32000-2

`§3a`, `§5a`, `§2b`, `§3d` — 46 of the 142, and no document is named anywhere near most of them.
The evidence is the standard itself: of its **1 061 numbered headings, zero** carry a letter after
the digits, in the body or in an annex (`grep -cP '^#{1,6}\s+[0-9]+(\.[0-9]+)*[a-z]'` over
`doc/md/`). An annex's *opening* letter is the only letter any clause number has; where an erratum
inserts a subclause it renumbers its neighbours rather than lettering it. **A table is different**
— `Table 125a` is a real caption — which is exactly why this rule is stated for the `§` side only.

So `§5a` is provably not clause 5. What it *is*, this reader does not say: `ProjectSection`'s
`document` is `None`, and the gate prints the count. It is `doc/PLAN.md` §5a, and a rule that
worked that out — by taking the last document named in the same paragraph, say — would be guessing
on a population where the nearest name is often a hundred lines above (`oracle.rs` writes
`§3a` forty-five times for one section of `doc/oracle-and-corpus.md`). `CLAUDE.md`'s own standard
applies: a rule that guesses is worse than a rule that declines.

### A file name is a file name in any case

The `.md` arm required an **upper-case** stem, which is a test of how the author spelled the name
rather than of what it is. The three-hundred-and-ninety-first session found half of that (`doc/` is
not upper case, so a path defeated it); the other half was still live, and ten citations of
`doc/oracle-and-corpus.md`'s sections were being checked against ISO 32000-2 because that document
is not shouted.

## 3. The decision: classify, do not report

A `§` now resolves to one of three things, and the third is new:

| | what it is | what the gate does |
|---|---|---|
| nothing in front of it | a clause of ISO 32000-2 | checks it against the clause index |
| another **standard** in front of it | `ForeignCitation` | **fails**, and teaches "RFC 3986 section N" |
| one of **our own documents** in front of it, or a number no clause can have | `ProjectSection` | counts it, prints it per document, checks nothing |

**The third row is a classification rather than a finding, and that is the load-bearing choice of
this round.** The alternative was to make `` `doc/todo/02` §2 `` a gate failure teaching the
spelling "doc/todo/02 section 2", which is what the `FILE.md` arm has done since the
three-hundred-and-ninety-first session. It was rejected on the evidence:

- **The population is the tree's own convention, not a mistake.** 142 sites in Rust, 12 in the
  ledger, and over a thousand in `doc/adr/` and `doc/history/` — where `doc/history/README.md` says
  a file is "written once, never rewritten" and an ADR is corrected only for pointer accuracy. A
  rule that would require rewriting the project's records is a rule about the wrong thing.
- **`doc/habits.md` cites itself that way**, and so does this sentence of it: "which is ADR 0232
  §2's rule". The instruction documents are where the convention is *taught*.
- **The harm was never the spelling.** It was that the number got resolved in the wrong document
  in silence. Classifying ends that completely: a `ProjectSection` is checked against nothing,
  counted, and printed beside the document it belongs to.

What a finding still is: another **standard's** section. `RFC 3986 §5.2` stays a build failure,
because there the spelling is the only thing standing between a reader and a clause of ISO 32000-2
that says something else entirely.

### Which documents are ours

A path under `doc/` whose leaf carries no extension (`doc/todo/58`, `doc/adr/0984`); any `.md` file
name, with or without a path, in any case; `ADR NNNN`; and an RFC numbered with a **leading zero**,
which is how this project numbers its own (`doc/rfc/0002`) and how the IETF does not (`RFC 3986`).
A file under `doc/` carrying a different extension is *not* a document with sections, and the tree
supplies the counter-example: `doc/conformance/ledger.toml`.

## 4. Calibration, per trap 13

Both halves are held by tests in `citation.rs`, and the negative half is the one that matters:

- **It fires** on `` `doc/todo/02` §2 `` and `` `doc/todo/03` §9 `` (classified, with the document
  named), on `RENDER_LIBRARY.md §4.5`, on `doc/QUORRA_FEEDBACK.md §12`, on `§3a` (classified, with
  no document), and — unchanged — on `RFC 3986 §5.2.2` and `` `RFC 3986` §5.2.2 `` as findings.
- **It does not fire** on `(ADR 0044), §12.5.6.5`, on `` `doc/conformance/ledger.toml`'s §8.7.4.1 ``,
  on `` `examples/absence_audit`'s §10.7.5 ``, on `ADR 0253 and ADR 0728.) §14.7.5.2`, or on
  `doc/conformance/ledger.toml §8.7.4.1` — five spellings that keep their clause citation, four of
  them copied out of the tree.

The whole-tree run is the second calibration and the stronger one: 15 349 citations still checked,
142 classified, and the gate green over a population of eleven documents whose names the report
prints.

## 5. A ratchet on the one shape nothing can check

`UNNAMED_SECTION_CEILING` starts at **46** and may only fall. A `§3a` with no document on its line
is the only member of this population that no instrument can resolve, and the fix is one edit by
whoever writes the comment: name the document in front of the sign, the way the ninety-six
citations beside them already do. It is a ceiling rather than zero because all 46 sites are in
crates this round did not own.

## 6. What this does not do

- **It does not touch a single site.** The scanner changed; the tree's prose did not. That was the
  round's instruction and it is also the right shape: the writing was correct and the reader was
  wrong about it.
- **It does not catch a standard cited with its year.** `ISO 32000-1:2008 §7.3.4.3` is read as a
  citation of *this* standard's §7.3.4.3 today, because the colon defeats the number test. Three
  sites, all in `crates/pdf-archive`, measured and left: ADR 0997 has the reading and the reason.
- **It does not read `doc/`**. The gate scans Rust sources and the ledger's notes; the thousand
  `§` in the ADRs and the history files are outside every instrument here, as they were.
