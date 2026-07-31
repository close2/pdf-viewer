# ADR 0074 — What a document says it needs

Status: accepted, 2026-07-31.

## Context

Two clauses ask the same question from opposite ends of the standard, and both were `silent`:

- **§7.12's extensions dictionary** (6 rows): the catalog's `/Extensions`, naming the *developer*
  extensions a file was written against — `/ADBE` at extension level 8 over a base version of
  1.7, and so on. 9 of the 974 corpus documents state one.
- **§12.11's document requirements** (6 rows): the catalog's `/Requirements`, naming "feature(s)
  of PDF beyond those commonly expected … required for correct handling in accordance with this
  document". 0 corpus documents state one.

Neither draws anything. Both are a document telling a processor, before a page is on the screen,
that it wants something the processor may not have.

## Decision

**Read both; report neither as a page defect; say it once when the document opens.**

`requirements.rs` reads Table 273's requirement dictionaries, Table 275's twenty-four types and
Table 48/49's extensions, and `requirements::unmet` names each requirement this program cannot
meet **with the reason**. `viewer-ui` prints those lines at open time, beside the one
`--no-sandbox` already prints about what it gave up.

### Why not a page report

Principle 3's reflex is that unsupported input must stay loud, and this looks like unsupported
input. §12.11.6 says it is not:

> If the reader encounters an unsupported feature (whether or not that feature was declared as a
> requirement), it shall take the normal fallback actions.

and NOTE 1 adds that "there is no formal connection between the requirement type and the
operation of the associated feature(s)". The declaration is a statement about the *document*; the
place a missing feature gets reported is where the feature is used, which is what every other
report in this tree already does. A page whose content stream draws perfectly does not become
incomplete because the catalog said the document wants form interaction — and trap 11 prices that
mistake in gated pages.

### The departure, stated as one

§12.11.6 also says that when the requirements cannot be met "then the processing of the document
shall not continue". This program continues. Two reasons, and the first belongs to the standard:
the test it refers to is §12.11.3's penalty computation, and **§12.11.3 states no threshold** —
100 means "this document will not produce the author's intent" and intermediate values "are
available to weight the value of this feature among other features … as well as when contributing
to the total penalty points to weigh against other documents in the choosing process". There is no
comparison to complete without a second document to choose between. The second reason is a
choice: refusing to open a file a person asked for is a worse failure for a viewer than showing it
with its limits named, and the clause's own fallback sentence is what makes that safe.

## `Kind::unmet` is a claim about this tree, and it will rot

Twenty-one of the twenty-four types answer with a sentence saying why this program does not meet
them; three answer "met". That table is not derived from the standard — it is this project
describing itself, in the same way a ledger row does, and it decays the same way: the session that
builds a layer panel has to come back and change `OCInteract`. It is written as a sentence per arm
rather than a boolean precisely so that the reason is visible when somebody reads it against the
code.

The three met ones are worth naming: `OCAutoStates` (§8.11.4.4's usage application dictionaries,
ADR 0044), `Navigation` (links, outlines and the three actions the clause lists, ADR 0070), and
`Encryption` (§7.6 at every revision and method, ADR 0031). Each was a session's work, and the
requirement is how a document would have asked for it.

## Consequences

- `silent` falls 146 → 132; clause 7 has **five** silent rows left — §7.11.4's embedded file
  streams, §7.11.6's collection items and §7.7.4's name dictionary — and clause 12 falls to 82.
- Nothing renders differently, and no gate moves.
- `/Extensions` is read and acted on by nobody, deliberately: an extension level says which
  developer's additions a file uses, and this reader implements the standard rather than
  anybody's additions.
