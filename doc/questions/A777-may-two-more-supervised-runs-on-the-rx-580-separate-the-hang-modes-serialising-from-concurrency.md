Status: complete
Given: 2026-11-04, in conversation — transcribed by the round
Owes: run (a) clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it (--exact) without RADV_DEBUG, and if (a) passes, (b) the 14:02 parallel nextest shape (clip_that_cuts_nothing, retained_frame, and sub_pixel_coverage) on RADV inside tools/hang-capture.sh's GPU lock (/run/lock/gpu.lock held until any kernel GPU reset has completed and vulkaninfo succeeds), with no limit on tries as long as the lock is held across any reset (a raster / hardware round)

> Q777: as long at the lock is taken, no limit on tries.

Reading: Option 1 is allowed (both run (a) alone without `RADV_DEBUG` and, if (a) passes, run (b) reproducing the 14:02 parallel suite shape inside `tools/hang-capture.sh`), and more generally there is no limit on the number of hardware diagnostic runs on the RX 580 as long as `/run/lock/gpu.lock` is held from before the run until any kernel GPU reset has completed and the card is ready again.
