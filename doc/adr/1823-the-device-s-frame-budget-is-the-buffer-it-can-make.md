# 1823 — The device's frame budget is the buffer it can make, and the owner's drawing is drawn on it

Session 1493. Status: **accepted** and **built**. The project owner asked on 2026-10-10 to "create
the biggest PDF which can be rendered on the GPU and continue improving the GPU path as well".
Context: ADRs 0607, 1809, 1810, 1813, 1817; raster's ADRs 0080, 0095; trap 1, 13, 87, 134. Code:
`raster/crates/raster-gpu/src/startup.rs` (`DEFAULT_MAX_FRAME_BYTES`, `MEASURED_WORKING_BUDGET`,
`working_budget`), `raster/crates/raster-gpu/src/device/construct.rs` (`frame_ceiling`), the four
shares in `encode/residue.rs`, `encode/expansion.rs`, `encode/parallel.rs`, `encode/meet/kept.rs`.
Instrument: `crates/render-raster/examples/heavy_drawing.rs`.

## 1. Which budget the witness meets, and the hypothesis that did not hold

The brief named six `FrameBudgetExceeded` sites and guessed the compute lane's edges buffer fires
first. **It is the encoder's count before the walk** (`encode.rs`, `encode` and `replay`): every
command is charged one rectangle instance and one coverage quad, 32 + 64 = 96 bytes, before anything
is flattened, on every lane. The witness's 423 206 976 bytes are 4 408 406 × 96 exactly — the list's
4 408 405 commands and the medium's rectangle. `heavy_drawing` at chop 6 shows the site directly: on
both the default and the moved-view lane it is refused at 295 608 864 = 3 079 259 × 96 bytes after
2.1 s of scene building, with **0 ms of encode**. So ADR 1817's table reads the 96 bytes as "six
flattened edges of 16 B"; they are the pre-count, and a smaller primitive does not move it.

With the budget lifted to 4 GiB no other site fires: the generator at chop 8 draws on all three
lanes (324 134 596, 422 746 776 and 364 863 532 bytes uploaded on `Cpu`, `Compute`, `Gpu`), and the
witness on `Cpu` and `Compute` (323 656 688 and 374 815 748 bytes), executing in 21–67 ms. The edges
do fire first on the compute lane for a middle size: chop 4 passes the pre-count (198 M) and is
refused at 314 462 568 bytes there.

## 2. The generator, and the largest size the device accepted

`heavy_drawing` writes the witness's shape from a formula: an A1 sheet, 20 × 15 blocks, one in four a
hatched room and the rest racks of 4 × 3 cubicles shaded by lines stated as unit steps under a
`1/chop` `cm`, one `q … m l S Q` a step and an `RG` every four, Helvetica labels. Each unit of chop
adds 505 920 strokes to 43 739 other commands; chop 8 is 4 091 099 commands, nine-tenths of the
witness. At HEAD the device drew chop 4 (2 067 419 commands) on `Cpu` and refused chop 5 by one byte
(268 435 457); on `Compute` it drew chop 2 and refused chop 4.

## 3. The two candidates, priced

- **Chunked buffers across submissions** buy nothing at this size. Both adapters here state
  `max_buffer_size` 2 147 483 647 bytes; the witness's largest stream is at most 4.4 M quads of 64
  bytes, 282 MB, a seventh of it. A frame needs a second buffer only past about 33 M marks, and the
  price is splitting batches' instance ranges, the compute lane's edges and the retained encode's
  replay across submissions. Declined until a page reaches it.
- **A budget stated from the adapter's memory** cannot be stated: `wgpu` 30 reports no memory size
  and no budget (Vulkan's `VK_EXT_memory_budget` is not surfaced), and an integrated GPU's memory is
  the processor's. What the adapter *does* state is the largest buffer it makes and binds, and that is
  what a sum charged before allocation must stay under for every buffer of it to exist. **Built**:
  `frame_ceiling` holds a device to the smaller of the request, `max_buffer_size` and
  `max_storage_buffer_binding_size`, and `Limits::max_frame_bytes` reports which. The request's
  default is 2 GiB, which is ADR 1810's ceiling on one interpreted page, so a stroke-heavy page the
  interpreter admitted is not refused by the device's count (96 bytes a command against the list's
  ~176). Its cost: llvmpipe binds 128 MiB of storage and is held to that on every lane, where 256 MiB
  stood — conservative for the two lanes that bind none.
- **The pre-count at 64 bytes**, one instance a command, was read and not built: the witness would
  still ask 282 MB, over 256 MiB, so it does not reach the target alone, and whether no command ever
  writes two instances is a claim to prove before a bound rests on it.

## 4. What the raise does not move

Three caches and the encode queue take shares of the frame budget. Raising them eightfold would be a
cost every page pays for an admission one page needed, so they take their shares of the budget up to
`MEASURED_WORKING_BUDGET`, the 256 MiB they were measured at, and a caller who lowers the budget still
lowers them. Every size HEAD drew draws the same bytes after (FNV-1a digests: chops 1, 2, 4 on `Cpu`,
1, 2 on `Compute`); the corpus arms are compared by page in the record.

**Agreement is the corpus gate's, not byte identity.** At every size the device draws, its bytes
differ from the oracle's (four strips) by at most 11 levels on `Cpu` and 6 on `Compute`, mean 0.05–0.14,
SSIM ≥ 0.9995 — inside `corpus.rs`'s bound, and the same at chop 1 as at chop 8, so the difference
is the lanes' own (ADR 0082's relaxation) and not the size's. The sampled `Gpu` lane at chop 8 is
outside it (SSIM 0.975), as its 0.18-pixel marks predict (raster's ADR 0076).

**Nor does it make the page fast.** Under the shipped default after the raise the witness's device
frame takes 7 884 ms on `Cpu` and 8 409 ms on `Compute` against the oracle's 7 590 ms at four
strips (one walk, loaded machine; 12.6–23.3 s under heavier load), and the host's encode is most of
it. What the raise removes is the refused frame's cost and the processor fallback that followed it;
what makes the frame fast is ADR 1817's mark classified before it is geometry.

## 5. The arms, and the next ceiling

Six corpus arms against HEAD's export, by page: **no frame digest moved on any arm**; at 4× the
device now draws `issue12810.pdf` (609 MB at HEAD) on every lane and `issue14497.pdf` (1.15 GB) on
`Gpu`, each agreeing with the oracle, so `REFUSED_BY_THE_DEVICE_AT_FOUR` loses its first name.

After the raise the generator draws chop 12 — 6 114 779 commands, 455 652 176 and 560 191 170 bytes
uploaded on `Cpu` and `Compute` — and is refused at chop 16 (8 138 459 commands) and 24 by the
**resource** budget, `max_resource_bytes`' 512 MiB: every one of the producer's steps is an outline
of its own, uploaded and resident. The interpreter's 2 GiB list admitted chop 24 (12 185 819
commands). So the next ceiling is not the frame's but the outlines', and the right answer to it is
not a larger number: the strokes are one segment each, and an outline per segment is what ADR 1817's
classification removes before anything is uploaded.
