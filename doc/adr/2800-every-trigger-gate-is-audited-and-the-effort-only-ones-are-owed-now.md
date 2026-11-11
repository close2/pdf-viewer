# 2800 — Every trigger gate is audited, and the effort-only ones are owed now

Session 1994, a ledger round. Status: accepted — the audit is `doc/todo/76`, its first gate is built
(ADR 2799). Answers `doc/questions/A800` item 2.

## Context

The owner, 2026-11-06: "build now and revisit all 'wait for a trigger' gates". A gate here is any
text that holds work back until something outside the work happens — a document arrives, a host
asks, a person acts, a crate is released, a standard is bought. No round had collected them.

## Decision

1. **The population is a grep, read hit by hit** (`doc/todo/76`'s header has it), and the table is
   one line per gate: where, what it holds back, what row or feature, and its kind.
2. **Seven kinds, and A800 lifts two of them.** *Effort only* and *another lane's* are owed now.
   *Owner-blocked* (a purchase, a certificate, a launcher the owner has not chosen to ship) and
   *scope exclusion* (`CLAUDE.md`'s closed list, revisited "by argument, never by attrition") need
   the owner's word, and `doc/questions/Q828` asks for it. *Outside release* cannot be built by
   anyone here. *Measured and declined* is principle 2's — "an optimization must be justified by a
   benchmark" — and a lever whose benchmark said no is not a trigger: building it now would be the
   speculative optimisation the same principle forbids. *Already built* is text that outlived work.
3. **Living text is lifted where it stands; records are not rewritten.** `doc/todo/01`, `doc/todo/63`,
   `A45`'s `Owes:` and the Readings of `A66` and `A168` say what A800 did; ADRs 1485, 1672, 1736,
   2137 and 2529 keep their words (ADR 1023) and are cited from `doc/todo/76`.
4. **The ledger rows of the other lanes are theirs.** §7.6.5's notes still name A66's trigger;
   the round that builds the handler rewrites them with the status they move to.

## Consequences

The order of the remainder is `doc/todo/76` section 3. A gate written after this one carries its kind
in its own text, so that the next audit is a read rather than a census.
