# 2466 — The docs step carries a data bound of its own and says when it was the bound

Session 1816, an "instruments" round of batch one hundred and twenty-two, 2026-10-27. ADR 2467 is
the same round's second ADR. Status: implemented — the owner's word of 2026-10-27, "Fix the two
instrument traps (trap 280's 12 GiB docs walk limit and trap 281's export timestamp touch) in batch
122." Context: trap 280, ADRs 1938 and 2190 (the step), ADR 0798 and `tools/bounded.sh`'s header
(the 12 GiB rule). Cites no clause and moves no ledger row.

## 1. What decides it

`tools/state.sh docs` ran rustdoc with warnings denied over the whole workspace as a `walk large`,
which is `tools/bounded.sh --tree 12` with the default `RLIMIT_DATA` of 12 GiB. In batches 119 and 121
the step exited 101 with nothing but "could not document" lines, while every crate rustdoc'd alone
was green: the cause, read from the command run by hand, was rustdoc's allocation failure while it
rewrote the workspace's shared search index (trap 280). `RLIMIT_DATA` counts address space the
allocator hands out, and the tree ceiling counts what is resident; the step crossed the first at a
resident size far under the second.

## 2. The change

- **The walk carries `--data` of its own** (`DOCS_DATA_GIB` in `tools/state.sh`), above the
  address-space peak measured from the rustdoc processes' `VmData` through whole runs, and the
  12 GiB resident tree ceiling is kept. The lint is not touched: `-D warnings`, every crate,
  `--document-private-items`.
- **The step's filter keeps the wrapper's own sentence** (`bounded: exit`, `STOPPED`, `KILLED`), so a
  stop by the data bound, or the exit line's resident peak under it, reads in the step's tail and not
  only in a run by hand.
- Per-crate rustdoc with a merged index was not taken: the merge flags are unstable, and each crate's
  run rewrites the same index anyway.

## 3. What was measured

In an export of HEAD whose target held a reflink copy of the batch's `doc/` (the state in which the
batch's own walk failed), with the sampler reading the `VmData` of the rustdoc processes whose working
directory is the export:

- Under `--data 12` the walk failed as in batches 119–121: "memory allocation of 1879048192 bytes
  failed" in the search index's `stringdex` tree, `could not document` for the dependents, and the
  largest `VmData` seen was 11.0 GiB.
- The same input under `--data 24`, by `tools/state.sh --round 1816 docs`, exited 0 after 951 s, with
  the largest `VmData` 16.2 GiB and the tree 7.80 GiB resident under the 12 GiB tree ceiling.
- On an export whose `doc/` started empty the walk passes under 12 GiB (73 s), so the bound matters
  to a target that already holds a workspace's documentation, which is the batch's.
- A planted unresolved intra-doc link in `pdf-ccitt`'s module comment: exit 101, `error: unresolved link
  to ...`, then `could not document pdf-ccitt`.

A first sampler read every rustdoc process on the machine and was discarded: a sibling's doctests
contaminated it.

## 4. What it costs

A larger address-space allowance for one walk in the lane, bounded in memory by the unchanged tree
ceiling. `tools/bounded.sh`'s header rule 2 (`--data` never above 12 GiB) is kept for every other
walk; this step is the one exception, named here and at its constant.
