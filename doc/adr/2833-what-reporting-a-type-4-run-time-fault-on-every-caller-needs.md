# 2833 — What reporting a type 4 run-time fault on every caller needs

Round 2007, the ledger lane.
Status: **deferred** — the construction a later round builds, not built here; the round that wrote it spent its wall
budget on ADR 2832.
Clauses: ISO 32000-2 §7.10.5.3, §7.10.1.

## What stands

§7.10.5.3 states three errors a type 4 program can meet only for some inputs: the remaining operand count differing
from `/Range`'s, an output that is not a number, and stack overflow. `pdf_colour::function::Function::eval` answers
anyway and records the error as an `EvaluationFault` bit on the function (`Faults`, an `AtomicU8`), and
`evaluation_fault()` reads it. Two callers ask it and report: shading ramps (`pdf_colour::shading`, which refuses the
ramp) and a soft mask's `/TR` (`pdf_model::soft_mask`). Tint transforms (`colour/separation.rs`, `to_rgb.rs`,
`to_cmyk.rs`, `queries.rs`), transfer functions (`transfer.rs`), black generation (`black_generation.rs`), mesh
shadings (`mesh.rs`) and the type 1 grid record and are never asked.

## Why it is not a one-line change

1. **`Faults::clone` copies the bits.** A colour space, a graphics state and a shading are cloned freely, so the
   instance the interpreter could ask after the page is not always the instance that evaluated. The first step is to
   make the bits shared across clones (`Arc<AtomicU8>`): a clone of a function is the same function of the file, so
   one fault seen by any clone is the function's.
2. **The interpreter holds no list of the functions a page reached.** Each kind needs an `evaluation_fault()` of its
   own that walks to its functions — `ColourSpace` through `Tints::Transform` and the alternate and base spaces, the
   transfer and black-generation entries of the graphics state, a mesh's and a type 1 shading's functions — and the
   interpreter asks each resource it resolved, once, at the end of `interpret`, pushing one `Unsupported` variant
   naming §7.10.5.3 and the fault's `Display` sentence (trap 11: it fires only on a recorded fault, which only the
   clause's three errors set).
3. **Evaluation after `interpret` returns.** Every CPU evaluation found is in `pdf-colour` or `pdf-model`; an image
   whose conversion is deferred to raster threads would meet a fault after the report is closed, and has to be asked
   where the conversion completes. This census is the first task of the round that builds it.

## The GPU evaluator

`Function::device_program` hands `pdf_render::program::ProgramStep`s to `render-raster`'s shading shader, which runs
the stack machine per pixel and records nothing. To report, the shader needs a fault word in a storage buffer bound
with the shading, set by `atomicOr` where the program's end-of-run count or type check fails or a push overflows,
and read back after the frame's submission. That readback is a map on the present path, which principle 2 prices: it
should be read only after a frame has presented, and only for a scene that drew a type 4 shading. An alternative that
needs no readback is to refuse `device_program` where the abstract walk cannot prove the three errors absent for the
region, and draw that shading on the CPU, whose evaluator reports.
