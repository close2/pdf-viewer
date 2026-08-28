# 775 — The first host on the confined boundary

**Subject**: `doc/todo/15` — road B's machinery was complete and had no consumer; ADR 0657's own
words were "no host takes that road yet". This round built the host: **`pdf-viewer-confined`**, a
window whose every page arrives from `pdf-view-worker` over the pipe, on both of ADR 0607's
payload arms. ADR 0713 has the argument; this file is the bookkeeping.

## What landed

- `crates/viewer-ui/src/bin/pdf-viewer-confined.rs` and `pdf-viewer-confined/screen.rs` — the
  window and the screen bookkeeping between a frame arriving and its pixels existing. Scope is
  deliberately the smallest complete host (open, arrange, turn, scroll, zoom, report, abort),
  with everything outside it refused by name; the level-hosts rule is argued *not* to extend to
  it (ADR 0713 §2).
- `viewer_host::drawing` became generic over its request (`trait DrawRequest`,
  `Drawing<R = RenderRequest>`), because a confined host cannot hold a `RenderToken` and a second
  copy of the queue is what `viewer-host` exists to prevent. `POLL` and `SETTLE` moved to module
  constants — an associated constant on a generic type cannot be named without its parameter —
  and the two native hosts' call sites moved with them; nothing else in them changed.
- `viewer-ui` gained the `viewer-confined` dependency, with the manifest comment saying why the
  flagship's launch path still links no transport.
- `tools/state.sh`'s `names_in_code` excludes the confined window's sources: counted under
  `viewer-ui` it made the `windows` section report the tier-2 window asking `Query::Frame` and
  fired `SPENT` on a reason that had not been spent (details in the script's comment and ADR
  0713 §1).
- Documents brought along: `doc/todo/15` (host exists; cancel-from-a-host proven; remainder
  sharpened), `doc/todo/34`, `doc/todo/README`, `doc/ui-boundary.md` (six consumers → seven),
  `doc/state-of-play.md`, `doc/crate-map.md`, `doc/todo/02` §5's install block, and
  `viewer-confined`'s own "What this is not, yet".

## Trap-13 calibration of the six new `screen.rs` tests

Each defect injected separately, watched fail, then reverted; the suite is green as committed.

| injected defect | failed |
|---|---|
| blit ignores the origin | `a_raster_payload_is_placed_at_its_origin`, `a_page_off_the_window_edge_is_clipped`, `a_list_payload_is_drawn_and_then_composes`, `a_moved_page_keeps_its_pixels` |
| `landed` skips the identity check | `a_stale_draw_is_dropped_rather_than_placed` |
| a moved page's pixels are not reused | `a_moved_page_keeps_its_pixels` |
| `take` never calls `Drawing::superseded` | `a_page_that_leaves_the_arrangement_interrupts_its_draw` |

## Proof under Xvfb

Release builds on display `:175`, 900×1100; ADR 0713 §"Proof" has the full account. The marks
arm (`PDF20_AN001-BPC.pdf`: worker confined in 9.4 ms, `1 as marks`, first frame at 0.126 s,
page turns, zoom, scroll, captures read); the raster arm (`personwithdog.pdf`: `0 as marks`,
photo on screen at 0.123 s); the abort (amplification level 4: Escape kills the worker and
interrupts the draw, exit joins in 0.095 s); the refusal (`issue6010_1.pdf` named by sentence).
The first abort run left the worker `<defunct>` — a `Canceller` kills without reaping — and
`stop` drops the `Confined` handle now; the re-run shows no zombie.

## Gates — the full §2 sequence (775 is a fifth round)

- `cargo fmt --all --check` — clean.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2712 tests, 2711 passed, 18 skipped, 1 failed: the known
  load flake** `viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it`
  (three sibling rounds were running). Per the standing instruction it was rerun alone and
  **passed**; both results recorded here, the test unchanged. A first attempt at the whole suite
  earlier in the round died during its build phase with no compile error — the shape
  `doc/environment.md` attributes to a neighbouring round — and was simply rerun.
- `cargo test --workspace --doc` — exit 0.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- All thirteen remaining lines exit 0, run in §2's order with nothing beside them:
  the two `--profile gates` builds, corpus (974 documents, 67 incomplete, 0 slow), oracle
  (1945 pages in 249.3 s: 983 agree, 61 contradicted, 836 ambiguous, 42 not comparable — a loaded
  machine's figure, the gate's own ratchets holding), the three text gates, both censuses, dates,
  xmp, jpeg2000, the quorra corpus gate, fixed_documents, and `cargo test -p conformance`.

## Sweeps

`parts`, `pointers` and `quotations`, all exit 0. `parts` was diffed against its own run on
`main`: the only differences are line-number shifts in two files this round edited — **no
sentence counting the tree's parts decayed**, because the new window is a binary inside
`viewer-ui` rather than a new crate, host row or worker. The consumer count that did decay —
`doc/ui-boundary.md`'s "six consumers" — was found by reading and updated to seven with the new
window named.

## Deliberately not done

- `doc/todo/02` §5's install into the main tree's `target/` is left to the merge round: this is
  an unmerged worktree branch, and installing its binaries over `main`'s would hand a person a
  neighbour of what `git log` says they have (trap 15's shape). The round's own measurements ran
  from this worktree's release build.
- The ledger is untouched: the change implements no clause — the clauses its comments cite
  (§7.6.4.1, §12.4.2, §12.6.4.8) were long settled — and `cargo test -p conformance` is green.
- CI on `main` was already failing when the round began (`tools/round.sh` said so); nothing here
  touches it and `gh` has no token under this account to read the log.
