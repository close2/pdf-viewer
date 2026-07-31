# ADR 0078 — The vocabulary a tag is written in

Status: accepted, 2026-07-31.

## Context

Four sessions have read §14.7's structure tree — its children, its attributes, its namespaces —
and one thing was missing from all of it: what a tag *means*. `Tree::role` answered a string. A
consumer asking "is this a heading" got `"H2"` and had to know what that was.

§14.8.4 is the answer, in twenty-one rows and eleven tables: forty-one standard structure types
in four categories, plus the rules for the types that are in more than one. §14.8.4.1 makes the
vocabulary closed for a tagged document — "[a]ll structure elements occurring within a tagged PDF
document shall have a type matching one of those defined as a Standard Structure Type, or a role
map providing a mapping from the non-standard type to a Standard Structure Type" — which is
exactly why `Tree::role`'s mapping exists and what its answer is *for*.

## Decision

**Transcribe the vocabulary and put the clause's own rules on it.** `structure::StandardType` is
the forty-one types, `Tree::standard_role` is `role` read through it, and three things about it
are the clause rather than a design:

- **`Hn` is a family, not a name.** Table 372 writes it as `H n`, "with n being a sequence of
  digits representing an unsigned integer greater than or equal to 1", so it is matched before
  the table: `H17` is as standard as `H1`, and `H0` is not.
- **A type does not always have one category.** `Figure`, `Formula`, `Link`, `Annot`, `Form`,
  `Title`, `FENote`, `Caption`, `L` and `Artifact` are two or three, and §14.8.4.1 settles it by
  context: "[i]f the structure element is used inside a block level element, it is an inline
  level structure element … In all other cases it is a block level structure element."
  `Category::of` is that sentence, taking the parent's category from a caller — because this
  crate walks `/K` on demand and keeps no ancestry, which is ADR 0072's decision and still right.
- **A fifth answer the clause gives without calling it a level.** `LI` is "[i]nternal to L (List)
  structure elements" and `TR` "[i]nternal to a Table structure": a statement about where an
  element may appear rather than about what it may contain. `Category::Internal` keeps the two
  kinds of statement apart.

`StandardType::since_pdf_2_0` marks the eight types PDF 2.0 added, because §14.8.6.1's *default*
namespace is PDF 1.7's and a document in it cannot mean `Em` by `Em`.

§14.8.1's mark information dictionary comes with it, and the distinction it draws is worth the
row: a document may have a structure tree without claiming to follow §14.8's rules, and `/Marked`
is the claim — "[a] tagged PDF document shall contain a mark information dictionary … with a
value of true for the `Marked` entry".

## What is deliberately left

**Annex L's nesting rules.** §14.8.4.2 defers them to an annex, and a nesting rule is a statement
about whether a *document* is well formed. Nothing here validates documents — the same position
§7.11.2.1's path rules and §7.12.4's version ordering are already recorded in — and a reader that
rejected a badly nested tree would be refusing to speak a page over a producer's mistake.

**§14.8.5's attributes**, which are what a `/BBox`, a `/ColSpan` or a `/Scope` says. They are the
last thing in clause 14 that is data rather than a consumer, and they are the rows this session
did not take.

## Consequences

- `silent` falls 119 → 97, **under a hundred for the first time**. Clause 14 has **11** silences left:
  §14.8.5's four attribute rows and its aggregate, §14.8.2.5's three about ordering, §14.8.2.6
  and §14.8.2.6.2's word breaks, and §14.7.7's worked example.
- Nothing renders differently and no gate moves; `Tree::standard_role` has no consumer yet, which
  is the debt this whole family carries and the one thing that would make all of it visible.
