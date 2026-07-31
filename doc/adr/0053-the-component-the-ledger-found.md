# ADR 0053 — The component the ledger found

Status: accepted, 2026-07-31.

## Context

Reading clause 12's interactive half and §14.7 produced 131 `silent` rows in four sessions, and
the point of writing a status per row rather than per clause is that the rows can then be read
*across* clauses. Four of them named the same absent thing:

- §12.3.2.4, named destinations — a `/Dests` **name tree**.
- §12.4.2, page labels — a `/PageLabels` **number tree**.
- §12.7.7, named pages — a `/Pages` **name tree**.
- §14.7.5.4, finding a structure element from a content item — a `/ParentTree` **number tree**.

Four families, in two clauses, blocked on one small piece of clause 7. **No single clause review
would have shown that, and no corpus document would ever have asked for it** — a demand curve
ranks features, and this is not a feature.

## Decision

**`pdf-syntax::tree` implements §7.9.6 and §7.9.7 as one module**, because the two clauses
differ in exactly one thing: §7.9.7 defines itself as "similar to a name tree … except that its
keys shall be integers instead of strings", with `/Nums` in place of `/Names`. `TreeKey` is that
difference and nothing else.

Two entry points, for two shapes of question:

- `lookup` **descends**, which is the clause's own reason for the structure existing: the pairs
  "can be looked up efficiently without requiring the entire data structure to be read from the
  PDF file" — principle 2 written by ISO.
- `number_pairs` walks the whole tree, which §12.4.2 needs because a labelling range runs from
  its own key to the *next* one, and no single lookup produces a neighbour. A document has as
  many ranges as it has numbering styles, so this is the one place the clause's efficiency
  argument does not apply, and saying which is which is the reason there are two functions
  rather than one.

**`/Limits` are read as a hint rather than as a gate.** The clause requires them on every
intermediate and leaf node and requires the keys sorted; real files get both wrong, and a reader
that trusted them would lose entries that are present. A node whose limits exclude the key is
skipped; a node with *no* limits is searched. That costs a malformed file a wider walk and costs
a well-formed one nothing.

## And then the first thing built on it: §12.4.2's page labels

Chosen because it is the only row in clause 12's navigation half **with no user-interface
question in it** — a label is a string computed from the document and from nothing else — and
because `CLAUDE.md` names it in scope beside outlines and destinations.

Four things the clause states that a first implementation gets wrong, and each is in the code
with its sentence:

1. **There is no default numbering style.** "[I]f no S entry is present, page labels shall
   consist solely of a label prefix with no numeric portion" — so a range with a `/P` and no
   `/S` gives *every* one of its pages the same label, which the clause's own NOTE spells out
   with `Contents`.
2. **`/A` is not base 26.** "A to Z for the first 26 pages, AA to ZZ for the next 26" — the
   twenty-eighth page is `BB`, where base 26 would say `AB`. The letter is
   `(n − 1) mod 26` and the *repeat count* is `⌈n ÷ 26⌉`.
3. **The Roman form is subtractive**, which the clause fixes not by stating an algorithm but by
   its example running `i, ii, iii, iv` — the additive form spells the fourth `iiii`.
4. **`/St` "shall be greater than or equal to 1"**, so a file writing zero gets the stated
   default of 1 rather than a page labelled `0`.

**The clause's worked example is the test.** Three ranges — lowercase Roman from page 0, decimal
from page 4, decimal with prefix `A-` and `/St 8` from page 7 — and the nine labels the standard
prints beside them: `i ii iii iv 1 2 3 A-8 A-9`. No corpus document exercises all three forms,
which is trap 8, and the example is the one place the standard states *answers* rather than
rules.

The corpus test is the other half, trap 4's: **22 of the 974 documents state page labels** and
every one of them labels its first page, which is what §12.4.2 requires of the tree ("[t]he tree
shall include a value for page index 0").

`viewer-ui` shows the label **beside** the index rather than instead of it, because a title
reading `iv` cannot also say `of 320`. That is a viewer's choice and it is written where it is
made.

## Consequences

- §7.9.6, §7.9.7 and §12.4.2 go from `reported`, `reported` and `silent` to `implemented`.
- Three `silent` rows now name a component that *exists*: §12.3.2.4, §12.7.7 and §14.7.5.4 need
  the tree and the semantics on top of it, and the tree is no longer the missing part.
- The general lesson is about the instrument rather than the feature. **A ledger with a status
  per subclause can find a missing component, not only a missing feature** — but only if the
  rows are written well enough to be read across clauses, which is an argument for the prose the
  notes carry rather than against it.
