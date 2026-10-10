# 1810 — What a run keeps is charged, and the budget is half the worker's address space

Asked by the project owner on 2026-10-10 (ADR 1809's request: "as long as it isn't a resource
problem"). Session 1486. Status: **accepted** and **built**. Amends ADR 1507's value and what it
charges. Context: ADRs 0607, 1507, 1809. Code: `crates/pdf-model/src/content/list_budget.rs`
(`MAX_LIST_BYTES`, `Interpreter::charge_painted`, `Interpreter::charge_readback`), `content/run.rs`
(the painting arms). Tests: `tests/hostile_budgets.rs`
(`a_form_read_again_and_again_charges_what_it_keeps_every_time`,
`a_form_cycle_through_a_tiling_cell_is_refused_by_its_list_bytes`).

## 1. The premise that did not hold

The brief's hypothesis was that the list's byte budget alone bounds a runaway stream's memory once
the operator count goes. It did not, in two ways, and both are measured.

- **Two things a run keeps were charged to the count, not to the bytes.** `list_budget.rs` said "a
  path is not charged: its segments each cost an operator", and the readback — text, one placement
  per code, one span per marked-content section, the `/Alt` and `/E` strings a span copies out of
  its property list — was charged to nothing. With a count, a form re-read a million times paid for
  its path and its spans in operators; without one it pays nothing, so a fan-out of `Do`s builds
  paths and copies strings without bound while the list's commands stay cheap.
- **The owner's own drawing sat 1.4% under the old 512 MiB.** `examples/display_list_census
  --stages` on `demo_heavy_engineering_A1.pdf`: 4 408 405 commands, **504.7 MiB charged**, 87 clips,
  1 256.0 MiB interpretation peak. So removing the count without touching the bytes would have left
  the page admitted by a margin the next drawing of its kind would not have, and charging its paths
  honestly would have refused it.

## 2. The decision

**Charged now**: a painted path's segments, at the painting operator (`f F f* S s B B* b b*`), once
however many commands share it — a glyph's outline is built once per font and never comes through
there; and the readback's growth, at the list's existing slow path every 64 operators, each part by
its entries' size and a described span by its strings too. Tiling copies keep their charge.

**`MAX_LIST_BYTES` is 2 GiB, half of the confined interpreter's `INTERPRETER_ADDRESS_SPACE_LIMIT`
(4 GiB).** That ceiling is the machine's answer to "a resource problem": a list that fits under it
can be drawn, one that does not kills the worker. Half leaves the other half for what lives beside
the list at its largest — the frame or the encoded list that crosses to the host (ADR 0607), or the
second interpretation a purity check keeps. The owner's page charges **740.1 MiB** with its paths
counted (interpretation peak 1 257.8 MiB), 36% of the bound.

## 3. What it moves

Over 958 first pages of `doc/pdf.js/test/pdfs` (the corpus gate's population), the census before
and after this change agrees on every page's command count and on whether anything was refused:
zero differences, so no frame of the six arms can move by this change. The charges rise (the census's
decades of bytes shift upwards) and nothing reaches the bound.

## 4. The cost

A hostile cycle is now stopped at 2 GiB rather than 512 MiB: `a_form_cycle_through_a_tiling_cell_is_refused_by_its_list_bytes`
takes 16.0 s under the `dev` profile and the `hostile_budgets` run peaks at 3.19 GiB; the `page` fuzz target keeps two interpretations, so such a unit can hold two lists of up to
2 GiB each against its `-rss_limit_mb=4096`, and that OOM report is this bound's and not a defect. A
path under hidden optional content is charged and then not kept, which errs towards refusing
sooner. What is still bounded only by the stream's bytes is the open marked-content stack of one
stream (`Marked` per unclosed `BMC`/`BDC`); it was bounded by the count at forty million entries
before, which was no tighter a guard, and it is the next thing to charge.
