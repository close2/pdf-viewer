# 1809 — No count of operators refuses a page

Asked by the project owner on 2026-10-10, as the first of batch seventy-seven's four parts: "I don't
like that we limit the number of operations, if we have valid files, which are not shown. As long as
it isn't a resource problem (only a time problem) we should allow it, and possible display a loading
bar." Session 1486. Status: **accepted** and **built**. Supersedes ADR 1808's figure and the
operator half of ADRs 0271 and 0306; ADR 1810 is the memory half that this one needs. Context: ADRs
0241, 0657, 0810. Code: `crates/pdf-model/src/content/run.rs`, `content/pattern.rs`,
`content/list_budget.rs`, `content.rs` (`Interpreter::operations`), `content/report.rs`
(`Interpretation::operators`). Tests: `tests/hostile_budgets.rs`
(`a_hundred_million_operators_are_run_to_the_end_of_their_stream`,
`a_stream_of_operators_past_its_byte_bound_is_ended_by_it_by_name`), `tests/tiling.rs`
(`a_cells_operators_are_read_once_and_not_once_per_site`).

## 1. The decision

`MAX_OPERATIONS` is gone. The interpreter still counts operators where §7.8.2's grammar puts them —
after the operands, ADR 0306's site — and the count refuses nothing. A count bounds time and not
memory, and **a count is not a cost**: one `sh` paints the whole page, one `Do` runs a form a million
operators long. Time is the person's to stop, which is ADR 0657's rule (no automatic deadline: "the
frame nobody will look at") and ADR 0241's mechanism (the confined worker is killed when the person
asks, in 0.83–1.97 ms).

## 2. What the counter is still for

- **The window's progress.** Round 1487's `crate::progress::publish(self.operations, …)` rides the
  list budget's periodic branch in `run.rs`, which stays where it was and is asked every
  `CHECK_EVERY` (64) operators; the count is the numerator it publishes.
- **A figure on the result.** `Interpretation::operators` is the count over every stream a run read
  — the page's, and each nested stream as often as it was read. It is what keeps
  `a_cells_operators_are_read_once_and_not_once_per_site` an instrument: with no bound to be refused
  by, re-reading a tiling cell per site would cost only time, and the figure (under 300 000 against
  34 million) is what still tells the two apart. `tests/replacement.rs` and
  `examples/replacement_census` hold it equal across a checkpoint.
- **A census figure.** `examples/content_budget_census` prints operators and tokens per page; its
  four-million line is kept as the figure the crawl's populations were counted at.
- **Pacing.** The list budget's slow path runs when the count reaches `ListBudget::next_check`,
  which is no longer capped at the old bound.

## 3. What replaces each use of the count as a budget

| where the count was a budget | what bounds it now |
|---|---|
| the top-level stream, `run.rs` | its bytes: every token spends at least one, and Table 31's `Limits::max_stream_len` (1 GiB) bounds the concatenation, by name (`ContentIssue::TooLarge`) |
| a run that re-reads nested streams | what it keeps, `MAX_LIST_BYTES` (ADR 1810), and the depth, `MAX_FORM_DEPTH`; a run that keeps nothing is the person's to stop |
| a tiling's copies, `pattern.rs` (`repeat_cell`) | `MAX_TILE_COPIES` per tiling, and the copy's bytes against `MAX_LIST_BYTES` page-wide, both asked before each copy; copies no longer add to the count, because a copy is not an operator |
| `list_budget.rs`'s `next_check` cap | nothing: the cap only made the count's refusal land in the same branch |
| the marked-content stack's comment, `run.rs` | the stream's bytes, as the token loop's; a closed section's span is charged (ADR 1810) |
| `ledger.rs`'s `MAX_SELECTIONS` comment | its own bound, a million entries, unchanged |

**A hundred million `0 g` ends at the end of its stream.** The test deflates 400 MB of them into a
fixture under a megabyte, reads them through the window and asserts that nothing is reported and
the square after them is drawn: 15.9 s under the `dev` profile. The same stream under a
`max_stream_len` of 4 MiB is ended by that bound, by name, with nothing past it drawn.

## 4. Who has no stop, and gets the owner's rule

`quorra-transform render` and `pdf-vfs` interpret in-process with no person watching a progress
bar; a page there runs to its end, and a person stops it the way a person stops any command. The
`page` fuzz target's `-timeout=60` (`doc/verify.md`) now reaches what a count used to cut: a nesting
fan-out of streams that keep nothing runs for as long as its author chose. Such a timeout is the
owner's allowed time, not a defect; a loop that consumes no byte and re-reads no stream is still
one, and the two are told apart by the stack, the way `doc/verify.md` already classes a timeout
(ADR 1746).

## 5. The cost

A page whose author wants it slow is slow until the person stops it, in every host, and a host
without a stop (the two above, a test) waits. That is the owner's ruling, stated as a cost: the
alternative — a count — refused the owner's own valid drawing at four million and would refuse the
next one past any figure.
