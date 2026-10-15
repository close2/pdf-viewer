# 1563 — The compute lane and a generated shader keep their refusals as the render lane does

Slot 6 of batch eighty-nine, 2026-10-15, a pixels round under the owner's switch to code
improvement. ADR 1962; no question; no ledger row moves.

**Premise.** It held for `compute.rs`. Its three modules and four pipelines were created outside
any scope, and with the new scope planted out, the forced refusal panicked in `wgpu`'s handler. For
`function.rs` it held of `pipeline/function.rs`, not `src/function.rs`, which is pure analysis. A
refused generated pipeline was compiled again on every ask. **Hypothesis.** It held: no page moved
on any device arm, and a forced refusal drew on the processor and was said once.

**Built.** Every compute creation runs inside `captured`. The device keeps the four pipelines, or
the refusal, as `compute::Kept`; a pipeline refusal is the new `PipelineProblem::ComputePipeline`.
The generated store keeps refusals by `FunctionKey`, and releasing the program drops them. The
drive found the title carrying `wgpu`'s chain of causes on four lines. The title now takes the first
line, and the terminal's note ends with the refusal (`surface.rs`, wording only). Two tests hold
this. With the keeping planted out, the compute test fails at its second frame and the generated
test fails on its compile count. With the scope planted out, the compute test panics (trap 13).

**Proof that nothing moved.** Four device arms (gpu and compute, 1x and 4x) were run from an export
of HEAD plus these files (`tools/batch.sh export 1563 --as mine`). They matched HEAD's arms
by page digest: 968, 968, 965 and 965 pages, 0 differing in each. The comparison was checked
against two lanes known to differ: HEAD's gpu-1x against its cpu-1x, 106 pages. `raster_golden`
from the export held 974, moved 0; it does not link `raster-gpu`.

**Drive.** The export's `quorra --coverage compute` ran under my own Xvfb on `:63`, on
`tracemonkey.pdf` and `PDF20_AN001-BPC.pdf`. The count pipeline was broken by an environment
variable that existed only in the export. Every page was refused and drawn on the processor, and
the note came once against three and four refused frames. The title was one line. Without the
plant, the device drew every page.

**Files.** `raster/crates/raster-gpu/src/{compute,device,pipeline}.rs` and `src/{error,pipeline}/…`,
`crates/viewer-ui/src/bin/quorra/surface.rs`, ADR 1962, this record.

**Gates.** rustfmt clean on the six files. clippy `-D warnings --all-targets` exit 0 on `-p
raster-gpu` and `-p viewer-ui`. nextest exit 0: `-p raster-gpu` 725 passed, 2 skipped; `-p
viewer-ui` 192 passed, 2 skipped. `tools/state.sh docs` exit 0. `cargo test -p conformance
--no-fail-fast` exit 101: 434 passed, 2 failed on record 1562. t2 `raster_golden` exit 0. The four
device arms exit 0. One `--clock` hold, load 3.68, 5 s, its output whole in the round's scratch: `frame_budget`
exit 0 and `launch_path` exit 0, clocks not judged, cold bring-up 20.8 ms.
