# 1922 — An appended page extends a page-label tree of any shape

Session 1922, a single archive round on the server VM. ADR 2671; no question, trap or habit. Worked
in the rounds worktree and its `scratchpad/r1922/`.

**Why.** ADR 2669 left the Defence document refused under both stating profiles because its
appended page "cannot be composed", attributed to page labels in a number tree with `/Kids`.

**The premise, measured.** The Defence document's `/PageLabels` is flat: a range for each of the
thirteen pages its root counts, twelve held. Its refusal was the key at 12, the appended page's
index, which `LABELS_NOT_EXTENDABLE` refused under a sentence naming only `/Kids`.

**Built (ADR 2671).** `labels_extended` reads the whole tree once (`LabelWalk`) and extends it: a
`/Nums` root takes the pair; a `/Kids` root takes a new indirect leaf, `/Limits [n n]`, as its last
kid, so the root is the only producer node rewritten and no `/Limits` stops being true. A flat
tree's range at a page index not held labels none of the pages carried and is taken out. Five named
refusals replace the one: not a tree (non-dictionary node, non-array entry, odd `/Nums`,
non-integer key, root of both), cycle, out of order, past the pages (in a leaf under `/Kids` only),
deeper than 64.

**Tests.** Four unit tests in `archive::preserve` (flat, Defence shape, two-level, each refusal
planted, the depth bound both sides); `the_statement_page_extends_a_label_tree_of_any_shape_with_a_range_of_its_own`
in `tests/archive/recovered.rs` reads three shapes back through `PageLabels` at both targets.
Withholding the new leaf fails it (appended page labelled `A-3`). The refusal test's fixture now
plants a key at the page not held in a leaf.

**Census.** ADR 2669's command, against its own after-run: the Defence document converts under
both profiles at 2b and 4, nothing is newly refused. `bug_jpx`, `issue5567`, `red_stamp` and
(`state-every-loss`, part 4) `bug1815476` also left the refused lists: the after-run of round 1921
could not reach the sandbox worker that decodes JPX (trap 10), and this run built it in its hold.
The four Defence outputs pass veraPDF 1.30.3.

**Ledger.** 7.9.7 and 12.4.2 stay `implemented`, with code, tests and notes naming ADR 2671.

**Gates.** `cargo fmt --check` of `pdf-transform` and `pdf-archive` and `clippy -D warnings
--all-targets` of `pdf-transform`: exit 0. `nextest` of both: 1327 passed, exit 0. The archive tier
behind the walk lock (`corpus`, `archive_corpus`, the witnesses): exit 0. `cargo test -p
conformance` after `git add -N`: exit 0. No device frame.
