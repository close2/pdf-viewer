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
mechanical — §14.8.4's forty-one standard structure types onto AccessKit roles — with **one** place
left where a decision has to be made and written down, and one dependency question beside it:

- **`accesskit_unix` needs an async runtime**, through `zbus`, and `CLAUDE.md`'s startup rules say
  "[n]o async runtime unless something genuinely requires one". This one does, and it is confined
  to a *host* crate rather than to anything on the render path — but it is a decision to take in
  writing before the first line, the same shape ADR 0186 took for an XML parser.

- ~~**reading order**~~ — **closed in the two-hundred-and-ninety-sixth session.** The map between
  the two orders' offsets is `Tree::logical_range`, and `AccessibilityNode`'s list has been in
  §14.8.2.5's logical order since the hundred-and-forty-ninth anyway (`accessibility.rs`'s own
  "The order the nodes are in"). Both halves of this bullet were stale when it was written.
- **what to say about what this program refused.** A page with an unreported gap is one thing; a
  page whose text is not drawn at all (todo 21) is another, and a reader that says nothing about
  it is lying by omission.
