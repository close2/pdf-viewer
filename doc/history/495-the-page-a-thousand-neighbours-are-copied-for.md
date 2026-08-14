# 495 — The page a thousand neighbours are copied for

**Finding.** `doc/todo/47` said the last fifth of a cold document-wide search was §7.7.3.2's page
tree and proposed an index of leaves; it also owed the round one measurement first — *measure
`find_leaf` before believing the walk is all descent* — and that measurement said the index was
the wrong answer. The walk was not spending its instructions descending. **Two fifths of it was
§7.7.3.4's inheritance applied to every node the walk stepped over and discarded on the next
line**, and the rest was `Document::get` handing back a deep copy of each neighbour so that Table
30's `/Count` could be read off one page dictionary at a time. ISO 32000-2's root has nine children
and one of them holds 998 of its 1023 pages, so finding page *n* copied about *n* page
dictionaries — quadratic in the page count, which is why the file's own "1.10 ms on the thousandth
page" was a cost that grows rather than the flat one it looked like. The fix holds a node as a
*name* until something needs the node, costs **no memory at all**, and takes 12.1% off a whole
cold sweep.

**Date.** 2026-08-14.
**ADR.** [0330](../adr/0330-the-page-a-thousand-neighbours-are-copied-for.md).
**Touched.** `crates/pdf-syntax/src/document.rs` (`Document::get_key_of`, the private `Held`, one
test), `crates/pdf-model/src/page.rs` (`Node`, and the four walks that take one),
`crates/pdf-model/tests/page_tree_nodes.rs` (one test), `doc/conformance/ledger.toml` (§7.7.3.2's
row: the new test and what Table 30's sentence now pins), `doc/todo/47-search-performance.md`
(amended — the page tree is done, and what the measurement found next is the new item),
`doc/adr/0330-*` (new), this file.

## The numbers, and how they were taken

Wall clock was not available: the machine carried a load average of 55 for the whole round, which
is ADR 0317's situation and its method — two binaries from one tree, counted with callgrind, same
document, same invocation:

```sh
valgrind --tool=callgrind --callgrind-out-file=<arm>.out \
    <arm>/find_cost doc/ISO_32000-2_sponsored_EC3.pdf zzzqqqxyzzy 0 split 100000
```

| one cold sweep, ISO 32000-2, 1023 pages | instructions | |
|---|---|---|
| whole run, before / after | 42 884 714 194 / 37 676 397 310 | **−12.1%** |
| `Pages::get`, before / after | 7 324 564 135 / 2 123 154 570 | **−71.0%** (17.1% of the sweep → 5.6%) |
| `interpret_with`, before / after | 34 826 824 963 / 34 819 564 096 | −0.02%, which is the check |

The readback is byte-identical at 2 658 697 bytes, which is `doc/todo/47`'s stated gate.

**One walk gains nothing, and it is the honest shape of the change rather than an exception.**
`Pages::indices` visits every node once and steps over none, so there is no copy to avoid — a
first touch has to parse the object whoever asks. `Outline::section_at`, on the open path, is
88 529 236 → 89 226 065: **0.8% worse**, two cache lookups where there was one.

## What the round did not take, and why it is written down

`Dictionary::get` builds a heap-allocated `Name` for **every dictionary lookup in the program** —
1 529 118 804 instructions, 4.1% of a cold sweep after this change and 2 503 477 461 before it. The
fix is `impl Borrow<[u8]> for Name` and a few lines, and `Name`'s ordering is its bytes' ordering,
so the map's invariant survives it. It is the hottest path in the tree, it belongs to no clause,
and folding it into this round's A/B would have made two changes one number. `doc/todo/47` carries
it as the item that is left.

## What the next round should know

- **A cache was the wrong instrument twice in a row on this item, and the second time it was
  avoided by measuring one level in.** `doc/todo/47` asked for "an index of leaves or a cache of
  resolved `Page`s" and priced their memory against each other; the answer was neither, because the
  question assumed the descent was the cost. The general form is in `doc/habits.md` already —
  *attribute by removing the suspect* — and this is the version where the suspect was named in the
  todo file and was wrong.
- **The oracle ran with an empty reference cache** (0.0% hit rate, 6175 references produced), which
  is what a fresh worktree target directory means rather than what trap 10a warns about. Its two
  tests pass and its ratchets hold; the corpus gate is identical to the base line for line,
  compared as sets against a run of the same tree with the change stashed.
- **`doc/todo/02` §5 was not run.** The release binaries a person runs are built from `main`, and
  this round's target directory is the worktree's; whoever merges owns that section for the merged
  result, exactly as §2 now says of the gates.
