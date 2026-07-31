# ADR 0079 — Which of five values applies

Status: accepted, 2026-07-31.

## Context

ADR 0072 read §14.7.6's attribute objects: which are attached to an element, and which wins
between `/A` and `/C`. §14.8.5 is the same mechanism described by the *other* clause, and it adds
two things §14.7.6 does not state.

**Owners are not labels.** §14.8.5.2: the owner "determines the interpretation of the attributes
defined in the object", so `/BBox` under `Layout` and `/BBox` under `HTML-4.01` are different
attributes that share a name.

**There are five priorities, not two.** §14.8.5.3 lists them, and the first is conditional in a
way that decides what this program does:

> The value of the attribute specified in the element's A entry, owned by an owner as specified
> by the O entry … excluding Layout, PrintField, Table, List and Artifact, if present, and if
> processing based on the format indicated by the owner value

## Decision

**Implement the first four priorities; leave the fifth and the choice at the fourth to whoever
knows the attribute.**

`structure::Owner` is Table 376, with the thirteen format-specific owners as one arm: this reader
translates to XML, HTML, CSS or RDFa for nobody, and what it needs from such a name is only that
it is *not* one of the five PDF-native ones. `Tree::attribute` therefore **skips** a
format-specific owner rather than preferring it — which is the clause's own condition, not a
simplification — and applies priorities 2 and 3 in the order `Tree::attributes` already returns.

`Tree::inherited_attribute` is priority 4, walking `/P` under the same bound §14.9.2.3's language
inheritance uses. It is a *separate function* because the clause makes inheritability a property
of the attribute rather than of the element — §14.8.5.4's tables say which attributes inherit —
and a reader that inherited everything would give a paragraph its table's `/ColSpan`. The two
functions return the same kind of answer because the clause says they are the same kind: "[t]here
is no semantic distinction between attributes that are specified explicitly and ones that are
inherited."

Priority 5 is a default, which is the same sort of per-attribute statement.

## Twelve rows this family does not owe

§14.8.5's sixteen rows were reviewed long before this session and twelve are `inapplicable` on an
argument worth restating, because it is the reason the *mechanism* is worth implementing while
the *attributes* are not: a layout attribute describes the layout process that produced an
appearance, and this program has the appearance. `/SpaceBefore` does not move anything; it says
how far something was moved by whoever made the file. What consumes those is a processor
reflowing the content, which is a different program.

What that leaves in clause 14 is §14.8.5.6's `PrintField` — `/Role`, `/checked`, `/Desc` on a
`Form` element, describing a *non-interactive* form field's appearance — and it stays `silent`
here with its row saying why it is the one attribute family that is rendering-adjacent.

## And one more understated row

§14.8.5.1 was `silent` while §14.7.6.1 and §14.7.6.2 recorded the same code as `implemented`. Two
clauses describe one mechanism from two directions, and reviewing one family left the other's row
untouched — the fourth instance in this run of ten sessions of a row claiming less than the code
does, after §14.7's six, §8.4.5's one and §14.7.5's four. The pattern is now specific enough to
state as a rule: **when two clauses describe one mechanism, reviewing one of them leaves the
other lying.**

## Consequences

- `silent` falls 97 → 93. Clause 14 has **7** rows left: §14.8.5's aggregate and `PrintField`,
  §14.8.2.5's three about content ordering, §14.8.2.6.2's word breaks, and §14.7.7's example.
- No gate moves, and none could: §14.1's "do not affect the final appearance of a document" holds
  for the whole of this family.
