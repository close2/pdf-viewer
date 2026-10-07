# 1411 — An idle launch waits on a power-up the node's open starts, and the lock names its holder

Measure slot of batch sixty-four. ADRs 1658 and 1659; no question written.

**Bring-up's idle excursion** (ADR 1658). `strace -T` and an `ioctl` shim on the gate's bring-up
child, 20 interleaved pairs: the excess is the first `AMDGPU_INFO_DEV_INFO`, 4.96 against 0.46 ms.
C probes located the cause. It is not the processor: `VRAM_GTT` reads 0.007 ms after idle, and a
spinning SMT sibling changes nothing. A one-message sensor query pays the same 5 ms. On one
descriptor, later queries are cheap at every interval, and waiting 5 ms after the open removes it.
So after about 50 ms of GPU idle, opening the render node starts about 5 ms of power-up, and a
firmware query waits for it. A query loop does not hold the state; an open loop does, but keeps
the GPU powered, which the owner's display would pay for. No launcher setting holds it. Built:
`raster-gpu` opens the awake render nodes before `Instance::new` and closes them after the adapter
check. Closing them at once waited for the power-up and gave back half. Idle-born, 22 an arm:
bring-up 26.32 → 22.59 ms, adapter check 9.83 → 6.19. Back to back, 52 an arm: 19.99 → 20.69,
check 6.04 → 6.01. The step itself costs 0.08 to 0.19 ms, and the bands are unchanged.
**The lock** (ADR 1659). Batch sixty-three's log is round 1405's alone: 4 508.4 s queued, 1 762.3
of it behind the open's arms export and 2 746.1 behind bare-`flock` holders. The records add about
2 640 s for 1402 and 261 for 1406. A second lane under `--tree 6` is memory-safe on the history
(about 47 GB of 61.9), but clock runs would wait for both lanes, and all of 1405's queue was clock
runs. Declined, to be asked again at 3 600 s a batch of eligible queue. The line gains `peak=` and
`behind=`. Mid-batch an `sccache` server started inside round 1412's hold kept the lock's
descriptor for about eighteen minutes after that wrapper ended. Closing the descriptor for the
command fixed that, but it broke ADR 1662's `arms-held` check, so it was taken out (owed, below).

**Premise.** Held: the adapter check carries the excess. Not held: "batch sixty-three's rounds
append" — one did, and `lock_cost` shows 2 586.5 s of its 4 508.4 (two `batch=HEAD` lines).

**Gates.** `bash -n tools/bounded.sh`: 0. `tools/bounded.sh --self-test`: 0, seven cases, both
holder checks failing with their defects planted. `rustfmt --check` on `startup.rs` and
`launch_path.rs`: 0. `RUSTFLAGS="-D warnings" cargo clippy --all-targets` on `raster-gpu` and
`viewer-ui`: 0. `cargo nextest run`: `viewer-ui` 0, 153 passed; `raster-gpu` 0, 705 passed,
the new test failing with `suspended` admitted. `cargo test -p conformance`: 0, 400
passed. Behind the lock, `pdf-sandbox --bins` first: `launch_path --release` counted exit 101, 1
of 31 outside; with clocks exit 101, 3 of 53 outside, `bring_up_ms` 16.2 inside. Outside: EC3's
memory (79.0 MiB) and first page (81.8 ms), `bug1815476`'s cold open (0.551 ms); none is the device.

**Unfinished.** EC3's viewer open takes about 50 ms in the shared tree from 19:50, against a
`pdf-syntax` cold open of 5.6 ms. That is a sibling's change and was not traced. The lock fix is
owed jointly to `bounded.sh` and `batch.sh` (ADR 1659 section 4).
