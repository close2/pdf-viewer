# 1526 — The inflate is pipelined into the divided read, not divided, and the next build is the drawer

Slot 5 of batch eighty-three, 2026-10-12. This was a measure round, the step back on the owner's
performance target. ADR 1888; no ADR 1889 and no question. No ledger row moved, because the
partial-row word yields to the target.

**Premise.** Held. The inflate is 163–175 ms whole and 185–202 ms through the window (1518: 173–212
ms). `git diff c6fb18fb -- crates/pdf-syntax/src` printed nothing.
**Hypothesis.** The first half held: the witness's one stream has zero `00 00 FF FF` restart points,
so dividing it needs rapidgzip's speculative decoder. The second half held in quantity but the other
way round: the pieces hide behind the inflate, not the inflate behind the pieces. Behind the
inflate, 48 pieces on 12 threads read in 268–306 ms against 394–442 ms after it. The read ends
63–79 ms after its inflate. The test was the output rate: the inflate writes at 1.69–1.80 GB/s and
the lexer reads at 0.71–0.76, so the inflate is 2.3× faster, and the exact planner on one thread
(359–397 ms) would be the floor. "The largest serial term" did not hold for the frame: the draw
whole on the pool is 490–525 ms, and it is the same on 12 threads as on 24.

**Picked.** Three things. The inflate is pipelined into ADR 1873's build, with at least four pieces
a thread and cuts guessed by bytes (4.6 GB/s) and confirmed by the piece before (`doc/todo/69` §7).
The inflate is not divided, which would win at most 42–45 ms. The next build is the drawer: first
find what in the pool's draw does not divide, then take the pieces in order (the first is done at
44–55 ms). One thread's `Underway` is 1.17–1.26 s and does not qualify. Projected headless frame:
0.76–0.83 s against 2.61–2.62 s.

**Unfinished.** The hand-over against the join is slot 1's hypothesis and was not re-measured. No
window figure was needed, so the owner's display was not used. The instrument is built against HEAD
and has not been checked against slot 1's `pdf-model` edits.

**Files.** `crates/pdf-model/examples/serial_terms_price.rs` (new), ADR 1888, `doc/todo/69` §7 (new
section; its status line is §6's build to edit), and this record.

**Gates.** These ran in an export of HEAD plus the example, with its own target directory.
`rustfmt --check --edition 2024` exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
--all-targets` exit 0. `cargo nextest run -p pdf-model`: 2 043 passed, 20 skipped, exit 0.
`cargo test -p conformance` exit 0 in the worktree. In the export, five `bounded` tests failed:
their population is git's tracked files, and an export has no git.
There was one `--clock --tree 12` hold, queued 100 s, held 177 s, exit 0, load 3.2–7.1: two passes
of `serial_terms_price` (unpinned 12/48 pieces, pinned 8, 48 on 20 threads) and `pipeline_price` on
12 and 24 threads. In every count the joined list was the page's. Duration 3 300 s.
