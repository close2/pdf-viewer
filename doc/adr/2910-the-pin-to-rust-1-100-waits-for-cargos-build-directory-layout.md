# 2910 — The pin to Rust 1.100 waits for Cargo's build-directory layout

Session 2047. Status: **accepted; the bump is held, not built.** It cites no clause and moves no ledger row.
Context: `rust-toolchain.toml` (the pin, 1.97.1), Q875, ADR 1024 (`clippy.toml`). The owner installed Rust
1.100.0 beside the pin on 2026-11-16, and that installation is the review the pin's header asks for.

## 1. What the bump needs, measured

The bump was made in full in round 2047's worktree and then taken back out. It sets the pin to `1.100.0`,
`rust-version` and `clippy.toml`'s `msrv` to `1.100`, and two comments that name the pin. Under the new
compiler, `cargo fmt --check` passes, rustdoc with `-D warnings` over the workspace passes, and
`cd fuzz && cargo +nightly check --bins` passes (nightly `1.101.0-nightly` of 2026-11-02, which the fuzz
workspace does not pin). Clippy found the following:

| lint | what it asks | rewritten as |
|---|---|---|
| `chunks_exact_to_as_chunks` (default set, new) | a constant-length `chunks_exact(N)` | `as_chunks::<N>()`, whose chunks are arrays; a `.remainder()` becomes the pair's second half, and slice patterns that could not fail lose their dead `_` arms and `else` clauses |
| `assert_is_empty` (pedantic, new) | `assert!(v.is_empty())` | `assert_eq!(v, [] as [T; 0])`, so a failure prints what was there; one place where `T` is private keeps `assert!` with the value as its message |
| `map_unwrap_or` (pedantic, widened) | `.map(f).unwrap_or_default()` | `Option::map_or_default`, new in 1.100's `msrv` |
| `manual_bit_width` (new for this `msrv`) | `u32::BITS - x.leading_zeros()` | `x.bit_width()` |
| `manual_midpoint` (extended) | `(a + b) * 0.5` | `f32::midpoint(a, b)`, which is the same value for every pair that does not overflow |
| `float_cmp` (narrowed) | no longer fires inside `assert_eq!` | each `#[expect(clippy::float_cmp)]` above such an assertion is removed, with its reason shortened to the lints that remain |
| `needless_range_loop` (widened) | `for i in 0..n` that indexes | `iter().enumerate()` |
| Cargo `unused_workspace_dependencies`, `unused_dependencies` (new) | six workspace entries no member names, and `pdf-model`'s `pdf-spec` that only an example reads | entries removed; `pdf-spec` becomes a dev-dependency |

Most of the rewrites are clippy's own machine-applicable suggestions. About forty compile errors followed,
where a chunk that had been a slice became an array: a `*` where it meets an array, a qualified path that
clippy's suggestion wrote relative to the module, and two `filter_map(|x| <[T; N]>::try_from(x).ok())`
calls that became `to_vec()`. 27 test files that the owner's audit commits added need the same lint-only
edits. They are held as a patch outside the tree, behind Q875.

## 2. What stops it: test executables are no longer in `deps/`

Cargo 1.100 places a test executable at `target/<profile>/build/<package>/<hash>/out/<test>-<hash>`
instead of `target/<profile>/deps/`. Programs stay in `target/<profile>/`. Every lookup here that
finds a program from a test's own path assumes the old place:

- `pdf_sandbox`'s `worker_program`, `viewer-confined`'s and `confined-transport`'s workers, and
  `pdf-vfs`'s server search beside the executable and one directory up;
- `tools/conformance/test-support/cargo_artifacts.rs` refuses a test that is "not in a Cargo deps
  directory", and `viewer-ffi`'s and `pdf-vfs-ffi`'s C-program tests rely on it;
- `viewer-ui`'s `launch_path` works out its profile from the directory it sits in, and its page-cache copies
  go "two levels up";
- `tools/pdfref`'s reference programs, and `tools/batch.sh export-sums`, which hashes the executables it
  finds under `<profile>/deps/`.

Under 1.100, `cargo nextest run --workspace` ran 9970 tests: 9862 passed, 108 failed, 73 were skipped.
Every failure is one of these lookups: `WorkerMissing`, "was not found next to", or "not in a Cargo deps
directory". The opt-out Cargo carries, `__CARGO_TEMPORARY_BUILD_DIR_NEW_LAYOUT_OPT_OUT`, is a private,
temporary variable and is not a remedy.

## 3. The decision

**The pin stays at 1.97.1 until the lookups find a program wherever Cargo puts the test.** That repair
reaches production code (`worker_program` runs in the shipped viewer), the test-support crate and the
merge's own tooling. A toolchain round should not carry it as a lint fix. It is a slot of its own, and it
can land before the bump, because a lookup that searches the executable's ancestors up to the profile
directory finds the program under both layouts. Two things remain once it lands: the lint patch, written against `0dd1127c` and rebased if needed, and
the answer to Q875. Round 2047 worked in `~/scratch/agent/r2047/`, and both patches are there:
`toolchain-1.100.patch` and `audit-tests-lint.patch`.

## 4. The launch path under each compiler

`viewer-ui`'s `launch_path` was run three times, with `PDFVIEWER_LAUNCH_CLOCKS=1` and in a clock hold: at
`0dd1127c` under 1.97.1, at `0dd1127c` under 1.100.0 (`RUSTUP_TOOLCHAIN`), and with the whole bump applied.
Both 1.100 runs had the workers named by `PDF_SANDBOX_WORKER` and `PDF_SCRIPT_WORKER`, because their own
lookup fails (section 2). They print every figure but judge no clock, because the profile is read from
the test's directory, and that directory is now a hash.

| figure | 1.97.1 | 1.100.0, same source | 1.100.0, bump applied |
|---|---|---|---|
| cold graphics bring-up | 51.7 ms | 51.9 ms | 55.9 ms |
| `open_kinstructions`, `Well-Tagged-PDF-WTPDF-1.0.pdf` | 9547.5 k | 10096.7 k | 10096.8 k |
| `open_kinstructions`, `ISO_32000-2_sponsored_EC3.pdf` | 53271.8 k | 55618.2 k | 55618.1 k |
| `open_kinstructions`, `PDF20_AN001-BPC.pdf` | 2192.3 k | 2206.0 k | 2206.0 k |
| first page, `PDF20_AN001-BPC.pdf` | 86.0 ms | 85.2 ms | 87.4 ms |
| first page from a cold page cache, same | 85.6 ms | 85.4 ms | 89.5 ms |
| page turn, `ISO_32000-2_sponsored_EC3.pdf` | 13.3 ms | 11.6 ms | 12.2 ms |

The counted figures are the ones a machine cannot move. **The new compiler makes an open 4.4 to 5.8% more
expensive on the two larger documents**, and the lint rewrites add nothing to that: the two 1.100 columns
agree to the thousand instructions. Both larger documents fall outside their `open_kinstructions` bands
(`9463 .. 9656` and `52862 .. 53931`). The clock figures move within their run-to-run spread. The tree at
`0dd1127c` already lies outside two bands under 1.97.1 (`bug1815476.pdf` and `opt_demo.pdf`, whose opens
execute fewer instructions than their floors). Those are the tree's own and not the compiler's.

So the round that lands the bump owes one thing more: find where 1.100 spends the extra instructions in an
open (`callgrind` over `Document::open` under each compiler) before it moves the two bands. A code
generation change still costs time to the first page, and principle 2 counts that cost wherever it comes
from.
