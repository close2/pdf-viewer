# 2043 — A declined fit is asked again, and raster names nothing to the loader

Session 2043, a ledger round in the audit worktree on the RX 580's return. Row §10.7.5; ADRs 2902 and
2903; no question.

**The slice.** Round 2034's four raster findings, in the brief's order: the hardware re-runs, the
`fitted_for_dashing` decline, the process-wide `SIGSEGV`, the compositor wake.

**The premise.** The decline was a defect: a dashed rectangle under `/SA` kept from a view whose fit
declined drew 168 bytes unlike a fresh build where the fit takes. The `SIGSEGV`'s premise did not hold:
no panic is needed — the Khronos loader (1.4.309) walks its logical-device list under one mutex while
`vkCreateDevice`/`vkDestroyDevice` change it under another, and `vkSetDebugUtilsObjectNameEXT` is that
walk; four `gdb` cores show it. The wake race was settled by round 2018 (ADR 2854) and is struck.

**Built.** `render-raster`: `fitted_for_dashing` keeps a declined fit as `KeptMark::Unfitted`, asked
again by `Retained::draws` (§10.7.5 with §8.5.3.2's dashes). `raster-gpu`: `instance_flags` drops
`InstanceFlags::DEBUG` (debug builds only; `DISCARD_HAL_LABELS` tried first, 8 of 150 still crashed).
`SIGSEGV` on the RX 580: `marks_across_views` 31/150 before, 1/450 after; `fitted_dashes_across_views`
10/200 before, 0/150 after. Lavapipe (both halves) did not crash in 270 runs before.

**Tests.** `fitted_dashes_across_views.rs` (two, byte for byte against a device built at each view).
**Plant**: the record dropped fails both (168 of 172 800 bytes differ).

**Hardware re-runs.** Adapter from `PDF_VIEWER_GPU_LOCK_LOG`'s `type=` lines: the suites and the corpus
walk drew on `DiscreteGpu` (the RX 580, RADV). `render-raster` corpus: 974 documents, 968 agree, 0
differ, 6 not comparable (issue6931_reduced agrees). `dotted_zoom` (load 2.4): kept 6.8–13.6 ms a step,
rebuilt 28.7–30.2 ms — the lavapipe figures in `StatedMarks`' comment replaced, ADR 2902 records them.
`render-raster` `turn_path` (release, `--clock`, load 4.5 at the child, 21 at the queue): 33 of 33
inside their bands. `viewer-ui` has no `turn_path` target; the brief's bands are this one's.

**Gates.** Over `render-raster` and `raster-gpu`: `cargo fmt --check` exit 0; clippy `-D warnings
--all-targets` exit 0; nextest 912 of 912 passed (8 skipped), 297 bring-ups on `DiscreteGpu`.
`cargo test -p conformance` exit 0. Tier 2: `pdf-model` `corpus` 101 (67 incomplete) and
`raster_golden` 101 (23 moved), neither crate depending on a touched one, so HEAD's;
`render-raster` `corpus` 0. `state.sh docs` 101 on an ambiguous `grid_fit` link of mine, fixed (`fn@`), rerun exit 0. No public type changed, so no fuzz check.

**Left of the row.** §10.7.5 stays `implemented`. The loader's race is upstream's; one crash in 450
runs left no core.
