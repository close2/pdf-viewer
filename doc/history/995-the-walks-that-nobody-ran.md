# 995 — The walks that nobody ran, and the population that is now checked

Date: 2026-09-12. ADR: 1015. Worktree `batch-992-997`, branched from `main` at `1ea10797`, with
siblings 992, 993, 994, 996 and 997 editing beside it.

Files: `doc/todo/02-every-round.md` (§2: six gate lines, two bullets, the `pdf-archive` map row,
the `pdf-transform` and hosts rows), `tools/state.sh` (`archive`, `confined`, `actions`,
`on-disk`), `tools/conformance/tests/state_sections.rs` (a second test), `tools/conformance/
tests/questions.rs` (the owed `rustfmt` hunk and `comes_after`), `doc/state-of-play.md` (the
validator's claim), `doc/verify.md` (`awkward_classes` handed to §2), one `// no sandbox worker:`
line each in `crates/pdf-archive/tests/corpus.rs`, `crates/pdf-transform/tests/archive_corpus.rs`,
`crates/pdf-model/tests/actions.rs`, `crates/pdf-syntax/tests/on_disk.rs` and
`crates/viewer-confined/tests/awkward_classes.rs`, `doc/adr/1015`.

The direction review's first next step. Six `#[ignore]`d test files were in no gate line — the
review said seven, and the seventh was a doc comment about an attribute ADR 0888 had removed —
and each was run alone under `tools/bounded.sh`, measured, and given a line. None is excused. The
population is a checked claim now: `state_sections.rs`'s second test reads the index for every
`.rs` under `crates/` and `tools/` with the attribute at the start of a line and fails by name on
a test file §2 does not name unless it carries `// not a gate:` with a reason, and fails the
other way on an excuse the sequence has overtaken. Calibrated four ways with a planted file
(untracked: printed; tracked: failed by name; excused: passed and printed; a spent excuse in
`on_disk.rs`: failed as spent) — and, before any line was added, against the tree itself, where it
named exactly the review's six.

The check prints two things it does not fail on, each with one instance: an ignored unit test in
`crates/pdf-model/src/signature.rs`, which no `--test` line can name and which owes the excuse
line in a file this round did not own; and the sibling's untracked
`crates/pdf-model/tests/raster_golden.rs`, which will need a §2 line and a `state.sh` section at
the merge.

## Costs, alone under `tools/bounded.sh`, siblings building (load 7 to 9)

| line | `bounded.sh` | its own summary |
|---|---|---|
| `pdf-archive --test corpus` | exit 0 after 2 s; peak 0.12 GiB | `over` 0 on all six targets; `missed` 1 on PDF/A-2b, `agreed` 473 / 9 / 17 / 971 / 21 / 27 |
| `pdf-transform --test archive_corpus` | exit 0 after 6 s; peak 0.17 GiB | PDF/A-2b 377 conforming, 377 still conforming, 433 converted, 176 refused; PDF/A-4 128 / 128 / 171 / 188; the others in the section's output |
| `pdf-model --test actions` | exit 0 after 1 s; peak 0.03 GiB | `/O` 3, `/C` 3, `/PO` 1, `/PC` 1, held both ways |
| `pdf-syntax --test on_disk` | exit 0 after 2 s; peak 0.32 GiB | 964 documents agree on 112574 objects; 10 refused both ways the same |
| `viewer-confined --test awkward_classes` | exit 0 after 12 s; peak 4.65 GiB | killed: 0 over ten classes, in 11.5 s |

## Gates

Run detached, in sequence, at a load average that reached 50 while five siblings built in the
same worktree and build directory.

- `cargo fmt --all --check`: **exit 0**, after `cargo fmt -p conformance` formatted the two owed
  hunks in `questions.rs` — and the second of those pushed a test over clippy's `too_many_lines`,
  which is why `comes_after` exists (ADR 1015 §4).
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`: **exit 101**, three lints in
  `crates/pdf-archive/examples/dates.rs` — a sibling's untracked file, mid-edit; none in any file
  this round touched. `cargo clippy -p conformance --all-targets` under the same flag: exit 0.
- `cargo nextest run --workspace`: **exit 100** — 3099 passed, 1 failed, 36 skipped, the run
  cancelled at the failure. The failure is `pdf-vfs-ffi::the_kio_worker`: CMake refused
  `kio/CMakeLists.txt` because the cache in the shared build directory was generated from
  `/home/cl/projects/pdf-viewer/kio` — the main checkout's source, not this worktree's. An
  artefact of one `CARGO_TARGET_DIR` under two checkouts (trap 15's shape), not of anything in
  this round; not repaired.
- `cargo test --workspace --doc`: **exit 101**, `pdf-model` (lib) did not compile —
  `crates/pdf-model/src/content/image.rs:308` names `crate::image::Parts` without its new
  `alpha` field, a sibling mid-edit (the same tree had compiled for `nextest` minutes earlier).
- `cargo fmt --manifest-path fuzz/Cargo.toml --check`: **exit 0**.
- `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`: **exit
  101**, the same `pdf-model` compile error.
- `cargo test -p conformance`: **exit 101** on `conformance.rs`'s
  `every_quotation_is_the_standards_own_words` — `crates/pdf-render/src/shading.rs:200` quotes
  §11.6.4.4 with "painti ng" in it, a sibling's file (117 lines changed against `main`, not this
  round's). Re-run with `--no-fail-fast`: every other binary green — the 234 unit tests,
  `bounded` (1), `questions` (2), `sandbox_gates` (1), `state_sections` (2, one of them new),
  `submodules` (1), `workspaces` (1); `conformance.rs` 5 of 6.
- `tools/state.sh archive actions on-disk confined save`: **exit 0** in 156 s, the sections'
  output as in the table above plus the save round-trip's own lines: 974 documents, 935 saved
  under `Restrict(On)`, 8 under `Off`, every failure list empty.

Four of the eight red lines are one sibling's compile error and one sibling's lint, one is a
sibling's quotation, and one is a build-directory artefact; nothing red names a file this round
wrote. Whoever merges owns §2 for the merged result, as the section says.
