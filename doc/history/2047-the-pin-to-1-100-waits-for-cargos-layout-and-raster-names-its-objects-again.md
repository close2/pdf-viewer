# 2047 — The pin to 1.100 waits for Cargo's layout, and raster names its objects again

Session 2047, a raster round in the audit worktree after the owner's system update (loader 1.4.363, Rust
1.100.0). No ledger row; ADRs 2910 and 2911; Q875.

**The slice.** The brief's three items in its order: the toolchain pin, Q874 on the new loader, the Mesa
check with the raster gates.

**The premise.** The pin and the loader held: 1.100.0 and `libvulkan.so.1.4.363` are installed, and the
loader's source takes `loader_lock` in `loader_get_icd_and_device`. The bump's premise did not hold: lint
fixes alone do not make the workspace clean. Cargo 1.100 puts test executables in
`build/<package>/<hash>/out/`, and every program lookup that starts from a test's path fails there (108 of
9970 tests). 27 owner audit files need lint-only edits (Q875). Mesa did not change (25.0.7).

**Built, then held.** The whole bump: the pin, `rust-version`, `msrv`, and fixes for clippy 1.100's
lints (`chunks_exact_to_as_chunks`, `assert_is_empty`, the widened `map_unwrap_or`, `manual_bit_width`,
`manual_midpoint`, `float_cmp`'s unfulfilled expectations) and for Cargo's unused-dependency lints. Under
1.100: fmt, clippy `-D warnings` outside the audit files, rustdoc `-D warnings` and the fuzz check pass. It
was then reversed out of the tree into a patch, because the lookups are a slot of their own (ADR 2910).
The launch gate was run at HEAD under 1.97.1, at HEAD under 1.100, and with the bump applied. Under 1.100
an open costs 4.4 to 5.8% more instructions on the two larger documents, and the lint rewrites add none.

**Built, kept.** `raster-gpu`'s `instance_flags` sets `InstanceFlags::DEBUG` again for debug builds (ADR
2911). On the RX 580, `marks_across_views` passed 450 of 450 runs and `fitted_dashes_across_views` 450 of
450, with no crash and no core, and all 9450 bring-up lines read `DiscreteGpu`. With loader 1.4.309 (ADR
2903) the crash rates were 31 in 150 and 10 in 200. **Plant**: none can reach the race, because that
needs the old loader. Under 1.100 each lookup fails out loud by name.

**Gates.** On the tree as left (1.97.1, `raster-gpu` touched): `cargo fmt --check` 0; clippy `-D warnings
--all-targets` over `raster-gpu` and `render-raster` 0; nextest 912 of 912 (8 skipped), 297 bring-ups on
`DiscreteGpu`; `render-raster` `corpus` 0 (968 agree, 0 differ, 6 not comparable, 499 bring-ups on
`DiscreteGpu`). `turn_path` 101 twice: 2 of 33 outside, then 1 of 33 (`issue13931` step 9.24 ms, under its
floor of 9.74). Its release build is HEAD's, because `DEBUG` is a debug-build flag. `cargo test -p
conformance` 0. `state.sh docs` 0. Under 1.100: workspace nextest 100 (108 failed); the launch gate 101 on
four `open_kinstructions` bands, against three figures at HEAD under 1.97.1. No `pdf-model` tier 2 was
owed, because nothing under it changed.

**Left.** The layout slot comes first, then the patch and A875. The 1.100 instruction cost needs a
`callgrind` reading. Q874 is the owner's to close.
