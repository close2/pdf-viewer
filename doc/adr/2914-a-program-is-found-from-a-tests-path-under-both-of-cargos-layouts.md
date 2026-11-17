# 2914 — A program is found from a test's path under both of Cargo's layouts

Session 2049. Status: **accepted.** It cites no clause and moves no ledger row. Context: ADR 2910 (the
pin to 1.100 waits for this), trap 10 (a worker is another package's binary).

## 1. What changed in Cargo, read on a probe crate

Under Cargo 1.100.0, a crate with a library, a program, an example, a build script and a test builds
every unit into `<profile>/build/<package>/<hash>/out/`: the test as `out/it-<hash>`, the build script as
`out/build_script_build`, and the program and example as `out/prog` and `out/ex`, which Cargo then copies
to `<profile>/prog` and `<profile>/examples/ex` as before. No `<profile>/deps/` is written at all. Up to
1.99 a test sat at `<profile>/deps/<test>-<hash>`. So a program is where it always was, and a test is
four directories further from it.

## 2. The rule, stated once

`pdf_sandbox::beside` holds it, because `pdf-sandbox` is the crate every shipped worker lookup already
depends on (`confined-transport`, `viewer-confined`, `pdf-vfs`, `pdf-script-worker`'s client and
`quorra-transform` reach it through `confined_transport::program_beside_executable`):

- **`cargo_profile_directory(executable)`** reads the profile directory from the path's shape alone:
  `<profile>/deps/<file>`, `<profile>/examples/<file>`, or `<profile>/build/<package>/<hash>/out/<file>`.
  Any other path is refused by `NotInCargoProfile`, which names it. A program directly in `<profile>/`
  is deliberately not read as one, or `/usr/bin/quorra` would name `/usr/bin` a profile directory.
- **`program(executable, file_name)`** searches beside the executable, one directory up, and then the
  profile directory where the path names one.

What Cargo states outright is used wherever it reaches: a test of the package that owns a program
reads `env!("CARGO_BIN_EXE_<name>")` (`pdf-transform`'s, `pdf-script-worker`'s and the toolkits' tests
already do), and scratch files go under `env!("CARGO_TARGET_TMPDIR")`. Neither reaches a library that
another package's test links, nor a shipped program, and those are the lookups the rule serves.

## 3. Each lookup, shipped or test-only

| lookup | shipped? | now |
|---|---|---|
| `pdf_sandbox::worker_program` | shipped (the viewer's image worker) | `beside::program` after `PDF_SANDBOX_WORKER` |
| `confined_transport::program_beside_executable` (`viewer-confined`, `pdf-vfs`, `pdf-script-worker`, `quorra-transform`) | shipped | the same |
| `pdfref`'s `Reference::Hayro::program_path` | test and tool | the same, after `PDFREF_HAYRO` |
| `cargo_artifacts.rs`'s profile (`viewer-ffi`, `pdf-vfs-ffi` C-program tests) | test-only | `cargo_profile_directory`, then its own check that the profile sits directly under the target directory |
| `launch_path`'s profile, page-cache directory and script-worker check | test-only | `cargo_profile_directory` / `beside::program` |
| `turn_path`'s profile | test-only | `cargo_profile_directory` |
| `tools/batch.sh export-sums` | merge tooling | also sums `<profile>/build/<package>/<16 hex>/out/<name>-<16 hex>`, which is a test binary and never a build script or a program's pre-copy, neither of which carries the hash |

**A shipped program's search does not change outside a Cargo build directory.** Its first two places
are the old ones; the third is new only for an executable inside `<profile>/build/<package>/<hash>/out/`,
a directory Cargo writes and no installation does, and for `deps/` and `examples/` it is the second
place again and is not searched twice. The other `current_exe` calls in the tree re-run the test
itself as a child, or name it in a log line, and read no directory from it.

## 4. Proof

`beside`'s own tests plant both layouts and the refusals: a test under `build/<package>/<hash>/out/`
with no `deps/` on its path finds a program put in the profile directory and finds nothing before it is
put there; `/usr/bin/quorra`, a bare name, the empty path and a `build/` path one directory short are
refused by name, with no panic. The whole workspace's nextest ran under 1.97.1 and under 1.100.0 (the
round's record has the counts).
