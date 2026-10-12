# 1885 — The device keeps `wgpu`'s default memory hint, because the small blocks cost every frame

Session 1524, asked by the batch's brief: ADR 1872 priced `wgpu::MemoryHints::MemoryUsage` against
`request_device` and named the frames it would move. Status: **accepted**; the hint was built,
measured and **not kept** — `raster-gpu`'s `request_device` (`device/construct.rs`) asks the default,
as before. Context: ADRs 1532, 1585, 1872; `doc/checks/launch-path.toml`'s `bring_up_ms`,
`doc/checks/turn-path.toml`. Instruments: `render-raster`'s `examples/bring_up` (its second argument
the hint), `tests/turn_path`, `viewer-ui`'s `tests/launch_path`. Cites no clause.

## 1. What it bought

In one clock hold with the carve-out at 362–364 MiB held by others (`mem_info_vram_used`), ten
interleaved processes of `bring_up vulkan` under each hint: `request_device` **4.13–5.84 ms under the
default, 2.11–2.91 under `MemoryUsage`**, the usable device 20.2–30.5 against 19.4–23.9 ms. The launch
gate's `bring_up_ms`, raster's own device: 18.04 ms on HEAD, 15.99 and 17.09 with the hint, all three
inside 13.6 .. 20.6 tonight — the carve-out had more room than ADR 1872's 409–420 MiB. No frame's
digest moved: the witness at five views on both device lanes, and the six corpus arms.

## 2. What it cost

`turn_path`, three exports interleaved twice in one hold (HEAD, HEAD plus ADR 1884, the same plus
the hint): ADR 1884 moves no figure; the hint moves **every step's transfer**. `issue12841_reduced.pdf`'s
step transfer 1.20–1.25 → 2.72–2.75 ms and the step 2.85–2.91 → 3.20, 4.05 and 4.28 ms, **outside its
band of 2.50 .. 4.00 in two of three runs**; `bug1721218_reduced.pdf`'s step transfer 0.27–0.29 →
1.53–1.60 ms and the step 82.6–83.3 → 87.6 ms; `22060_A1_01_Plans.pdf`'s step 14.6–14.8 → 15.8 ms. On
the witness at 2× the transfer and the readback each grew by 1–3 ms. The host-visible blocks are 4
to 32 MiB under this hint against 64 to 128 under the default, and a frame's staging is asked of
them every frame.

## 3. Decision

**A launch pays `request_device` once; a person zooming pays the transfer at every step.** Principle
2 counts both, and an interactive frame's latency first: 2 to 3 ms off one bring-up, on a desktop
whose carve-out is full, does not buy 1.3 to 1.5 ms on every step after it and a page turn outside its
band. The hint is not taken, and ADR 1872 §2's band, with its comment, stands.

**The lever left is `MemoryHints::Manual`**, whose range sets the first device block and the largest
(`wgpu-hal` 30: host blocks half the device's, clamped to 4 .. 256 MiB): a small first block and the
default's ceiling could give bring-up the small block and the frames the large ones. It is a
measurement of its own — the same three instruments, the same hold — and is not taken here.
