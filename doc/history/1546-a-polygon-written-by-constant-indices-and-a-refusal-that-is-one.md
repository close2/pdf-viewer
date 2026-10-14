# 1546 — A shader's polygon is written by constant indices, and a backend's refusal is a refusal

Slot 1 of batch eighty-seven, 2026-10-14, a pixels round under the owner's switch to code
improvement, on the owner's DX12 report. ADRs 1928 and 1929; no question; no ledger row moves.

**Premise.** Held for (a): `image.wgsl` appended `out.corner[out.count]` in loops bounded by
`polygon.count`, and the owner's two positions are `naga`'s `while(true)` and its second append,
seven lines down from ours. Did not hold for (b)'s "at bring-up": the image lane is not in the
warm set, so a refusal arrives with the first frame that draws an image (ADR 1929 §3). Found
beside it: `line.wgsl`'s `cut` had the same append, and `captured`'s validation scope could not
catch the refusal at all — a backend compiler's refusal is `wgpu`'s internal type.
**Hypothesis.** Held as far as this machine can test: the translated HLSL writes no array element
by a run-time index. FXC itself was not run (no FXC here).

**Built.** Both polygons appended by eight constant-index selects, every corner loop to the
constant eight with a break at the count, arithmetic and order unchanged. `captured` opens an
internal and a validation scope. The store keeps a refused pipeline by kind and format and answers
the next ask in its words without compiling; `quorra` draws the refused frame on the processor and
says so once, as it already did for every refused frame. A planted test (an entry point the module
lacks) fails with the keeping removed.

**Proof that nothing moved.** A scratch probe drew 612 frames per lane (1 296 oblique images, 540
thin-line scenes) on `Cpu`, `Gpu` and `Compute`, on llvmpipe and on the Radeon 890M: 1 836 digests
an adapter, HEAD's export against this round's, 0 differ; a planted halved area moved all 1 296
image frames and 261 line frames. The six corpus arms against `/home/AI/arms-1546/`: 968, 968, 968,
965, 965 and 965 pages, 0 differ, 0 on one side only; the sandbox worker's digest is HEAD's.

**Unfinished.** The compute lane's pipelines (`compute.rs`) compile outside any error scope, and a
refused generated §7.10.5 pipeline is not kept — named in ADR 1929 §5 for whoever owns those files.
The owner's re-run is `doc/todo/50`'s item 5.

**Files.** `raster/crates/raster-gpu/src/shaders/{image,line}.wgsl`, `raster-gpu/src/pipeline.rs`,
`pipeline/tests.rs`; `crates/render-raster/src/lib.rs`; `doc/todo/50`; ADRs 1928, 1929; this record.

**Gates.** rustfmt clean on the three Rust files. `clippy -D warnings --all-targets`
exit 0 on `raster-gpu` and `render-raster`. `nextest -p raster-gpu` exit 0, 723 passed — `line_lane`
8, `function_conformance` 3, `encode_threads` 11 of them; `nextest -p render-raster` exit 0, 104
passed. `cargo test -p conformance` exit 0, 436 passed. Six arms exit 0. One `--clock` hold, 315 s,
exit 0: `turn_path` 33 of 33 banded figures inside, `launch_path`'s counted half 31 of 31 inside.
Duration 6325 s.
