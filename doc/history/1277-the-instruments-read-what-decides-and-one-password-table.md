# 1277 — The instruments read what decides, and every corpus gate reads one password table

Batch forty-two, instruments slot. No ledger row moved: the contract named none. ADRs 1391, 1392.

## Built
- **`tools/batch.sh check`** reads `population`, as `commit` does; a quoted path (space, CJK) is
  placed by its real directory. `batch.rs` holds it (fails against the old script).
- **`--bin pointers`** reads the tree's `.gitignore` files (`gitignore.rs`, a subset matcher):
  `Reach::Ignored` prints the pattern and counts by it; ignored directories are not walked.
  `NOT_CARRIED` 13 → 6 (submodules, `doc/adr_revisit`, `scratchpad`, each with its reason).
  Not carried 1159 → 602 by hand + 966 by pattern; absent 251 → 250.
- **`tools/state.sh instruments`**: named 63 / not named 134 → 196 / 0. 133 catalogued in
  `doc/verify.md` (usage line + header's first sentence); one deleted,
  `render-raster/examples/shading_probe.rs` — a scratch probe with a hard-coded path into
  another project, cited only by record 457.
- **Passwords**: the eight private tables (five `pdf-transform` walks, two `pdf-vfs`,
  `save_round_trip.rs`) and `pdf-syntax/tests/encryption.rs`'s cases read
  `crates/pdf-model/tests/support/corpus_passwords.rs` through `#[path]`. All nine lacked
  `issue21579.pdf`, which now joins each walk. `save_round_trip` examined it: out of
  `REFUSED_OPEN`, into `POLICY_REFUSED` and `NOTHING_TO_SAVE_ON` (`/P -1084` clears Table 22
  bit 6), `Restrict(Off)`'s saved floor 8 → 9 (three readers agree). `encryption.rs` checks nine.
- **Zero-test gate lines (ADR 1392)**: `doc/todo/02`'s `-p pdf-transform --test gate` had no
  `--ignored` (its one test is ignored), and `headless_gpu` had no command (none ignored; run with
  `--ignored` it runs zero). Fixed; `batch.rs` now holds every gate line's flag to its file, and
  `gates()` fails a line that ran nothing. Before: 1 found by the check; after: 0.

## Documents
crate-map (render-cpu's per-shape floor, ADR 1374; untagged widgets, ADRs 1369/1381;
`batch.rs`, `gitignore.rs`), HANDOVER (`check`'s population, a row for the password table),
PLAN (the oracle opens published-password pages, ADR 1377), state-of-play (the floor per shape;
raster's tight-bend tiling, ADR 1375); `doc/todo/01`'s eighth sweep names the pattern rung.
`doc/todo/65` re-read against the 14 + 4 rows: nothing false outside siblings' bullets.

## Gates
rustfmt 0; `bash -n` 0; clippy clean on my files (siblings' `pdf-model` lints, read by file);
`cargo test -p conformance` 0. Behind the lock, all 0: `save_round_trip` (after the examination
above), `writer`/`split`/`merge`/`pages`/`optimize_corpus`, `write_corpus`/`read_corpus`.

## Left
`doc/questions/A170` in `doc/state-of-play.md` is absent here (owner's file, not in this worktree).
