# 1929 — A pipeline the backend's compiler refuses is captured, kept, and refuses only the frames that need it

Session 1546, slot 1 of batch eighty-seven. Status: **accepted, built**. Asked by the project owner
from Windows without `--cpu` (2026-10-15), with ADR 1928. Context: raster's ADR 0042 (a refused
pipeline is a value) and ADR 0040 (the warm set); ADR 1821 (a refused page is paid once). Code:
`raster/crates/raster-gpu/src/pipeline.rs` (`captured`, `PipelineStore::get`), its test in
`pipeline/tests.rs`; `crates/render-raster/src/lib.rs` (`QuorraRasterError::Render`). It cites no
clause and moves no ledger row.

## 1. Why the refusal was not a refusal

`captured` ran every compile inside a **validation** error scope. `wgpu` sorts an error into one of
three types and a scope catches one. A descriptor `wgpu` itself finds wrong is a validation error;
a pipeline the backend's own compiler refuses is an **internal** one — `wgpu-core` 30 maps
`hal::PipelineError::Linkage` to `CreateRenderPipelineError::Internal`, whose type is `Internal`. So
FXC's refusal passed the scope, reached the uncaptured handler (which printed it), and `get` handed
the frame a pipeline handle `wgpu` had marked invalid as though it were built.

## 2. The decision

- **`captured` opens an internal scope and a validation scope**, pops them innermost first, and
  returns whichever caught. A backend's refusal becomes `PipelineProblem::Pipeline`, and the frame
  that needed it `RenderError::PipelineUnavailable`, in the adapter's words.
- **The store keeps a refused pipeline**, by kind and format as it keeps a built one, and answers a
  later `get` with the same refusal without compiling again — the reason it already kept a refused
  module: the same descriptor is refused the same way every time, and on DX12 asking again is a
  whole failing compile inside every frame that draws an image.
- **One refusal takes only its frames.** A frame that needs no refused pipeline still draws on the
  device; another format of the same kind is still compiled (raster's ADR 0042 test stands).
- **What a host hears is a refused frame**, which `quorra`'s window already answers by drawing it on
  the processor and saying so once, in the title and on the terminal (`surface.rs`, ADR 1821).

## 3. The premise that did not hold

The brief asked the lane to refuse **at bring-up**. The image lane is not in the warm set (raster's
ADR 0040: the rectangle, coverage, composite, blit and present passes are), so it compiles when the
first frame with an image asks for it, and that frame is where the refusal arrives. Warming it
would cost every device a background compile per format to save a refusing adapter one failing compile
once, now that the refusal is kept; the person sees the same thing either way — the first page
with an image drawn on the processor, said once.

## 4. Held by

`a_frame_whose_pipeline_will_not_build_is_refused_by_name_and_only_that_frame`: the image lane is
planted with an entry point its module lacks, a frame with an image is refused naming `raster image`,
the second frame hears the same words with the refusal kept, and a frame of a rectangle draws on
the same device. With the keeping removed it fails. The plant is a validation error, because no
backend here refuses with an internal one; the internal scope rests on `wgpu-core`'s mapping above.

## 5. Not done

The compute lane's pipelines (`compute.rs`) are compiled outside any scope, and a refused generated
§7.10.5 pipeline is not kept; both are named for the round that owns those files.
