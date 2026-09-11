# 0997 — Two more things a section sign is, and the one this round declined

Session 977. Status: **accepted**. The second half of ADR 0987's reading: one more shape the
scanner now recognises, one it still cannot, and the residue no rule can settle.

## 1. A code span holding nothing but the sign is the character being named

Found by running the changed gate rather than by reasoning about it. Four `pdf-archive` module
headers, written by a sibling round in the same working tree, say things like

> Every `§` here is an ISO 32000-2 number, and this walk serves rows that bind a **PDF/A-2**
> target, whose base standard is ISO 32000-1:2008.

and every one of them was reported as `` `§ here is an ISO 32000-2` is not a citation this checker
can read``. The sentence is correct writing — a file whose clause numbers resolve differently in
two editions of the standard *has* to say which edition it means — and the checker was failing the
build over it.

Why it had never happened: the one directory the scan does not read is `tools/conformance` itself,
and until this round it was the only place in the tree that wrote prose *about* the convention.
The moment the convention gets explained somewhere else, the checker reports the explanation.

**The rule**: a `§` with a backtick on each side of it is the character, not a citation, and is
skipped. It is as narrow as it can be made — `` `§` `` and nothing else; a bare `§` with no number
after it is still a `MalformedCitation`, which the test holds on the same line as the exemption.

## 2. A standard cited with its year still passes as ours, and three sites do it

`another_document` requires the number after an acronym to be digits and hyphens, so `ISO 15076-1`
is recognised and **`ISO 32000-1:2008` is not**: the colon fails the test, the word is not a
document, and the `§` after it is checked against ISO 32000-2's clauses. `doc/todo/56` measured
this shape in the eight-hundreds and called the year "the spelling a round will reach for"; it is
now in the tree three times, all in `crates/pdf-archive`:

| site | written | reads as |
|---|---|---|
| `src/table/file_structure.rs:374` | `ISO 32000-1:2008 §7.3.4.3` | ISO 32000-2 §7.3.4.3 |
| `src/table/file_structure.rs:661` | `ISO 32000-1:2008 §F.3.4` | ISO 32000-2 §F.3.4 |
| `src/table/file_structure.rs:2088` | `ISO 32000-1:2008 §7.3.4.3` | ISO 32000-2 §7.3.4.3 |

All three resolve, so the gate is silent, and the third is the dangerous kind: the comment's own
point is that the *two editions* say the same thing there, which is a claim about the older one
that nothing checks.

**Not fixed here, and the reason is ownership rather than difficulty.** The scanner change is one
character class; what it costs is that three lines in a crate owned by a concurrent round have to
be rewritten in the same commit, or the build goes red for its author. The rule to write, when a
round owns both: allow `:` and a year in the number, and exclude `ISO 32000-2` *and*
`ISO 32000-2:2020` from the foreign side, since naming this standard with its year is not a
finding.

## 3. The residue: what no rule here can settle

Three shapes remain uncheckable, and they are written down so that the next round does not
rediscover them as new:

- **A section of ours with no document on the line and no letter in its number.** `§2's Bomb B` in
  `pdf-syntax/src/filter.rs` is `doc/todo/10` §2, named as such twelve lines away in
  `document.rs` and nowhere near the sign here. It is indistinguishable from a citation of clause
  2, *Normative references* — which the tree also makes, correctly, three times in
  `pdf-model/src/structure.rs` about MathML. Nothing can tell those apart, and ADR 0987's ratchet
  covers only the half that a letter suffix betrays.
- **Another standard named in prose rather than by acronym and number.** `doc/todo/56`'s example is
  "the JavaScript for Acrobat API Reference §12.5"; the live one in this tree is
  `pdf-archive/src/target.rs`'s "PDF/A-2 at one of §5's three levels", which is ISO 19005-2's
  clause 5, *Conformance levels*, and reads today as ISO 32000-2's clause 5, *Version
  designations*. A checker that guessed at every noun phrase before a `§` would report the
  sentences that merely mention another document, and this one has to be right every time.
- **The thousand `§` in `doc/adr/` and `doc/history/`**, which no instrument here reads at all.
  They are records, and ADR 0987 section 3 says why they stay as written.

## 4. What is true of the rule now, and what is not

`doc/todo/56` said, until this round, that "`§` in this tree means *a clause of ISO 32000-2* and
nothing else". That was never true of the *tree*, which has always written `§` for its own
documents' sections. What is true after ADR 0987 is a claim about what the **gate checks**, and
that is what the paragraph now says:

> A `§` this tree does not put a document in front of is a clause of ISO 32000-2, and
> `cargo test -p conformance` checks every one of them against the standard. A `§` after another
> standard's name fails the gate. A `§` after one of this project's own documents is that
> document's section, counted and printed rather than checked — except where its number could
> still be a clause and no document is named, which nothing can see.

`doc/todo/56`'s paragraph now says that, and it is the one live instruction document that stated
the rule. **`CLAUDE.md` does not state it at all, in any wording**, which is worth recording because
this round was sent to narrow the sentence there and there is no such sentence: the claim lives in
`doc/todo/56`, in ADR 0984 section 6 and in `doc/history/814`, which is a record and stays as
written. Nothing in `CLAUDE.md` was edited, because nothing in it was wrong.
