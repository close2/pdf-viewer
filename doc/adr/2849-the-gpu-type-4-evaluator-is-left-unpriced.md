# 2849 — The GPU's type 4 evaluator still records no fault, and what pricing its two remedies needs

Round 2015, the ledger lane.
Status: **deferred** — named, not built; the round that wrote it spent its budget on ADR 2848.
Clauses: ISO 32000-2 §7.10.5.3, §8.7.4.5.2.

## What stands

`Function::device_program` lowers a type 4 program for `render-raster`'s shading shader, which runs the stack machine
per pixel and records nothing. Since ADR 2848 every CPU evaluation of a type 4 function is asked, including the type 1
grid a CPU backend draws after the interpretation; a type 1 shading drawn on the device is the one route on which a
§7.10.5.3 error for some inputs is drawn from the shader's stand-in and said nowhere.

## The narrowing this round found, which either remedy rests on

A program with **no conditional, whose stack counts the walk can read**, cannot fault at run time: `provable_error`'s
walk then models its depth and each slot's type exactly on its one path, so a count or boolean error is refused at
parse and an overflow is proven where it lies outside every arm (`unconditional`). The faults that depend on the input
are confined to two shapes: a program with `if` or `ifelse` whose arms leave different depths or types or push past
`MAX_STACK`, and a `copy`, `index` or `roll` whose count is computed, where the walk abandons (`operand_demand`).

## The two remedies, and what pricing each needs

1. **A fault word read back after present.** A storage buffer bound with the shading, `atomicOr` where the program's
   end count or type check fails or a push overflows, mapped after the frame. Its cost is a map on the present path
   per frame that drew a type 4 shading, measured with the clock gate's `turn_path` arm against a page that draws one
   (`function_based_shading.pdf`), and a shader change that every type 1 shading pays.
2. **Refusing `device_program` where the walk cannot prove the three errors absent**, and drawing that shading from
   the CPU grid, whose evaluator records. By the narrowing above the refusal can be confined to those two shapes;
   its cost is the population of type 1 shadings carrying such a program — a count `examples/type4_operator_census` can be extended to make over the
   corpora — each moved from the device to the processor.

Neither number was taken here, and no guess at either stands in for it. The second is the cheaper to take; the round
that takes both builds the remedy the numbers favour.
