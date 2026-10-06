# 1367: the device thread is named, and `request_device` compiles nothing

Performance slot, batch fifty-seven. ADR 1569; no ledger row, no question, and no ADR 1570 (nothing
was dropped that a later round could want back without a test saying so).

**The device thread, named** (ADR 1569 section 1). `create_launch_instance_timed` returns
`LaunchSteps` (instance, adapter check), `QuorraRasterizer::startup` hands them over with the
device's `StartupTimings`, and the launch gate prints one line for the bring-up child and four
device-thread instants on every first-page timeline. On RADV, thirty interleaved children:
instance 11.1–12.5 ms (the loader opening the radeon and lavapipe drivers, 77.5 M of 90.2 M
instructions), adapter check 3.0–9.0 (the kernel; one `AMDGPU_INFO` ioctl took 6.5 ms under
strace), `request_adapter` 0.2, `request_device` 2.5–3.0 before and 1.7–2.1 after, assembly and
host about 1.2. A headless gate has no surface configure.

**The lever** (sections 2 and 3). A release instance carried `VALIDATION_INDIRECT_CALL`, and
`request_device` compiled two compute pipelines for it. Raster records no indirect call, so the
flag is off, and a test reads the crate's sources so that the flag comes back if one appears.
Callgrind, A/B arms exported from HEAD with only that line between them, md5-distinct:
`Device::build` 10.05 → 6.55 M, bring-up 90.16 → 86.65 M. Nothing else measured to take; leaving
`lavapipe` unloaded (about 2 ms) takes an environment variable, which raster's ADR 0017 keeps out.

**Bands: unmoved, with the reason in the file.** Six gate runs interleaved A/B read 18.7–25.8 ms
of bring-up on both arms, and `request_device` read 4–10 ms on both. Under strace the excess sat in
the kernel's GEM allocations, with 1 GiB free. A 0.9 ms saving does not show in that swing.

**For the owner: the first present on the real adapter** (`Xvfb` has no DRI3). After the merge's
`install`, on your display, closing each window once page one is drawn; `graphics device` to
`surface configured` is the span ADR 1558 read as 31.7 ms on `llvmpipe`:

```sh
cd /home/cl/projects/pdf-viewer && for i in 1 2 3 4 5; do target/quorra --trace=launch \
  doc/ISO_32000-2_sponsored_EC3.pdf | grep -E 'graphics (instance|device)|surface configured|first present'; done
```

**Gates.** `rustfmt --check` on the five source files: 0. Clippy `-D warnings` on `raster-gpu`,
`render-raster` and `viewer-ui`, all targets: 0 each. `cargo nextest run`: `raster-gpu` 686
passed, `render-raster` 100, `viewer-ui` 148. `cargo test -p conformance`: 0, 52 binaries passed.
Behind the lock: render-raster corpus at 1×, exit 0, 968 / 0 / 0 / 6 and 1-vs-N 0, in 68 s;
launch_path counted half, exit 0, 26 banded and 0 outside, in 111 s; raster examples, exit 0,
14 passed, in 76 s. Clocks: six runs, four exit 0 and two exit 101, failing on both arms alike.
