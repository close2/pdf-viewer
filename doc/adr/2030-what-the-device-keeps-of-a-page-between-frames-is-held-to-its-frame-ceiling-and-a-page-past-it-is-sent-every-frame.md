# 2030 — What the device keeps of a page between frames is held to its frame ceiling, and a page past it is sent every frame

Session 1597, slot 6 of batch ninety-four, `doc/todo/75` item 5, for the project owner's ask of
2026-10-19 about a page kept on the device. Status: **accepted** and **built** in `raster-gpu`
(`device/residency.rs`'s `ResidencyBudget`, `ResidentCharge`, `Residency` and the public
`ResidencyRefused`; `device/staging.rs`'s `instance_streams`; `Limits::max_resident_bytes`,
`Options::max_resident_bytes`, `Device::resident_bytes`, `RetainedScene::residency_refused`) and
`render-raster` (`present::Retained::residency_said`; `examples/zoom_frame.rs`'s
`ZOOM_FRAME_RESIDENT`). Context: ADRs 1823 §3, 1992 §5, 1993, 2022. Tests: `raster-gpu`'s
`tests/residency.rs` and `device::residency`'s unit test. Cites no clause and moves no ledger row.

## 1. What the device holds between frames, and the two premises that did not hold

ADR 1993's kept instance streams are the one page-derived allocation the device held between
frames under no bound: the compute lane's segment arena holds outlines the resource budget charged
(`max_resource_bytes`), its edges capacity is priced against the frame budget, images, ramps and
meshes are resident resources, and the atlas is one texture of a stated size. **The brief's "ADR
2016's windows and ADR 1968's lanes each hold more" is not the device's**: ADR 1968's remembered
refusals are values in the window, ADR 2016's windows lists in the confined worker. **Nor could the
budget be "a share of the adapter's reported memory"**: `wgpu` 30 reports no memory size and no
budget (ADR 1823 §3; `MemoryBudgetThresholds` is an instance option by which a backend refuses
creation at a percentage of the native budget, and returns no figure).

## 2. Decision

- **One figure, stated by the device**: `Limits::max_resident_bytes`, the smaller of the host's
  `Options::max_resident_bytes` (a request, defaulting to `DEFAULT_MAX_FRAME_BYTES`) and the frame
  ceiling — the largest buffer the adapter makes and binds (ADR 1823). Held to it, what the device
  keeps of the pages it retains is never more than what it admits one frame to allocate.
- **Charged before a buffer is made, given back by construction**: a retained encode's first frame
  charges its three streams' bytes; the charge is a value held beside the buffers in the encode's
  cell, and its `Drop` returns the bytes, so the encode's replacement, `forget`, a new scene or the
  handle's drop each give them back. A line stream one encode adopts from the one it replaces (ADR
  2022) is charged to the encode that holds it: the replaced encode's charge is returned when it is
  dropped in `RetainedScene::prepare`, before the new encode's first frame charges.
- **A page past it is drawn the same and sends its streams every frame**: the cell keeps the
  refusal for the encode's life, so no later frame asks again; `RetainedScene::residency_refused`
  names the bytes needed, held and the budget; `render-raster` prints it once per scene on the
  terminal (`note: …`), so a zoom whose every step is a new encode says it once.
- **A frame's own attempts share its streams**: a frame run again after the compute lane grew
  (raster's ADR 0095) reads the streams its first attempt made, kept or not. `Device::render`'s
  re-run used to send them twice while `reported` counted one copy; it now sends one.

**What it means for the window**: one page is retained per device (`render-raster`'s one
`FrameSlot`), and the frame budget admitted its streams, so the default never refuses the window's
page; the budget bites where a host retains several encodes on one device or asks for less. A
smaller default would be a figure nobody measured, priced on the witness below as 25–36 ms of
transfer a replay.

## 3. What it moved

One `--clock` hold (load 11.2 → 8.6, headless, the Radeon 890M through RADV, whose budget is
2 147 483 644 bytes), the witness at the fit, 1.25× and 1.25× again (the replay), minimum of three
rounds, two runs a lane, the default and `ZOOM_FRAME_RESIDENT=1` interleaved:

| the replay | default (kept) | refused |
|---|---|---|
| `Cpu`: frame | 132.1, 220.8 ms | 222.1, 175.2 ms |
| `Cpu`: transfer, bytes | 1.8, 2.0 ms; 36 360 892 | 30.6, 27.2 ms; 300 860 300 |
| `Compute`: frame | 134.4, 155.9 ms | 199.4, 216.4 ms |
| `Compute`: transfer, bytes | 2.4, 2.6 ms; 95 529 537 | 37.9, 38.1 ms; 360 028 945 |

The witness keeps 264 499 408 bytes (ADR 1993's figure to the byte), 12.3 % of the budget; the fit
and the step send what they sent in both arms. The refused arms printed the note once a round. The
turn gate's eleven rows: 33 figures banded, 24 judged, 0 outside.

## 4. Proof that no pixel moved

Every frame's digest in the hold is the same in both arms and is HEAD's, taken from HEAD's export
in a walk of its own: `4e8b7a49e2066799` and `2a3e472c0cca768b` under `Cpu`, `56a7982441d77cb1` and
`a38a4dba7cd3ac19` under `Compute`, at the fit, the step and the replay; HEAD's replay sent the
default arm's bytes, stream by stream. `tests/residency.rs` holds a refused page to its immediate
frame's bytes on the adapter, its replay to sending its streams again, the refusal's three numbers,
the charge returned on `forget` and on drop, and a second page refused by what the first holds.
Planted back — the ceiling ignored, the refund dropped — two tests fail each time.
