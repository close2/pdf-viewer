# 2848 — A page's type 4 faults are asked of every function it parsed

Round 2015, the ledger lane.
Status: **accepted and built** in `pdf-colour` (`function.rs`: `Faults` shared across clones; `function/watch.rs`:
`watching`, `Watched`) and `pdf-model` (`content.rs`: `interpreted`, `replace`, `report_function_faults`;
`content/report.rs`: `Unsupported::FunctionEvaluation`, `Interpretation::functions`,
`Interpretation::function_faults_after_drawing`).
Builds ADR 2833's first three steps, and departs from its second in the one way set out below.
Clauses: ISO 32000-2 §7.10.5.3, §7.10.1, §8.7.4.5.2.

## The clause

§7.10.5.3: "It shall be an error for the number of remaining operands to differ from the number of output variables
specified by Range or for any of them to be objects other than numbers." and "it shall be an error to overflow the
stack." A program that meets one on every run is refused at parse (`provable_error`); one that meets it for some inputs
answers, records the fault on the function (`EvaluationFault`), and was reported only by shading ramps and a soft
mask's `/TR`.

## What is built

1. **`Faults` is an `Arc<AtomicU8>`**, so every clone of a function shares one cell: a clone is the same function of the
   file, and the instance asked need not be the one that evaluated.
2. **The cell of every type 4 function is registered where it is parsed**, with the innermost `watching` scope open on
   the thread (only type 4 can fault). `interpreted` and `replace` open the scope around the whole page — planes,
   annotations, soft masks, separations — and at the end push one `Unsupported::FunctionEvaluation { fault }` per
   distinct fault met, only where a cell holds one (trap 11).
3. **The census of evaluations after `interpret` returns** (round 2015, every call of `Function::eval` and of the
   wrappers that reach it): every one is made inside the interpretation — tint transforms through `Tints::eval`,
   images (whose conversion is eager, its rayon bands joined before the decode returns), `/TR` sampled into a table,
   `/BG` and `/UCR` through `to_cmyk_at`, ramps and mesh colours at build — except §8.7.4.5.2's type 1 grid, which the
   display list carries as `FunctionColours` and a backend evaluates at its own resolution. The interpretation keeps
   its `Watched`, and `Interpretation::function_faults_after_drawing` reads the same cells after a backend has drawn.

## Why a scope at the parse rather than a walk per kind

ADR 2833 proposed an `evaluation_fault()` walk per kind and the interpreter asking "each resource it resolved". The
interpreter resolves a function-bearing resource in at least twelve places (`cs`/`CS`, `scn`'s pattern and shading,
image and inline-image spaces, JPX's declared space, image masks, `gs`'s `/TR`, `/BG` and `/UCR`, the soft mask's group
space and `/BC`, the page group's blending space and its press, overprint's spot check) and keeps few of them; each
would need a list kept beside it, and a place missed is a fault never said — the silent route trap 5 is about. Every
one of those routes parses through `Function::parse_within`, so registering there is complete by construction.

What it costs: one shared cell per type 4 function parsed, nothing per evaluation. A cell whose function and clones are
all dropped is final, so a scope folds those into one byte when its list doubles; the list is bounded by the type 4
functions alive at once (`a_watch_holds_the_live_functions_and_the_dropped_ones_bits`).

What it does not see, stated rather than hidden: a function parsed on another thread than the scope's, and one parsed
before the scope opened. The census found no parse inside the interpreter's parallel sections (they evaluate), and no
cache that outlives an interpretation holds a parsed function; `replace` carries the cells of the run that made its
checkpoint (`Replacement::functions`).

## What is left

- **An image's table of levels.** A one-component image of eight bits or fewer converts every level its depth holds
  (`image::unpack::palette`), so the function is evaluated across its domain and a fault at a level no sample uses is
  said. That is the one route where the report states more than the drawing; narrowing it means converting only the
  levels present, a change to a measured hot path that a round of its own prices.

- **The type 1 grid's caller.** `function_faults_after_drawing` is the question; no host asks it yet, because the
  backends that draw the grid (`render-cpu`, `render-raster`, `pdf-render`'s own) carry no report, and a viewer's
  report is written from `unsupported`. The host that draws and then reports is the place to ask it.
- **The GPU evaluator** (ADR 2833's last section): `device_program` hands the program to `render-raster`'s shader,
  which records nothing. ADR 2849 prices the two options.
