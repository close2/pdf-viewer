# 1919 — The clip test, alone under RADV's hang mode, does not hang the card

Session 1919, a single round (pixels + instruments) on the server VM. ADR 2668, Q777, trap 318; ADR
2669 unused. Worked in the rounds worktree and its `scratchpad/r1919/`.

**Why.** A776 allowed one supervised `RADV_DEBUG=hang` run of `clip_that_cuts_nothing::
a_clip_that_contains_a_mark_takes_nothing_from_it` on the RX 580, for the dump of 14:02:33 (ADR 2665).

**The run (ADR 2668 §1).** `tools/hang-capture.sh` writes A776's conditions as one script: the binary
built first, outside the lock; `/run/lock/gpu.lock` held across the rest (an fd-held `flock` for the
run; since, the script re-runs itself under `with-gpu-lock`, as ADR 1662's gate asks); the test on RADV
under a 60 s wall with its own hook's lock pointed at a private file and that file's hang marker
cleared; `journalctl -k` read, and on a timeout a wait inside the lock for `GPU reset(n) succeeded`;
then `vulkaninfo --summary` on RADV to exit 0, ten minutes at most; then the release. Lock held
23:56:55.921 to 23:56:56.946 UTC, 1.0 s. Test exit 0, `1 passed`, both frames drawn on the
`DiscreteGpu` (176 ms and 64 ms, the 2× one being the frame that hung at 14:02). Kernel: no entry.
Probe exit 0. No reset; no dump, since RADV writes one only on a hang.

**What it says (§2).** RADV printed `Enabled debug options: syncshaders, hang`: the hang mode serialises
every draw, dispatch and submission. The run differed from 14:02 in two ways, alone and serialised, and
a pass cannot say which removed the hang (trap 318). Checked in the source and set aside: a timestamp
slot resolved unwritten, the one mechanism that would stall the command processor with no wave to kill —
`wgpu-core` 30 clears a never-written slot rather than copying it, and a slot written by an earlier frame
stays available.

**What stands (§3).** Every guard of ADR 2665. No cause, so no fix, so the other two suspects
(`retained_frame::a_zoom_step_misses`, the `sub_pixel_coverage` tests) stay withheld and were not run.
The upstream report stays drafted, unfiled: it promises a dump. Q777 asks for (a) the same test alone
without `RADV_DEBUG`, then (b) the three files in one parallel `nextest` inside the script's lock.

**Gates.** No crate touched, so no tier 1. `nextest` of `raster-gpu` and `render-raster` on lavapipe
(`WGPU_BACKEND=vulkan`, lavapipe ICD): 904 passed, 8 skipped, exit 0; its lock log shows 933 bring-ups
of `type=Cpu`, none elsewhere, and no hold. The one RADV run as above, exit 0. `cargo test -p
conformance` after `git add -N`: exit 101 twice (`bounded`, the bare `flock`; `evidence`, a path
outside the tree), then exit 0. The wrapped script's dry run held the lock 0.2 s. No amdgpu reset.

**Unfinished.** The hang's cause, and with it the fix, the other suspects' runs and the upstream report
(Q777). `tools/hang-capture.sh` has no mode for a command other than one test binary, nor for a run
without `RADV_DEBUG=hang`; each is made if Q777 allows the run that needs it.
