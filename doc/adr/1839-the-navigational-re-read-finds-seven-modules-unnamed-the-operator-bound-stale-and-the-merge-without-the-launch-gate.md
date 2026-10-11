# 1839 — The navigational re-read: seven modules unnamed, the operator bound stale, and a merge that does not run the launch gate

Session 1501. Status: **accepted**. ADR 1638 rule 7's re-read of the four navigational documents,
owed since batch seventy-seven, under `CLAUDE.md`'s *Where knowledge lives* and ADR 1023's rule. It
decides no new rule; it records what was false so that the next re-read starts from the residue.
Context: ADRs 1023, 1638, 1680, 1760.
Prose: `doc/PLAN.md`, `doc/crate-map.md`, `doc/state-of-play.md`, `doc/HANDOVER.md`; `doc/todo/02`
section 2.

## 1. How the four were checked

As ADR 1760 section 1, in the order its habit puts first ("a navigational re-read starts with a
census of modules"). The census: `git diff --diff-filter=A 522408c2 HEAD --name-only`, filtered to
`/src/.*\.rs$`, against each crate-map row's text, word-bounded, by a throwaway script;
`cargo metadata --no-deps`: 43 members, 43 rows. Then the pointer sweep, `tools/state.sh
navigation` before and after: one absent pointer in the four, `doc/questions/A170`, the owner's
uncommitted answer, as at the last two re-reads. Then a throwaway extractor over every backticked
code name in the four, each segment looked up in the tree's sources, calibrated first by a planted
file of three false names (three of three named): one real hit, `MAX_OPERATIONS`. Then the records
of sessions 1468 to 1495 read for what they built, and the sentences the census and the records
pointed at read as *what is*. Last, `doc/todo/02` section 2's tier blocks against `tools/batch.sh`'s
`gates` function, line by line.

## 2. What was false, and what replaced it

| the document said | the tree says | checked by |
|---|---|---|
| `MAX_LIST_BYTES` "asked where `MAX_OPERATIONS` already is" (crate map, `pdf-model`) | no operator bound exists; the list's bytes, paths and readback charged, half the worker's address space | ADRs 1809, 1810; `grep -rn MAX_OPERATIONS crates` |
| the list "is bounded in bytes as well as in operators" (`state-of-play.md`) | bounded in bytes; no count of operators refuses a page | the same |
| "Explicit memory and time budgets" (`PLAN.md` section 6, built) | memory budgets; time bounded by a stop — the person's, or a caller's deadline where nobody watches | ADR 1809 section 4; `REQUEST_TIMEOUT` |
| `encode_in_strips` cuts where `unsplittable_rows` permits (crate map, `render-cpu`) | `unsplittable_rows_where`, asking the backend per stroke (`drawn_row_by_row`) | ADR 1819 |
| the three rasterisers depend on `pdf-render` "and on nothing else of ours" (`PLAN.md` section 3) | `render-raster` depends on `raster-gpu` and `raster-scene` | `cargo metadata` |
| both renderers "rasterise on the CPU single-threaded" (`PLAN.md` section 4a) | `hayro-speed` runs both on one thread; `render-cpu` divides a page everywhere else | `hayro-speed.rs`'s header |
| seven modules named nowhere | `progress.rs`, `rich_text/span.rs` (and `rich_text/`), `engine/util.rs`, `glimpse.rs`, `notice.rs`, `loading`, `quorra-confined/exchange.rs`, each named in its row | the census |
| the device's frame budget, `tile_price`, `heavy_drawing` named nowhere | `raster-gpu`'s and `render-raster`'s rows; `state-of-play.md` beside ADR 1472's tiles | ADRs 1817, 1823 |
| `HANDOVER.md`: no row for the owner's target; a fuzz campaign "behind the lock" | a row naming `doc/todo/69` and the instruments; a `--long` run on the second lane | ADRs 1756, 1817 |
| `launch_path` "the one line here that is `--release`" (`doc/todo/02` section 2) | `turn_path` and the raster examples are `--release` too | the tier 2 block |
| rule 5: "A merge runs everything, always — all three tiers" | `tools/batch.sh gates` runs 33 lines — two builds, 11 of tier 2's 12, all 20 of tier 3 — and not `launch_path` | `gates()` in `tools/batch.sh`; `git log -S launch_path -- tools/batch.sh` is empty |

`state-of-play.md` also gains the owner's target of 2026-10-10, once, in its header, marked as the
one sentence there that is not a capability.

## 3. What the hypothesis said, and what held

The brief guessed that the four still described the operator bound and the in-process open as they
were before batch seventy-seven, and that the gate map was current. **The first half held for the
bound** — in three of the four documents — **and not for the open**: rounds 1487, 1494, 1495 and
1492 had each written their capability into `state-of-play.md` themselves (the loading bar, the page
shown as it is built, the confined window answering, the refused page paid once), so the drift was
the crate map's, which none of them owned. **The second half did not hold**: `tools/batch.sh gates` has never
run `launch_path`, and the sentence that said the merge does is the map's own safety rule. The premise's count
also moved: the brief's pathspec printed 5 added modules, and re-derived at this HEAD it prints 3,
because git's pathspec `**` is not the shell's; the filter in section 1 prints 7, all unnamed.

## 4. Left to its owner

- **Whether the merge runs `launch_path`** is `tools/batch.sh`'s owner's: add it as a clock gate
  beside `t2-turn_path`, or keep it the round's. The section now states which is true today.
  `doc/todo/02` section 8 step 4 ("`tools/batch.sh gates` for tiers 2 and 3") is outside this
  round's section and carries the same omission.
- `crates/render-cpu/src/glimpse.rs`'s header says the witness's frame "grants it no cut at all and it is
  one strip", which ADR 1819 made false; a code comment, a sibling's file.
- `doc/todo/69` says a generator of the witness's shape "is what a gate can hold, and the gate owes
  one"; `examples/heavy_drawing` is that generator (ADR 1823), and no gate holds it yet.
- If this batch's slot 5 (fills drawn a row at a time) lands, the `render-cpu` row's "of each
  stroke" becomes "of each stroke and fill".
- Carried, not checked: the crate map's "Six of the eight generators" (`pdf-vfs`), and
  `state-of-play.md`'s "four documents … with 115 beads".
