# 1644 — A shading's functions are parsed once an interpretation, whatever the conversion

Session 1404. Status: **accepted**. Carries out ADR 1632 section 5's second item. Amends nothing.
Context: ISO 32000-2 §7.10, §8.7.4.5, §11.4.7; `CLAUDE.md` principle 2's rule that an optimisation
is justified by a benchmark and explained by a comment, and principle 3's budgets; ADRs 0069, 1632;
traps 13, 73, 92.
Code: `crates/pdf-colour/src/shading.rs` (new: `Parsed`, `Stated`, `MAX_PARSED_VALUES`;
`Cache::build`, `kind_of`, `ramp`, `axial`, `radial`, `mesh`, `function_based`, `FunctionColours`),
`crates/pdf-colour/src/function.rs` (new: `Function::held_values`).
Tests: `pdf-colour`'s `shading::tests` — a function parsed under one conversion builds what a parse
builds under another (three spellings of `/Function`, two planes, against a cache that never saw the
first); a reference to an array is not an array of one reference; a group past the bound is parsed
and not kept. The first two were watched failing with the key planted wrong twice: `/Function 8 0 R`
keyed as `[8 0 R]`, and every group under one key.

## 1. Counted first

Callgrind, `examples/turn_interpret` on `bug1721218_reduced.pdf` page 1, gates profile, one thread,
a run of two less a run of one (ADR 1632 section 1's method). **The turn is 1 357.6 M instructions;
`shading::Cache::build` is 405.0 M of it and `Function::parse_group` 145.3 M, 137 parses.** The file
states 41 shadings, each naming a function object of its own (qpdf's expansion, 41 `/Function`
entries, 41 distinct). `Cache::built` is keyed by the conversion, as it must be: the group's two
planes (§11.4.7) and the soft masks' groups each build the same shading under a conversion of their
own, and each build parsed its function again — a type 0 function's stream inflated and its samples
decoded every time. ADR 1632 priced this at about 2 ms on the black half alone; the count says the
chromatic run repeats parses too, and the whole repeat is 96 of 137.

## 2. Built

`Cache` keeps a memo of parsed `/Function` groups beside its builds, keyed by how the shading's
entry names its objects. **Exact by construction**: `Function::parse_group` reads the document and
the object and nothing else — no conversion, no resource dictionary, no graphics state — a reference
resolves to one object of one immutable document, and one cache serves one document (the interpreter
swaps it out for an imported page, §8.10.4). So the group a key names is the group a second parse
would make, and a `/Function` stated inline, which has no identity, is parsed as before.

- **Two spellings, two keys.** `/Function 12 0 R` takes the group from whatever object 12 is, one
  function or an array of them; `/Function [12 0 R]` takes object 12 as one function and refuses an
  array. Keyed as one list of identifiers the second would be answered with the first's group, which
  the second test above catches.
- **Bounded.** The memo holds at most 2^22 numbers (`Function::held_values`: samples, spline
  coefficients, coefficients, bounds, instructions, recursively), the most samples one type 0
  function may carry, so keeping functions costs a cache at most what one function at the parser's
  own limit costs while it is evaluated. A group past it is parsed and not kept: a parse, never a
  colour. A failed parse is not kept, for `Cache::build`'s reason.
- `FunctionColours` holds its group as an `Arc<[Function]>` shared with the memo, so a type 1
  shading built under two conversions holds its samples once.

## 3. What it bought

Callgrind, the same method: the turn **1 357.6 → 1 258.9 M** (−98.7 M, 7.3%); `parse_group`
145.3 → 47.0 M, 137 parses → 41, the file's own count and the least one interpretation can make;
`Cache::build` 405.0 → 306.5 M. The interpreter's own work outside it is unchanged (`run_reader`'s
self cost over a run of two, 224.03 → 223.92 M). At ADR 1632's 20 M instructions a millisecond that is about 5 ms of the
turn's `interp`. A page with no shading never reaches the memo, and an empty `BTreeMap` allocates
nothing.

The corpus gate's per-page digests on six arms against the batch's export of HEAD are in the
record of session 1404, with the clock that `turn_path` read.
