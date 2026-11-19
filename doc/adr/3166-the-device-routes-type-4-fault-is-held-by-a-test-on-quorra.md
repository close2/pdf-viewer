# 3166 — The device route's type 4 fault is held by a test on quorra as well as on the processor

Session 2116. Status: **accepted and built** in `render-raster` (`tests/headless_quorra.rs`). Follows ADR 2940, which
priced and closed ADR 2849; changes no code.
Clauses: ISO 32000-2 §7.10.5.3, §8.7.4.5.2. Row §7.10.5.3 stays `implemented`.

## What the brief asked, read against today's code

The brief named ADR 2849's deferred item — `Function::device_program` handing `render-raster`'s shader a type 4
program that records nothing — as open, and asked for its two numbers. Both were taken in session 2063 (ADR 2940):
the census of type 1 shadings over 67 447 paths found none that `Function::may_fault` sends to the processor, and
the fault word read back after present cost 0.96 ms a frame on the RX 580. Remedy 2 was built: `device_program`
returns `None` for a program `may_fault` cannot clear, and the processor's grid draws it. Nothing of the item is
left to price or build, and this round took neither number again.

What ADR 2940 held only on the processor's side was the *route*: its test asks `pdf-colour` that such a program is
refused a device and records its fault when evaluated there. That the GPU backend then draws the shading from the
grid, and that the grid it drew is what a host's question after drawing reads (ADR 2946), was asserted nowhere on
the device's side.

## Decision

One test on quorra, beside the two that already hold the device's own refusals:
`quorra_draws_a_program_some_input_makes_fault_from_the_grid_and_the_fault_is_said`. The program leaves two outputs
for x above 0.5 and three at or below it; the interpretation's opacity corner is x = 0, where it conforms. It asserts
that the display list offers no program, that `Interpretation::function_faults_after_drawing` is empty before any
backend has drawn, that quorra draws the page evaluating no program on the device
(`last_function_paints().evaluated() == 0`), that the question after quorra drew names
`EvaluationFault::OutputCount`, and that quorra's picture is the CPU oracle's within the step allowance the other
function-shading scenes take. Run on the RX 580 (unpinned, under `xvfb-run -a`), it passes.

The processor backend's half is `pdf-model`'s
`function_faults::a_type_1_shading_that_errs_where_it_is_drawn_is_said_after_drawing`, unchanged.

The first draft put the count error below 0.5; the interpretation's own corner sample met it, the fault was said by
the interpretation, and the question after drawing — which says only what the interpretation did not — answered
nothing. That is the construction working (ADR 2946 says each fault once), and the fixture now errs where only the
grid reaches.
