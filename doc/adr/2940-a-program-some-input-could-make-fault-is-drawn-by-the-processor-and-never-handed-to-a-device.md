# 2940 — A program some input could make fault is drawn by the processor, and never handed to a device

Session 2063, a pixels round on the RX 580. Status: **accepted and built** in `pdf-colour` (`function.rs`:
`Function::may_fault`, `faults_excluded`, `joined`, `promise_stack`, and a fourth condition in
`Function::device_program`). Prices and closes ADR 2849; follows ADR 2848.
Clauses: ISO 32000-2 §7.10.5.3, §8.7.4.5.2. Rows: §7.10.5.3 and §8.7.4.5.2 stay `implemented`.
Instruments: `pdf-model/examples/type4_operator_census.rs` (the population), `raster-gpu/examples/fault_word_price.rs`
(the map).

## The clause

§7.10.5.3: "It shall be an error for the number of remaining operands to differ from the number of output variables
specified by Range or for any of them to be objects other than numbers." and "it shall be an error to overflow the
stack." Since ADR 2848 the processor's evaluator records each one it meets and the page says so; a §8.7.4.5.2 type 1
shading drawn on the device evaluates the program in a shader that records nothing.

## What the premise check found

ADR 2849 named the device route as one that could draw all three errors silently. Two of the three never reach it:
`raster-gpu`'s own `admit`, which every device program passes before a frame, refuses arms of different depths
(`UnbalancedBranches`, an `if` included), a count `copy`, `index` or `roll` computes (`DynamicStackCount`), a count of
outputs other than the `Range`'s (`OutputCount`), and a stack past 64 slots (`StackTooDeep`, far under the
evaluator's 1 000). What reached a device and could fault there was the second half of the first sentence: an output
slot one arm leaves a boolean in and the other a number (`SlotType::merge` makes it `Undecided`, and `admits` checks
only the count).

## The two numbers, taken

- **Remedy 2's population.** The census over the pdf.js suite, `doc/corpora`, `doc/corpora-own` and the corpus
  cache (67 447 paths, 7 725 naming `/FunctionType`, 7 350 type 4 functions): 33 type 1 shadings, 24 with one type 4
  function, and `Function::may_fault` sends **none** of them to the processor. Planted back — a shading whose program
  is `{ pop 0.5 gt { 1 } { true } ifelse }` beside one with `{ 0 } { 0.5 } ifelse` — it names the first and not the
  second.
- **Remedy 1's cost.** `fault_word_price` on the RX 580 (RADV, `DiscreteGpu` in the lock log), release, 40 rounds
  per arm interleaved, `function_based_shading.pdf`'s radial program over `turn_path`'s 1600×1000 window, load 13.9:
  a frame read back, as `turn_path`'s arm draws it, 4.39 ms minimum (4.61 median); the same and the word copied,
  submitted, mapped and waited for, 5.35 ms (5.52) — **0.96 ms a frame, about a fifth**, every frame that draws a type
  4 shading, faulting or not. Into a texture, as a window draws, the two arms are not separable (0.58 ms against
  0.45 ms, the word arm the quicker): `Device::render` there returns before the device finishes, so the map's wait is
  hidden in whatever the frame overlaps, which a headless run cannot say. The shader's own `atomicOr` is unpriced; it
  would run only on a faulting pixel.

## Decision

Remedy 2. `Function::device_program` hands a device only a program `Function::may_fault` clears, and the processor's
grid — whose every cell is asked, and whose faults `Interpretation::function_faults_after_drawing` reads — draws the
rest. It costs nothing a frame pays, moves no shading of the corpora, and needs no shader change; remedy 1 would put a
binding on every type 1 shading's pipeline and a map on every frame that draws one, to report a fault no corpus
document has.

## The construction

`faults_excluded` is `provable_error`'s forward walk asking the opposite question — whether *no* run errs — and it
keeps at a join what that walk discards: each slot keeps what both arms agree on (`joined`), and an integer against a
real is held as a number. So the depth is exact at every instruction of every path; the walk proves no fault where
every count is a known value, both arms of every conditional leave one depth, no push meets `MAX_STACK`, and the end
holds `/Range`'s count of slots each certainly a number. It errs only towards "may fault", which costs the device
lane and nothing else. Path enumeration was tried first and abandoned: `pi_seven_segment.pdf`'s program is a chain
of `ifelse`s, and its paths double with each.

## Held by

`function.rs::a_program_some_input_makes_fault_is_not_handed_to_a_device`: four programs some input makes fault
(two count errors, a boolean output, an overflow inside an arm) and one computed count are refused a device and,
evaluated by the processor, record their fault; four programs whose arms join numbers are handed over.
`the_witness_pages_program_keeps_no_inexact_operator` still lowers `pi_seven_segment.pdf`. Planted back — the
condition dropped from `device_program` — the first test fails.
