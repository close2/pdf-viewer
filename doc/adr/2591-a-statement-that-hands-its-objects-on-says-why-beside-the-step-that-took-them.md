# 2591 — A statement that hands its objects on says why, beside the step that took them

Status: implemented — session 1878, the eighth slot of batch 128 (an archive round). Context: ADR 2562 (a halftone
row's `append-statement` hands on to `discard`), `doc/rfc/0014` section 3 (a page that cannot be composed hands each
object to the next step, and the report names each object's step), ADR 2590 (the same slot's font row).
Code: `crates/pdf-transform/src/archive/mod.rs` (`unstated`, `statement_not_composed`, `statement_handed_on`),
`crates/pdf-transform/src/archive/report.rs` (`Unstated`, `Conversion::unstated`),
`crates/pdf-transform/src/archive/statement.rs` (the composition-failure sentences, `NOTHING_LISTED`).

## Context

At a halftone row whose list is `append-statement` then `discard`, a statement page that could not be composed hands
the site's objects to `discard`. `statement_not_composed` answered that case with `None`, so the removal went ahead
and the report said `discard` with nothing beside it: the composition's own sentence, the only record of why no page
lists the objects, was dropped. RFC 0014 section 3 asks the report to name each object's step, and a step taken in
place of the named one is only legible with the reason the named one failed.

## Decision

- `unstated` is the one question both callers ask: where the row names `append-statement` and no composed page lists
  the site, why not, and the step that takes the objects where the row hands them on. `statement_not_composed`
  refuses with its sentence where nothing takes them; `statement_handed_on` makes a `report::Unstated` (site, step,
  sentence) where something does.
- The record is pushed into `Conversion::unstated` only where the site's decision proceeds. Where the step it was
  handed to refused, that refusal is the site's reason, and a statement nobody composed is no part of it.
- The text report carries one line `not stated: <site>: … handed to <step> — <sentence>`; the JSON carries an array
  under `unstated`. The decided row is unchanged.
- The six composition-failure sentences (`TOO_MANY_STATED`, `STATEMENT_TOO_LONG`, `STATEMENT_TOO_MANY_PAGES`,
  `NOT_A_STATED_ROW`, `NOT_IN_THE_SOURCE`, `PAGES_NOT_WALKED`) ended "so the loss is not taken". At a site that hands
  on, the loss is taken, so the sentence would contradict the line it stands in. They now end "so no statement page
  is composed", which is true at both kinds of site. Where an embedded-file row's statement fails, the refusal's
  sentence changes accordingly and nothing else does.
- A composed page that lists none of the site's objects says so in `NOTHING_LISTED` at a site that hands on, and
  keeps `NOTHING_STATED` ("the loss is not taken unstated") where it refuses.

## Consequences

- The census against HEAD's carries no `not stated:` line, with the pack or without it: no corpus document reaches a
  halftone statement that fails to compose. No count moves.
- `a_halftone_statement_that_cannot_be_composed_says_why_beside_the_discard` (`tests/archive/statement.rs`) is the
  test; with the push taken out it fails.
