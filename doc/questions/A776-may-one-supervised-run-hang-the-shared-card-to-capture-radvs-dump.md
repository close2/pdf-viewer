Status: complete
Given: 2026-11-03, in conversation — transcribed by the round
Owes: run clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it (--exact) on the RX 580 with RADV_DEBUG=hang under the shared GPU lock (/run/lock/gpu.lock), holding the lock until any kernel GPU reset has completed and the card is ready again before releasing the lock, and capture the RADV hang dump or concurrency finding for ADR 2665 (a raster / hardware round)

> Q776: as long as you lock the GPU this shouldn't be a problem.  Just lock the GPU, make your test, and only release the lock when the GPU has reset.  (Unless I am not understanding the problem correctly)

Reading: Option 1 is allowed under the shared GPU lock (`/run/lock/gpu.lock`): acquire the lock before running `clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it` (`--exact`) with `RADV_DEBUG=hang`, and if the card hangs, wait inside the lock until the kernel's `amdgpu` reset completes (`journalctl -k`) and the card is healthy again before releasing the lock (since the other user's harness creates and destroys its Vulkan device per test child inside the same lock, no peer context exists while the lock is held).
