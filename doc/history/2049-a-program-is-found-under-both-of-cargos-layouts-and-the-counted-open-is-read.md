# 2049 — A program is found under both of Cargo's layouts, and the counted open is read

Session 2049, a build round alone in the audit worktree. No ledger row; ADRs 2914, 2915 and 2916.

**The slice.** The brief's three items in order: the lookups, the 1.100 open cost, the launch figures.

**The premise.** It held. A probe crate under Cargo 1.100.0 puts every unit in
`<profile>/build/<package>/<hash>/out/` and writes no `deps/`; programs and examples are still copied to
`<profile>/` and `<profile>/examples/`. No lookup read that shape, and nothing about it needed the owner.

**Item 1 (ADR 2914).** `pdf_sandbox::beside` states the rule once: `cargo_profile_directory` reads
`deps/`, `examples/` and `build/<package>/<hash>/out/` from the path alone and refuses any other path by
name (`NotInCargoProfile`); `program` searches beside the executable, one up, then that directory.
`pdf-sandbox`'s worker lookup, `confined_transport::program_beside_executable` (the shipped workers),
`pdfref`'s `pdfref-hayro`, `cargo_artifacts.rs`, `launch_path`'s profile, cache directory and script-worker
check, and `turn_path`'s profile use it; `batch.sh export-sums` sums the new layout's test binaries. A
shipped program's search is unchanged outside a Cargo build directory. **Plant**: `beside`'s tests put a
test under `build/pdf-model/<hash>/out/` with no `deps/` and find the worker only once it is in the
profile directory; `/usr/bin/quorra`, a bare name, the empty path and a `build/` path a level short are
refused by name, no panic; `cargo_artifacts_tests` and `batch.rs`'s export-sums test carry the new shape.

**Item 2 (ADR 2915).** `callgrind` over the counted child, same source, 1.97.1 (LLVM 22.1.6) against
1.100.0 (LLVM 23.1.5): zlib-rs 0.6.6's `inflate_fast_help_avx2` runs ~30% more instructions (489 of 549
thousand added on WTPDF, 1667 of 2346 on EC3), and EC3's xref-entry closure 6% more. No band moved.

**Item 3 (ADR 2916).** The two counted floors fell at `79d0c6c4`: `/PV` is raised when a page enters a
viewport, and the counted child has none (`raise` −96.6 and −42.9 thousand, read against `77514286`'s
build). Restated by the file's rule: `bug1815476` `1726 .. 1762`, `opt_demo` `392 .. 401`.
`bug1815476`'s warm open read 1.85 ms, inside; `PDF20_AN001-BPC`'s read 2.392 against 2.390 with its
counted figures inside, which does not move a band.

**Gates.** 1.97.1: workspace nextest under `xvfb-run -a` 0 (9974 of 9974, 73 skipped); fmt 0; clippy
`-D warnings --all-targets` over the ten touched crates 0; `cd fuzz && cargo +nightly check --bins` 0.
1.100.0, private build directory: workspace nextest 0 (9974 of 9974, 73 skipped; 2047's 108 failures
gone). `launch_path` with clocks 101 twice, every counted figure inside: at loads of 1.9 to 6.9 the one
outside named above; beside a train walk (load to 14.8) four other clocks, none outside in both runs.
`cargo test -p conformance` 0 (474 passed) after `git add -N`; rustdoc `-D warnings
--document-private-items` of the ten crates 0.

**Left.** The pin, the lint patch and Q875 are the bump's, and it re-takes the counted bands (ADR 2915).
