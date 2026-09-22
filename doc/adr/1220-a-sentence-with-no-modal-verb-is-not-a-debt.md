# ADR 1220 — A sentence with no modal verb is not a debt, and neither is a validator's job

## Status

Accepted, 2026-09-22. Session 1191. Settles §9.8.3.3, §12.11.3 and §12.11, each of which had been
`partial` on a residue that turned out not to be addressed to a reader.

`§N` is ISO 32000-2 and nothing else.

## Context

`doc/todo/65`'s fifth bucket collects rows whose stated blocker is *nothing owed* — the residue is
real but no reader is the audience for it. Two of its entries had been sitting there long enough
that the sentence in the row was being read instead of the clause.

**§9.8.3.3**, the `/FD` glyph classes, recorded "the reader's half is done; enforcement is a
validator's job the reader does not own". That is a true sentence about the tree and a strange one
about a ledger: a row is `partial` when the clause asks for something this program does not do, and
"a validator's job" is by construction not that.

**§12.11.3**, the requirement penalty values, recorded that the clause's middle-of-the-range
sentence "waits on a second document rather than on a reading", and §12.11's aggregate row was
`partial` for the same reason and nothing else.

## Decision

**§9.8.3.3 is `implemented`.** The clause addresses exactly one act to a processor: "[t]he entry's
value shall be a font descriptor whose contents shall override the font-wide attributes for that
class only." `substitute::overridden` performs the overriding — dropping the seven entries the
clause (as Errata Collection 3's Issue #5 repairs it) says such a descriptor shall not hold, which
is a reader declining to let a class descriptor answer a question it is not there to answer — and
`glyph_class::Decision` is what makes *for that class only* decidable against Table 123's own
descriptions, refusing by name where no character can settle it. Every other sentence binds the
dictionary a producer writes: the defaults the main descriptor shall define, what each key shall be,
the entries the class descriptor shall not include, the `should` about proportional Latin metrics.
Not one asks a processor to reject or report such a file. There is no unmet `shall` here, so there
was no debt.

**§12.11.3 is `implemented`, and §12.11 with it.** The residue is one sentence, and reading it for
its verb is the whole argument: "Values between 0 and 100 are available to weight the value of this
feature among other features in the same document requirements array as well as when contributing
to the total penalty points to weigh against other documents in the choosing process if alternatives
are available." *Are available to weight* states what the values are for; it is not a `shall`, not a
`should`, and not addressed to anybody in particular. Even read at its strongest, the half that
looks outward carries the clause's own condition — "if alternatives are available" — which a program
opening one document never meets. Everything else §12.11.3 states is performed: the range and its
clamp, the two ends' meanings, the total over the requirements that cannot be met, and the threshold
that total is compared against. §12.11's aggregate was `partial` for this and nothing else, and every
subclause under it is now `implemented` or `out-of-scope`.

## Consequences

- Three rows move to `implemented` and `doc/todo/65`'s fifth bucket loses two entries.
- §9.8.3.3's EXAMPLE 2 — two glyph classes rather than one — is now a fixture,
  `glyph_class.rs::the_clauses_own_example_states_two_classes_and_both_are_applied`. It is worth
  having beside the refusals: the clause's own example is `Proportional` with `HKana`, which Table
  123 describes by disjoint characters, and it is the case `Proportional` with `HRoman` had to be
  separated from.
- `requirements::Requirement::penalty` carries the reading of the middle-range sentence beside the
  code, so the next reader of that field finds the disposition rather than the row.
- **The general shape, and it is what this ADR is for**: a residue recorded in a row is a claim
  about the clause, and the two ways it decays are that the sentence has no modal verb at all, or
  that its modal verb binds somebody who is not a reader. Neither is visible from the row. Both are
  one minute's reading of the clause, which is the reading `doc/todo/65`'s fifth bucket exists to
  prompt and is not the reading its entries had been getting.
