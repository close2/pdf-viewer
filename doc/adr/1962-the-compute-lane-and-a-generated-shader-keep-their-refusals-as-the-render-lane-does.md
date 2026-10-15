# 1962 — The compute lane and a generated shader keep their refusals as the render lane does

Session 1563, slot 6 of batch eighty-nine. Status: **accepted, built**. Context: ADR 1929 (a pipeline
the backend's compiler refuses is captured and kept) and its section 5, which named these two for the
round that owns the files; raster's ADR 0042 (a refused pipeline is a value), ADR 0053 (a §7.10.5
program's generated shader), ADR 0080 (the compute coverage lane). Code:
`raster/crates/raster-gpu/src/compute.rs` (`Pipelines::compile`, `Kept`, `dispatch_chain`),
`raster/crates/raster-gpu/src/pipeline/function.rs` (`function_pipeline`, `forget_program`),
`raster/crates/raster-gpu/src/error/pipeline.rs` (`PipelineProblem::ComputePipeline`). It cites no
clause and moves no ledger row.

## 1. What the two sites did

- **The compute lane compiled its three modules and four pipelines outside any error scope.** A
  module or pipeline the adapter refused went to the device's uncaptured handler, and the handle came
  back as though built. On a headless device that handler is `wgpu`'s default, which panics: the
  plant in section 3 panicked in `wgpu_core.rs` with the scope taken out. Under `quorra` it is the
  handler `render-raster` installs, which records the error for the window to report; the frame then
  dispatched pipelines that did not exist.
- **A refused generated pipeline was not kept.** `function_pipeline` cached only a success, so a
  shader, style and format the adapter refused was generated, parsed and built again in every frame
  that reached it. On DX12 that is a whole failing compile inside each such frame, which is the cost
  ADR 1929 removed from the fixed table.

## 2. The decision

- **Every creation in the compute lane runs inside `captured`**, both scopes, as the render lane's do.
  A refused module is `PipelineProblem::Shader` under its label. A refused pipeline is the new
  `PipelineProblem::ComputePipeline`, which has no format field because a compute pipeline has no
  colour target.
- **The device keeps the lane's outcome, built or refused** (`compute::Kept`). The first frame that
  takes the lane compiles it. Every later frame that takes it is refused in the same words, before
  any of its buffers is made, and nothing is compiled again. The four pipelines are kept or refused
  together, because a frame of the lane needs every one of its passes.
- **A refused generated pipeline is kept by its `FunctionKey`** (`function_refused`), the way `refused`
  keeps the fixed table's by kind and format. A frame that reaches the key is refused in the same
  words without a second generate, parse or build. A generator defect is still reported on every
  frame that reaches it, at the cost of one compile. Releasing the program drops its refusals along
  with its pipelines and modules. That is why the shader's owner is now recorded before the parse,
  so that a refused module's refusal is found and dropped too.
- **A host hears a refused frame.** `quorra`'s window already answers one by drawing it on the
  processor and saying so once, in the title and on the terminal (ADRs 1821, 1929).
- **The title takes the refusal's first line, and the terminal's sentence ends with the refusal.**
  `wgpu` writes a refused pipeline as a chain of causes, one per line. In the title that put four
  lines into one window name. On the terminal it split the sentence before its own ending. This
  holds for the render lane's refusals as well as the compute lane's. The trace and the terminal
  still carry every line (`crates/viewer-ui/src/bin/quorra/surface.rs`, `adopt`).

## 3. Held by

- `compute::refusal_tests::a_refused_compute_pipeline_is_refused_by_name_kept_and_takes_only_its_lane`.
  The count pipeline is planted with an entry point its module lacks; the plant is a thread-local
  `#[cfg(test)]` seam. The first frame of the lane is refused, naming `raster compute count`. The
  plant is then lifted, so the second refusal in the same words can only be the kept one. A rectangle
  still draws on the same device. With the keeping removed, the test fails at the second frame. With
  the scope removed, it panics in the uncaptured handler.
- `pipeline::function::tests::a_refused_generated_pipeline_is_kept_until_its_program_is_released`
  counts compiles. The second ask compiles nothing, and releasing the program empties the refusals.
  With the early return removed, the test fails on the count.
- **Driven.** `quorra --coverage compute` under Xvfb on `tracemonkey.pdf` and `PDF20_AN001-BPC.pdf`,
  with the count pipeline's entry point broken through an environment variable. The variable
  existed only in an export of the tree, never in the tree itself. Each page was refused by the
  device and drawn on the processor. The terminal said `note: the page was drawn on the processor
  instead, …` once in each run, against three and four refused frames in the trace. The title was
  one line on every page. Without the plant, the export's build drew every page on the device.

## 4. Not done

A refused frame still costs the device its encode before the upload refuses: 1 to 17 ms a page in
the drive. That is because `quorra` remembers only budget refusals (`renderer/lanes.rs`,
`remembered`). The window does not yet hold a pipeline refusal, which is a fact about the adapter
rather than about the page.

The compute lane refuses the whole frame, and the processor draws it. Under `Coverage::Cpu` with
compute assist (ADR 0090), the tiles the lane would have drawn could instead go to the path lane on
the same device. That would be a change to what the encoder routes, not to what the store keeps, and
it would need a measurement first.
