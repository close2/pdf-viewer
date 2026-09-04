# 0897 — The `shall` a namespace row never named, and the cousin that was already carrying it

Session 928. Status: **accepted**. The second finding of the same band of `partial` rows ADR 0896
records; this one kept its status and changed what the status is *for*.

## Context

§14.7.4.2's row has been `partial` since it was written, for one stated reason: Table 356's
`/Schema` is stored and never fetched. The note enumerated the table's three entries — `/NS`,
`/Schema`, `/RoleMapNS` — said which two are used, and closed with a read-and-kept sentence from
session 525 confirming that `structure.rs` stores `/Schema` and nothing fetches it.

Both halves of that reason are wrong in the way `doc/habits.md` warns a reading can be: the note
is true about the tree, and it is answering a question the clause does not ask.

## Decision

### `/Schema` is a permission, so it is not what a `partial` can rest on

The clause states it as "An optional Schema entry may be provided to identify the schema file for
the namespace", and NOTE 2 declines to say what format one would be in. That is ADR 0896's shape,
and on its own it would have moved this row to `implemented` with the other three.

### The clause's third `shall` is in its closing paragraph, and the row never named it

> When the owner of an attribute object … is specified by an NS entry, the namespace name shall be
> considered as identifying the owner.

and, for namespace names corresponding to Table 376's owner values, "they shall be considered
equivalent". A reading that stops at the table misses it, because it is stated in prose after the
table and its subject is an object defined three clauses away.

**It has a consumer.** `Tree::attribute` filters the attribute objects it will consult:

```rust
.filter(|object| object.kind.is_pdf_native() || object.kind == Owner::Namespace)
```

so an object whose `/O` is `NSO` is consulted, while the format-specific owner it may be equivalent
to is skipped — §14.8.5.3's priority 1 applies "if processing based on the format indicated by the
owner value" and this program processes to no format. Whether that is right turns on exactly the
equivalence §14.7.4.2 states, and on which of §14.8.5.3's priorities an `NSO` object falls under.

### The question was already open one row over, and the two rows did not know about each other

§14.8.5.3's row has carried it since session 811 (ADR 0743), in detail, down to naming
`Namespace::is_standard` as the predicate either answer needs — and it never cites §14.7.4.2, which
is where the sentence that maps a namespace name onto an owner actually lives. §14.7.4.2, for its
part, was `partial` for a file specification nobody has to fetch.

That is `doc/ledger-and-claims.md`'s **seventh shape** — two rows about one mechanism — arriving in
a form the shape's own description does not cover. The description says the tell is two rows
*disagreeing*, one giving a capability reason where the other names code. Here they do not
disagree: one row states the mechanism and the other is silent about it, and silence cannot
contradict anything. The `inapplicable` sweep prints a cousin beside a row; nothing prints a row's
*missing* cousin.

So §14.7.4.2 stays `partial`, for the equivalence sentence rather than for `/Schema`, and both rows
now name each other so that neither can be settled alone.

## Consequences

- **A row's enumeration of a table is not a reading of its clause.** Two of this clause's four
  reader-facing sentences are in the table; the third is prose above it (the `/NS` name "should
  take the form of a uniform resource identifier"), and the fourth is prose below it and is the one
  that reaches code. A note that opens "Table 356's namespace dictionary:" has already chosen to
  read the table.
- **The pair is now one question with two rows on it**, and settling it means deciding both
  membership — which namespace names correspond to a Table 376 owner, which the standard does not
  state and which is therefore a documented choice under `CLAUDE.md` principle 5 — and rank, since
  priority 1 sits above priority 2 where its condition holds. That is more than this round had, and
  it is written down as owed rather than guessed.
- The instrument this suggests is narrow enough to be worth naming: **for each `partial` row, the
  clause's `shall` sentences that are not inside a table**. `--bin entries` reads the entries a
  clause states; nothing reads the sentences between them.
