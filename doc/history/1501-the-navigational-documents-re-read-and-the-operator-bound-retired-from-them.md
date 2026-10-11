# 1501 — The navigational documents re-read, and the operator bound retired from them

Slot 6 of batch seventy-nine, 2026-10-10, a docs round. ADR 1839; ADR 1840 and Q397 not used. No
ledger row moved: the partial-row word yields to the owner's target, and no clause was touched.

**Premise.** Did not hold in its count. The brief's pathspec printed 5 added modules; at this HEAD
it prints 3, because git's `**` is not the shell's. Filtered by `grep -E '/src/.*\.rs$'` the diff
from `522408c2` lists 7, and all 7 were named nowhere in `doc/crate-map.md`: `progress.rs`,
`rich_text/span.rs`, `engine/util.rs`, `glimpse.rs`, `notice.rs`, `loading.rs` and
`quorra-confined/exchange.rs`.

**Hypothesis.** Half held (ADR 1839 section 3). The operator bound was stale in three of the four
documents (the crate map's `MAX_OPERATIONS`, `state-of-play.md`'s "in operators", `PLAN.md`'s "time
budgets"). The in-process open was not: the rounds that built the loading bar, the page shown as it
is built, the confined exchange and the refused page had written them into `state-of-play.md`. The
gate map was not current: `tools/batch.sh gates` runs 33 lines, and `launch_path` is not one of
them, while rule 5 said the merge runs every line of every tier.

**The re-read.** The census first (modules before pointer sweeps), then `cargo metadata` (43
members, 43 rows), the pointer sweep (one absent pointer, the owner's uncommitted `A170`), and an
extractor over every backticked code name, calibrated by three planted false names (three named),
which found `MAX_OPERATIONS`; then the records of sessions 1468–1495, and each sentence they
pointed at read as *what is*. **Corrected**, in ADR 1839 section 2's table: the seven modules, the
device's frame budget and the two heavy-page examples, the list budget, `render-cpu`'s strip
question, the rasterisers' dependencies, the speed comparison's one thread, the fuzz lane, a
HANDOVER row for the owner's target, `launch_path`'s `--release` and the merge's gate list; the
target of 2026-10-10 stated once in `state-of-play.md`'s header. Lines 765 → 771, 64 → 64,
1 075 → 1 085, 88 → 89; `wc -w` over the four 38 840 → 39 597.

**Unfinished**, each a sibling's or another owner's (ADR 1839 section 4): whether the merge runs
`launch_path`; `doc/todo/02` section 8 step 4's same omission; `glimpse.rs`'s "one strip"; todo 69's
owed gate for `heavy_drawing`; the `render-cpu` row's "each stroke" if slot 5's fills land.

**Files.** `doc/PLAN.md`, `doc/crate-map.md`, `doc/state-of-play.md`, `doc/HANDOVER.md`,
`doc/todo/02-every-round.md` (section 2), ADR 1839, this record.

**Gates.** `cargo test -p conformance --no-fail-fast`: exit 0, 430 passed, 0 failed. `--bin
quotations`: exit 0. `--bin pointers`: exit 0, one absent pointer in the four (`A170`).
`tools/state.sh --round 1501 navigation` before and after: exit 0, 0 history hits in each of the
four. No Rust touched, so no rustfmt, clippy or nextest; no walk, so no lock. Duration in the report.
