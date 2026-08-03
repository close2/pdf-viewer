# An AccessKit bridge

Status: the answer exists and nothing asks.
Priority: 31 — the fifth of §0's five owed features, and the only one still open
Clauses: §14.7, §14.9
Code: `crates/viewer-core/src/lib.rs` (`Query::AccessibilityTree`), a new host

`Query::AccessibilityTree` has answered since the hundred-and-forty-ninth session with §14.7's
elements, §14.9's spoken form of each (`/Alt`, `/ActualText`, `/E`, `/Lang`) and the
quadrilaterals they cover (ADR 0134). **Nothing consumes it.** That is the whole item: it is a
host, not a vocabulary.

`AccessKit` with AT-SPI on Linux is the stack `CLAUDE.md` names. The mapping is mostly
mechanical — §14.8.4's forty-one standard structure types onto AccessKit roles — with two places
where a decision has to be made and written down:

- **reading order**. A selection is taken in *content* order, and §14.8.2.5's logical order is
  what a screen reader wants. `Interpretation::marked` carries the `/MCID` spans and
  `Tree::logical_text` produces the logical string; what is missing is the map between the two
  orders' offsets — the same missing map as `33-annotation-editing.md`'s last item.
- **what to say about what this program refused.** A page with an unreported gap is one thing; a
  page whose text is not drawn at all (todo 21) is another, and a reader that says nothing about
  it is lying by omission.
