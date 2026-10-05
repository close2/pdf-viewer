# 1537 — A turn is judged only on a quiet device, read off the driver's own counter

Status: accepted. Session 1351. Extends ADR 1513's admission rules for `turn_path`; answers the
second half of trap 101. Builds on ADR 0916 (the load ceiling) and ADR 1519.
Code: `crates/render-raster/tests/turn_path.rs` (`device_busy`, `keep_the_device_busy`, `take`,
`a_planted_busy_device_is_seen_as_busy`); `doc/checks/turn-path.toml`'s `device_busy_percent`.

## 1. What the gate could not see

`turn_path` admits a child below a load average of one per physical core and inside the launch
gate's calibration band. Both are the processor's. A neighbour drawing on the graphics device adds
nothing to the load average while it queues ahead of every submission a child makes, and round
1342 watched step rows fail by 2.5× (`issue19802.pdf`) at a load the ceiling admitted: a false
failure, which a gate may no more give than a silent pass.

## 2. Where the figure comes from, and why not the alternatives

- **The driver's own counter**, `/sys/class/drm/card*/device/gpu_busy_percent` on amdgpu: the share
  of a recent interval the graphics engine was working, for every process on the device. It is read
  from outside the measurement — over half a second (twenty readings, 25 ms apart) before a child
  starts and again after it exits — so the figure is the neighbours' and the compositor's, with at
  most the tail of the child's own work in the after-reading (section 3). The busiest device counts
  where there are several.
- **Not the gate's own timestamp queries.** wgpu's timestamps time *this* process's passes; a pass
  queued behind a neighbour shows only as a longer frame, which is the symptom, not the cause.
- **Not a probe frame.** Drawing to find out whether the device is free is work done to raise or
  read a clock — the decline ADR 1519 made for spinning the cores.

A device that offers no counter (another driver) is printed *unread*, and the child is judged on
the processor's figures alone, which is what the gate did before; the print says so, so that it is
never mistaken for the device check passing.

## 3. The threshold and its calibration

`device_busy_percent = 20` in the check file. Measured on 2026-10-05: the idle device with the
owner's compositor reads 0–1% (forty readings); `keep_the_device_busy` — a compute shader of
dependent multiply-adds dispatched and waited for back to back, one submitting thread, about 130 ms
a dispatch — reads 88–100%. The counter is the driver's average over a recent interval and lags: read at once
after the plant exits it has read 78% and 1% on different runs, and in a quiet run of the gate every
child read 1–6% before and 6–9% after, the difference being the child's own tail. Twenty sits above
both — idle, the compositor and a child's tail — and under any neighbour that occupies the device a
fifth of the time, which is already enough to queue a 1.1–1.8 ms step behind a dispatch. A reading
that lags high costs a retry before a child or its verdict after one, never a false verdict.

Planted for a whole run of the gate (the plant spawned by hand from this binary's `busy` phase):
every child read 99–100% busy, every row printed `not judged (device N% busy against 20%, …)`, and
the run exited 0 after 765 s: 20 figures banded, 0 judged. Under the plant every one of the twenty
read above its band's ceiling — the steps 41–96 ms against ceilings of 1.8–48.7 ms — so each was a
false failure the gate would otherwise have given.

## 4. What the probe already caught, and why the check is still owed

On this machine the processor and the device share one package and one memory: under the plant the
calibration probe read 1.24–1.59 ms against its 0.62–0.78 band, so a *saturating* neighbour was
already refused, under the wrong name. The case trap 101 records was a partial one — step rows
failing at a probe the band admitted. The device check names the cause in both cases and catches
the partial one; and on a machine whose device has memory of its own the probe would see nothing.

## 5. What it does not do

It does not wait for a quiet device beyond the load ceiling's own three tries ten seconds apart, and
it does not lower any band. `launch_path` reads the same device and has no such check; its owner's
round may take `device_busy` as written.
