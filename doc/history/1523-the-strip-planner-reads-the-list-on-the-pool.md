# 1523 — The strip planner reads the list on the pool, and the cut stays one thread's

Slot 2 of batch eighty-three, 2026-10-12, a pixels round on the owner's performance target; the
partial-row word yields to it. ADR 1882; no question. No ledger row moves.

**Premises** held: `plan_strips` was serial (`grep par_` named no line of it) and `MarkBounds`
measured every bound on the pool. **The hypothesis held in half**: one plan of the witness at 5.86×
was 5 964 M instructions, 2 634 M of them `device_bounds` asked twice a command, so the bounds were
`MarkBounds`' work twice. What is left is two walks and a replay, not a cheap scan, so the list is
read on the pool and the cut decided serially, as the brief's second branch says.

**Built.** `pdf_render::strip_inputs`, `StripInputs`: runs of commands read on the pool, each
command's bound taken from `MarkBounds` where it was measured at the target's own transform
(`MarkBounds::exactly_at`). A row's cost is summed in the list's order, band by band. The forbidden
rows, and their union with the freed fills' rows, are unions; the replay is a pair of counts.
`plan_strips` measures the bounds first and reads these. The serial functions stay as reference.

**Figures** (one clock hold, HEAD and this alternated three times, `band_first`, ms). Planner at
the pool's width, 24 threads: 231–351 → **43–48** at 5.86×, 232–236 → **39–43** at 7.33×; 4
threads 206–359 → 55–102, 210–217 → 55–72. The zoomed page drawn whole, 24 threads: 553–660 →
**337–378**, 630–712 → **378–452**; 4 threads 849–1 280 → 611–866, 993–1 048 → 720–1 015. The
planner's median share of the 24-thread draw went from 36–41 % to 10–13 %. Headless only: no
window was driven, so no figure was taken on Xvfb or on the owner's display.

**Unfinished.** The draw after the plan, about 290–410 ms on 24 threads, is the next term
(`doc/todo/69` §7). A window's zoom notch was not re-taken.

**Files.** `crates/pdf-render/src/{strips,reach,lib}.rs`, `crates/render-cpu/src/lib.rs`, ADR 1882,
`doc/state-of-play.md`, this record.

**Gates.** `rustfmt --check --edition 2024` (four files): exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-render -p render-cpu --all-targets`: exit 0. `cargo nextest run -p pdf-render -p
render-cpu`: 482 passed. The new equality test fails under each of four plants. `cargo test -p
conformance`: 434 passed, 1 failed, and the failure is not this round's: `names` points at slot 3's
`raster-gpu/src/encode/instance.rs:250`. From HEAD plus these hunks: `raster_golden` (held 16 s,
exit 0) held 974, moved 0; the division test is the brief's 176 / 1 308 / 3. Arms cpu-1× and cpu-4×
(held 484 s, exit 0): 968 and 965 pages, 0 digests and 0 mean errors differ from
`/home/AI/arms-1522/`; the other four are owed none (ADR 1882 §3). Clock hold (queued behind
round 1522, held 257 s, exit 0): `band_first` 12 runs, 0 pixels differ; `turn_path` 33 judged, 0
outside. Duration 7200 s.
