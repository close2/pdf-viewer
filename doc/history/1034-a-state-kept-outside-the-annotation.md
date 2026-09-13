# 1034 — A state kept outside the annotation, and a debt kept in the wrong row

Ledger slot of batch 1032–1037, five siblings in one worktree. Contract: §12.5's sixteen `partial`
rows. **Three closed, all by reading the clause rather than the note; one new module, nine
calibrated tests, one census. No pixel moves.**

| row | was | now | what closed it |
|---|---|---|---|
| §12.5.6.3 Annotation states | `partial` | **`implemented`** | `pdf_model::annotation_state`, new |
| §12.5.6.4 Text annotations | `partial` | **`implemented`** | its whole residue was Table 175's `/State` and `/StateModel` |
| §12.5.6.5 Link annotations | `partial` | **`implemented`** | the residue is §12.6.4.6's and §12.6.4.8's, and both rows carry it |

**1. §12.5.6.3 is a walk, which is why nothing had done it.** "The state is not specified in the
annotation itself but in a separate text annotation that refers to the original annotation by means
of its IRT entry" — so the answer is a fact about the *other* annotations on the page, and neither
`/State` nor `/StateModel` had a reader anywhere in `crates/`. `annotation_state::states` walks the
page's replies breadth-first from the original. Three decisions, in ADR 1051 because a later round
should not re-open them: depth is the chronology (the clause says "in reply to the previous reply
for a given user" and names no date); a `/State` with no `/StateModel` takes the model Table 174
puts that state under, rather than being refused; a name outside Table 174 is carried as the file
spells it. `/T` is read through `markup::group_source`, because §12.5.6.2 makes it a group
attribute.

**2. The corpus cannot rank any of it and the standard's own PDF can.**
`examples/annotation_state_census`: **0** `Text` annotations with an `/IRT` in the 963 pdf.js
documents that open, out of 34 835 annotations — and **1752** in ISO 32000-2's own 11 462, all
`Review`, 1205 `Completed` and 547 `Accepted`, of which **664 reply to another state change**, which
is exactly where the depth rule decides. The fixtures are therefore hand-built (trap 8) and each of
the nine was watched fail with its own rule removed (trap 13); the cycle guard's removal does not
fail a test, it fails to terminate.

**3. §12.5.6.5 was `partial` for a neighbour's debt.** Its residue was "a link whose action is a URI
or a launch leads nowhere here by design". Table 176's `/A` *is* performed — `link::at`,
`action::read`, `viewer_core::interact` — and which actions are performable is §12.6.4's subject:
§12.6.4.6 is `reported` and §12.6.4.8 is `partial`, so the debt is owed and reported, twice, in the
rows that own it. `/PA` needs no reader (a permission for an editor of annotations), and `/QuadPoints`'
underline case is not a debt either — Issue #17's NOTE 1 says the activation area and the visual
appearance are not required to be the same.

**Gates.** Tier 1 whole, green. Tier 2 owed because `pdf-model` is rule 2's, and run — exit codes
in the round's report. No pixel can move: `grep -rn annotation_state crates/` finds one line
outside the module and its census, and it is `pub mod` in `lib.rs`. Five siblings edited this
worktree throughout, so every gate but `conformance` measured their trees as well as this one;
`raster_golden.tsv` was already modified when this round began.
