# 1923 — A parent tree of any depth takes the appended pages' entries

Session 1923, a single archive round on the server VM. ADR 2672; no question, trap or habit. Worked
in the rounds worktree and its `scratchpad/r1923/`.

**Why.** `read_the_parent_tree` refused a tagged document's appended page wherever `/ParentTree`
held `/Kids` (`PARENT_TREE_NOT_EXTENDABLE`), and round 1922's census named eight corpus documents
refused by it under the stating profiles.

**The premise, measured.** `mutool show` of each of the eight: a parent tree root of `/Kids` alone,
in an object of its own. It held.

**Built (ADR 2672).** ADR 2671's walker and extender became one `NumberTree` in `preserve.rs`, read
by both trees under refusal sentences of their own (`TreeRefusals`). A `/Kids` root takes one new
leaf as its last kid, `/Limits` the least and greatest of the pairs it holds; a `/Nums` root takes
them at its end, a `/Nums` stated by reference written into the root. Keys come from
`/ParentTreeNextKey`, corrected past the greatest key read, and `extended` checks they ascend. The
rewrite puts back only `/Kids`, `/Nums` and `/Limits`. Four refusals replace the one: not a tree,
cycle, out of order (now in a flat tree too), deeper than 64.

**Tests.** Three unit tests in `archive::tagged` (two levels with an understated next key, a `/Nums`
by reference, every refusal and the depth bound both sides).
`a_parent_tree_with_kids_gains_the_statement_pages_entries_at_any_depth` in
`tests/archive/statement.rs` converts three shapes at 2a and part 4 and reads every page back
through `pdf_model::structure::ParentTree`. Withholding the new leaf fails it, though this tree's
validator still passed that output.

**Census.** ADR 2669's command against round 1922's after-run: no parent-tree refusal remains and
nothing is newly refused. `state-every-loss` converts `064034`, `316267`, `459011`, `UMTRI-2008-80`
at 2b and `064034` at 4; the rest stay refused by other rows. The five outputs pass veraPDF 1.30.3.

**Ledger.** 7.9.7 and 14.7.5.4 stay `implemented`, notes and tests naming ADR 2672.

**Gates.** `cargo fmt --check` of `pdf-transform` and `pdf-archive`, `clippy -D warnings
--all-targets` of `pdf-transform`: exit 0. `nextest` of both: 1331 passed, exit 0. The archive tier
behind the walk lock (`corpus`, `archive_corpus`, the witnesses): exit 0. `cargo test -p
conformance` after `git add -N`: exit 0. No device frame.
