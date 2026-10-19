# 2010 — The pooled drop is not compiled under the interpreter, and every test that reaches the pool declines by itself

Session 1587, slot 3 of batch ninety-three. Status: **accepted** and **built** in `pdf-render`.
Context: ADR 0450 (each test that drives a dependency's `unsafe` declines under Miri by itself, and
the workflow's line carries no `--skip`), ADR 1941 §2.3 (a list of 65 536 commands or more is let go
of on the pool), ADR 1994 (the share a window keeps). Code: `DisplayList`'s `Drop`
(`crates/pdf-render/src/display_list.rs`), `reach::tests`. It cites no clause and moves no ledger row.

## 1. What CI's advisory job said

The `nightly` job's Miri step failed on `group_cost::tests::groups_spanning_the_page_are_refused_past_the_bound`.
The test pushes past `LET_GO_ON_THE_POOL` commands, so the list's `Drop` hands them to rayon's pool,
whose work-stealing reaches crossbeam-epoch 0.9.20. Its `Local::register` takes a pointer from a
reference to an eight-byte list entry (`internal.rs:561`) and retags it as the whole `Local` that
holds the entry (`internal.rs:562`, a range of `0x180` bytes). Stacked Borrows forbids a retag
outside the range of the tag it is derived from, so Miri stops the run. This is the same
dependency and the same rule ADR 0450 found under `paint.rs`'s divided reduction. **The premise
held, and the hypothesis did not hold.** With the drop guarded, the run stopped again, this time at
`reach::tests::the_share_a_window_keeps_of_a_short_list_is_its_selection_counted`. ADR 1994 added
that test after CI's failed run. It reaches the pool through `MarkBounds::measure` and was the one
test of its module without the guard.

## 2. Decision

1. **Production code that hands work to the pool only to save time is not compiled under Miri.**
   `#[cfg(not(miri))]` sits on the `Drop` impl, its constant and its rayon import, and the comment
   beside it names the dependency and the rule. Under the interpreter the commands are dropped
   where the list is dropped, as any list under the bound already is. No test reads which thread
   dropped them. The build that ships is the same build as before: outside Miri, the library's
   macro-expanded source (`-Zunpretty=expanded`) is byte-identical to HEAD's.
2. **A test that reaches the pool through code whose answer is the pool's declines by itself**, with
   ADR 0450's `cfg_attr(miri, ignore = …)` and the reason pointing at `paint.rs`. A serial
   `measure` under Miri would bring back `reach`'s ignored tests, but it would be a second path for a
   function whose result the gates read. This ADR does not take that on.
3. **The workflow's line is unchanged.** Its comment already states the rule this follows.

## 3. Held by

`cargo +nightly miri test -p pdf-render -p pdf-syntax --lib`, the job's own line, on this machine
(`doc/history/1587`). A new `par_iter` reached from an unguarded test fails that line in the same
way. The job stays advisory, so a merge's tier 3 is where this is read.
