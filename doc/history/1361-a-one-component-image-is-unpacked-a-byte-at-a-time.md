# 1361 — A one-component image is unpacked a byte at a time, and a warm-up is read without waiting

The PERF slot of batch fifty-six. ADRs 1557 and 1558. No ledger row moved; no question written.
Tree state for every figure: the batch worktree with all six rounds' edits as they stood that night.

**The unpack (ADR 1557).** `packed_pixels` builds 256 runs of pixels from `sample_rgba`'s own
answers for every image of one component at 1, 2, 4 or 8 bits with no colour key and no matte;
`fill_packed_rows::<RUN>` copies one run a byte. The brief's sample-layout sentence is §8.9.3's,
not §8.9.5.2's. Callgrind, `callgrind_interpret` page one, worker beside the binary: bug1815476
403.2 → 127.9 M (`decode_ccitt` 244.3 → 5.4), issue9940 543.4 → 80.8 M (`unpack` 496.2 → 33.6).
Counted before building more: no 16-bit, no 2-bit and no sub-byte multi-component image in the
corpus's uncompressed dictionaries, so those stay per-sample; 4-bit and 8-bit `Indexed` share the
route because the 4-bit image on bug1815476's page cost 11.6 M of `raw_sample` alone.

**The launch (ADR 1558).** bug1815476's page one is interpreted at 20.3–20.9 ms against a device at
22.9–24.0: the device is the longer thread on all five rows. The gate prints the first frame's
stages: scene 0.1–0.3, device 5.4–9.2 (encode 1.6–3.8, transfer 0.2–2.6, readback 0.6–1.0, one
first-use pipeline 0.3 on three rows); `first_frame` settled 100 ms or not reads 8.4–9.7 against
8.5–8.7, so no warmth. A window's `Device::startup` read waited for the warm set — 3.2–4.3 ms on
RADV, 22 on llvmpipe — and the warm-up's state now has its own lock: 0.000 ms after. Under Xvfb
the wait moves into the ground's first present (31.7 ms device → surface either way); reordering
the warm set changed nothing and was reverted. bug1815476's bands: `first_page_ms` 28.9..41.6 →
23.8..36.3; `peak_anon_mib` 39.0..50.0 → 47.5..59.0 — the stencils now overlap the decodes ahead
(massif 27.65 MB both arms; +8.5 MiB only with two or more rayon threads).

**Gates.** rustfmt --check on my files 0. clippy -D warnings pdf-model, viewer-ui (launch_path),
raster-gpu --all-targets 0. nextest of the three 2618 passed (pdf-model 1780). `cargo test -p
conformance` 0, 380 passed. Behind the lock: raster_golden 0 (held 974, moved 0, 18.3 s);
render-raster corpus 0 (968 / 0 / 0 / 6, 1-vs-N 0, 51.5 s); oracle 0 (1025 agree, 47
contradicted, 836 ambiguous, 69.6 s); launch_path no clocks 0 (26 banded, 0 outside); with clocks
after the move 0 (42 banded, 0 outside; clocks not judged, the parent's calibration 0.905 ms).

**Left.** The window's ground waits for the warm set on llvmpipe, store lock or driver not
separated; on RADV through a real window it is the owner's to measure. 0.5–3.1 ms of the first
frame's span is outside `FrameCost`. A proposed trap: a measurement binary copied aside without
`pdf-sandbox-worker` beside it draws no CCITT, JBIG2 or JPX image and says nothing.
