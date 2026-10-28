# 2528 — A calculator function under a redaction is served by a stitching function written over the producer's program

Status: accepted. Supersedes the paragraph of ADR 1363 that refuses "a calculator function serving a region-only
value" for a one-input function; the two-input refusal stands. Session 1847, a clause round of batch one hundred and
twenty-five. Context: the owner's question of 2026-10-27, "i would like to move all ledger rows to final states.
What is open, what is holding us up?", ADRs 1124 (the constant a destroyed value is cleared to), 1363, 1371.

## What ADR 1363 decided, and the premise under it

A §7.10.5 calculator function is a program, so no value of it is "the region's" by cutting its text, and ADR 1363
refused the page: "no finite re-expression holds its values exactly, and a program rewritten to omit the region's
values would be one this build authored". That is true of a rewrite of the program. It is not true of a function
that *contains* the program, and §7.10.4 supplies one: a stitching function (Type 3) applies a different function to
each subdomain of its `/Domain`, and Table 41's `/Bounds` and `/Encode` choose the subdomains and what each piece's
function is handed as input.

## The construction

The parameters the region alone carries are `used` less `kept` (ADR 1363's `Intervals`), spans of positive length.
`build_calculator` writes, as an object of its own where the producer's was one, a stitching function over the
calculator's own `/Domain`:

- one piece per span, whose function is the constant ADR 1124 clears to — a Type 2 function with `/C0` and `/C1` of
  zeros, one for each pair of the calculator's `/Range`, which it also carries so that the clip the program is
  subject to is the constant's;
- one piece per stretch between spans, whose function is the **producer's own calculator object**, named by the
  reference the shading names, and whose `/Encode` is the piece's own bounds, so the program is handed the input it
  was handed before. The program's bytes are not copied, rewritten or composed (the test reads them back);
- `/Bounds` the pieces' starts. A piece at either end of the domain leaves no program piece there.

Nothing here authors a program: the content of every mark outside the spans is the producer's text evaluated at the
producer's input, and the stitching dictionary is structure, in the sense the authoring exclusion draws (CLAUDE.md,
"Assembling documents from existing documents").

A calculator beneath another stitching function (the common case, §8.7.4.5.4's EXAMPLE 1) is reached through
`build_stitching`, which maps `used` and `kept` through the outer `/Encode`; the new stitching function is then the
outer piece's function, nested, and a reversed outer `/Encode` is handled by that mapping alone.

## Where it is not exact, stated

1. **Boundaries.** §7.10.4 gives each piece "closed on the left and open on the right", so a cleared span's left end
   is served the constant and its right end is the next piece's. `Intervals::less` keeps the ends a span shares with a
   kept span as kept, so the left end is one parameter value destroyed that need not be (more destroyed, never less).
   Under a reversed outer `/Encode` that one value can be the kept side's boundary. A parameter value is a set of
   measure zero in the shading's plane.
2. **Interpolation.** The reader evaluates §7.10.4's interpolation of the input in its own precision; handing the
   program an input one rounding away moved a grey by one level at some pixels outside the region in the tests
   (`held_outside_within(.., 1)`), which is why those tests state a level.
3. **Numbers.** A reader narrows each bound to `f32`. A program piece that narrowing makes empty between two spans is
   folded into them (more destroyed); a span it makes empty is refused by name rather than left standing.

## Bounds and what is refused

- **The whole tree is held to the reader's budget** (`build_function_tree`, `count_function_nodes`). The viewer's
  function reader builds at most `pdf_model::function::MAX_FUNCTIONS` (4096) functions for one root and counts a
  function once for every time a reference to it is followed. A bound on each calculator is not a bound on the tree:
  a producer's outer stitching function of k pieces that all name one calculator makes k calculators, each stitched
  over its own spans, and k = 20 with 2 500 placements already wrote a tree the reader refuses, while k = 200 with
  20 000 placements and 64 outputs reached 429 MiB resident and 26 MB of output (the Opus validator's probe) and a
  shading that drew nowhere, outside the region too. The producer's tree is counted first (every dictionary or
  stream, references followed each time, depth at the reader's limit of 8, the count stopping once past the
  budget); every stitching function written over a calculator spends its piece count from what the producer's tree
  leaves, because it replaces one node by the stitching function and its pieces; past it the page is refused by
  name, before anything is built. Both constants are the reader's own, made public in `pdf-colour`, so the
  two cannot drift. A producer's tree already past the budget leaves no room, and the viewer refuses it whole
  regardless. The test builds an outer stitching of 2047 pieces over one calculator (4095 functions), reads the
  output back with the viewer's own reader and finds the region destroyed; 2048 would make 4097 and is refused.
  Against HEAD's six arms nothing moves: no page is drawn by this change. The validator's probe at both its
  settings is now refused, at 37 MiB and 76 MiB resident (debug); the bound test peaks at 76 MiB.
- **At most 1024 pieces for one calculator** (`MAX_CALCULATOR_PIECES`). The region count cannot reach it alone (the
  path cut refuses a page at 512 polygons), but one shading placed many times can: the test places it 511 and 512
  times, the first accepted and the second refused by name. Its memory: 35 MiB in the debug profile with 64 outputs,
  against 13 MiB for a test that refuses at once (`ru_maxrss`).
- **At most 64 outputs** (`MAX_CALCULATOR_OUTPUTS`, the reader's `MAX_VALUES`). A `/Range` of a million entries would
  be copied into every cleared piece, and the reader reads no function of more.
- **Refused by name** (the page, as before): a calculator that states no `/Range` (Table 38 requires it for Type 4),
  or no two increasing `/Domain` numbers; and a **two-input** calculator of a Type 1 shading whose domain the region
  meets. The last is conforming input — §8.7.4.5.2 admits a function of two inputs — and is therefore a departure,
  counted as the row's second: a stitching function has one input, no function of two is divided by it, and a
  construction for two inputs would be a lattice of programs, which this ADR does not argue.

## What the row is after this

§12.5.6.23 stays `departed` on two refusals. One is the `JPXDecode` family: a component past the twenty-four bits the
decoder reads exactly, and a deep raw codestream of two, five or more colour components that states no colour box
(ADRs 1371, 2470, 2480; the brief named the first only, the second is a refusal in `redact/images.rs` and `deep.rs`
that this ADR does not touch). The other is the two-input calculator function above.
