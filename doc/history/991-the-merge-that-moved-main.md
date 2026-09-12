# 991 — The merge of 984–990, the answers, and the day `main` became the working branch

Date: 2026-09-12. Rounds merged: 984 (the direction review, ADR 1005), 986 (signatures reported
before they go, ADR 1007), 987 (the output intent travels with the conversion, ADR 1008), 988
(the form knockout group's witness was another shape, ADR 1009), 989 (the frontier read to zero
and the edition sweep as a test, ADR 1010), 990 (a salvage with one rule and the save gate, ADR
1011). The owner answered `Q53`–`Q60` the same day (committed on their own, `257f878e`) and
amended the questions convention — every `A` file now carries a `Status / Given / Owes` header and
`tools/state.sh questions` prints the index.

Files the merge itself changed: `crates/pdf-model/src/content/ext_gstate.rs` (987's intent into
988's file), `crates/pdf-model/src/soft_mask.rs` (`entry` deleted), `crates/render-raster/tests/corpus.rs`
(`issue18032.pdf` out of the refused list, 988's hand-off into 987's file), `doc/conformance/ledger.toml`
(§14.11.5 closed), `tools/conformance/tests/state_sections.rs` (990's parser restricted to fenced
blocks, and an unfulfilled expectation removed), `crates/pdf-syntax/src/lexer.rs`
(`#[inline(never)]` on the salvage, measured), `doc/reviews/984-…` (one figure corrected: the crate's
source is 93,413 lines, 18% of the tree, not 191,001 and 37% — the review's own command counted its
tests), four habits placed, `doc/history/990` written from ADR 1011 because the session limit cut
that round off before its record.

**The one regression the merged sequence found was the launch path's instruction count**, run
alone: `open_kinstructions` 26 773.8 against a band top of 26 760 and 185 492.1 against 185 424 —
0.04%, in a figure with no clock in it. Bisected by swapping HEAD's lexer in: 0 outside. The
salvage rewrite had lost its `String` and become small enough to inline into `read_number`, and
the reshaped register allocation cost every well-formed number a few instructions. `#[inline(never)]`
on the salvage puts both figures back in band; the attribute carries the numbers.

**And the layout changed on the owner's word**: "keep the projects/pdf-viewer directory as branch
`main` and use worktrees for the work", then "merge everything into main! main should be our
working branch!" `main` was a pure fast-forward of the campaign branch, so the ref was moved to the
tip — the tree the sequence ran on is byte for byte what `main` is — and the next batch runs in a
worktree on a short-lived branch that is fast-forwarded into `main` at its boundary.

Two edits are owed to a follow-up commit because the files that need them are owned by the owner's
account and not writable by this one: the `save_round_trip` line in `doc/todo/02` §2 (ADR 1011 §2
states it; `tools/state.sh` already runs it), and one `rustfmt` hunk in
`tools/conformance/tests/questions.rs`, which is why `cargo fmt --all --check` is red at this
commit and at no other line.
