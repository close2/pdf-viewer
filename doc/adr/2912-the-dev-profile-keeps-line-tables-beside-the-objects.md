# 2912 — The dev profile keeps its line tables beside the objects

Session 2048. Status: **accepted, built.** It cites no clause and moves no ledger row.
Context: `Cargo.toml` `[profile.dev]`, ADR 0222 (`debug = "line-tables-only"`, `opt-level = 1`),
`doc/reviews/2048-the-build-and-the-agents.md` (the review this came out of).

## 1. Where a build directory's bytes are

A cold `cargo test --workspace --no-run` under the pinned 1.97.1, `CARGO_INCREMENTAL=0`, in a private
target directory, `du` by kind afterwards:

| | MiB | files |
|---|---|---|
| the whole directory | 75 553 | |
| `debug/deps` | 57 185 | |
| `debug/examples` | 17 865 | 240 examples |
| executables in `deps` (test harnesses and bins) | 53 255 | 825 |
| `.rlib` | 1 870 | 522 |
| `.rmeta` | 716 | 520 |

So the libraries are 3 % of the directory and the linked programs are 94 %. `readelf -S` summed over
every executable says what is inside them: `.debug_info`, `.debug_str`, `.debug_line`, `.debug_ranges`
and `.debug_aranges` are about 78 % of their bytes and `.text` about 8 %. Line tables are small for one
crate; they are large here because every test harness links the same dependency graph and the linker
copies that graph's tables into each of 825 + 240 programs.

## 2. The settings tried, and what each costs a backtrace

Each setting was checked first on a throwaway crate with this profile and a test that panics through a
`#[inline(never)]` function, `RUST_BACKTRACE=1`:

| setting | backtrace frame names file and line | where it is set |
|---|---|---|
| as before | yes | |
| `split-debuginfo = "unpacked"` | yes | `[profile.dev]` |
| mold `--compress-debug-sections=zlib` | yes | a link argument |
| mold `--compress-debug-sections=zstd` | **no** — the standard library's symboliser reads no zstd | a link argument |
| `debug = 0` | **no** | `[profile.dev]` |

Then cold `cargo test --no-run -p pdf-model` (239 integration-test binaries), one sample each, under the
post-merge train's load (load average 34 to 175, so the walls are noisy and the sizes are not):

| setting | directory | executables | wall |
|---|---|---|---|
| as before | 30 205 MiB | 20 416 MiB | 211 s |
| zlib | 12 611 MiB | 8 537 MiB | 238 s |
| zlib and unpacked | 8 872 MiB | 5 922 MiB | 195 s |
| `debug = 0` (the floor, refused above) | 6 313 MiB | 4 336 MiB | 150 s |

## 3. The decision

`split-debuginfo = "unpacked"` in `[profile.dev]`. It is the one setting of the three that keep a
backtrace's file and line which lives in the manifest: compression is a link argument, and link
arguments are the machine's (the user-level Cargo configuration, where `mold` is named), so the review proposes it
there rather than this tree carrying a linker flag CI does not use.

Measured on the whole workspace, cold, the same command as section 1:

| | before | after |
|---|---|---|
| `debug/` | 75 553 MiB | 45 149 MiB (of which `.dwo` 426 MiB) |
| `debug/examples` | 17 865 MiB | 9 646 MiB |
| processor time (the build's scope, `cpu.stat`) | 16 043 s | 12 974 s |
| peak memory of the build's scope | 44 104 MiB | 51 269 MiB |
| wall | 1 049 s | 486 s |

The wall and the peak are not comparisons: the "before" build ran beside the round's own warm-up build of
the same workspace, so it had half the processors and its links overlapped less. The processor time is
the comparison that load does not move much: the linker writes about 30 GiB less.

## 4. What it costs

- A test binary copied out of `target/debug/deps` loses its line tables, because the skeleton unit
  names the `.dwo` by its path. No test copies one: `grep` finds `fs::copy` beside `current_exe` only in `launch_path`, which copies a document. `viewer-ffi`'s C programs are
  compiled, not copied.
- The standard library's own tables are precompiled into it and still travel in every executable, so
  this halves rather than removes the duplication; the link-argument compression of the review is
  what takes the rest.
- Every artefact's hash changes once, so the first build after this commit is cold. Every round's
  build already is (the build directory is pruned at each round's close).
- `release`, `gates`, `bench` and `fuzz` do not inherit `dev`, and are unchanged; so is every launch
  number.
