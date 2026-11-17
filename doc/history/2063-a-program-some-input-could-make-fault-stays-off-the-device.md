# 2063 — A program some input could make fault stays off the device

Session 2063, a pixels round in the gwg worktree on the RX 580's return. Rows §7.10.5.3 and §8.7.4.5.2; ADR 2940;
no question.

**The slice.** The brief's two items: ADR 2884's device figures, then ADR 2849's two remedies priced and the cheaper
built.

**The premise.** The first item did not hold: round 2043 took ADR 2884's figures on the RX 580 (ADR 2902,
`dotted_zoom` and `marks_across_views` on `DiscreteGpu`, the `render-raster` corpus with `issue6931_reduced`
agreeing, `turn_path` 33 of 33), so no second ADR was written and 2941 is unused. The second held — nothing after ADR
2849 priced either remedy — narrowed: `raster-gpu`'s `admit` already refuses unbalanced arms, computed counts, a
wrong output count and a stack past 64 slots, so only a non-number output could reach a device unrecorded.

**Priced.** Remedy 2: the census over 67 447 paths finds 33 type 1 shadings, 24 with one type 4 function, and sends
none to the processor (a planted faulting shading is named). Remedy 1: `fault_word_price` on `DiscreteGpu`, a
read-back frame of `function_based_shading.pdf`'s radial program 4.39 ms, with the word mapped after it 5.35 ms.

**Built.** `pdf-colour`: `Function::may_fault` over `faults_excluded`, a forward walk that keeps at each join what
both arms agree on, and `device_program` hands over only a program it clears (§7.10.5.3, §8.7.4.5.2). `pdf-model`'s
census counts the population; `raster-gpu`'s example prices the map and is in CI's `--check` list.

**Tests.** `a_program_some_input_makes_fault_is_not_handed_to_a_device`. **Plant**: the condition dropped from
`device_program` fails it. No `raster_golden` page can move: the processor's path is unchanged and no corpus shading
changes lane.

**Gates.** Over `pdf-colour`, `pdf-model`, `raster-gpu`, `render-raster`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0; nextest under `xvfb-run -a` pinned to the RX 580, 3 353 of 3 771 passed and 418 failed, every
one a software-adapter test finding no llvmpipe under the pin; those 785 re-run unpinned, 785 passed (293 bring-ups on
`DiscreteGpu`). rustdoc `-D warnings --document-private-items` exit 0. `turn_path` (release, `--clock`,
`DiscreteGpu`): 33 of 33 inside their bands. Conformance after `git add -N`: exit 0. No public type changed, so no
fuzz check. Tier 2 is the train's.

**Left of the rows.** §7.10.5.3 and §8.7.4.5.2 stay `implemented`; a host that draws the grid and then asks it is
still owed (ADR 2848).
