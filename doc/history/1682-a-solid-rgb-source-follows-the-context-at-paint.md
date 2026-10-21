# 1682 — A solid RGB source follows the context at paint

Session 1682, slot 3 of batch 105; ADR 2198 and ADR 2199; same model.
Started 2026-10-21T20:50:54Z; scoped work ended 2026-10-21T23:56:11Z; 11117 seconds.
No owner decision needed. §11.7.5.3 remains partial for pattern tint and profile bypass.

**Contract.** Retain resolved RGB components in graphics state and convert under the
pair/target at painting, including solid stencils. q/Q and inherited Forms keep their
source; changing space retires it. Preconverted uncoloured tiling tint clears it.
Image/solid equivalent orders now produce the clause-derived [52,62,79,255].

**Design.** Two fixed interpreter slots retain RGB bits, complete Conversion and
pre-alpha Color. Borrowed and owning contexts share one seven-field key; misses
retain identities, selection seeds existing conversion, alpha is applied afterwards.
No manual invalidation graph. Non-RGB conversion and profile bypass stay unchanged.
Pattern tint still omits current conversion inputs; its standalone probe and exact
next-round contract are ADR 2199, not an assertion blessing the wrong pixel.

**Evidence.** Fixed baseline 020a86f3 fails five of nine stateful cases; candidate passes nine.
Target entry/return, q/Q, pair changes, stencils, space changes and warm/cold alpha
are covered. Both full-field borrowed-key tests pass, including function identity.
Unconditional conversion and first owned-key cache timings were rejected; all logs
remain. Short third samples were bimodal; a load 16.935 attempt collected no samples.
Twelve CPU2-pinned balanced pairs then pass all 48 unchanged load/GPU guards, exit 0.
Median paired deltas per 100k paints: RGB 1.741 ms/default CMYK 2.2885/stated 2.592/
alternating 4.1785; ratios 1.08475/1.03922/1.04387/1.01597. ADR 2198 retains all ranges.
These whole-interpretation costs are accepted for correctness, not performance neutral.

**Gates.** Scoped conformance/fmt/Clippy/rustdoc exit 0; model 2134 passed, 21 skipped.
Nine paint and two key tests pass; colour all-targets Clippy exit 0. Pinned hold 339s,
peak 0.02 GiB. Pictures show only the stale blue patch become its clause-derived colour.
Final combined corpus/six arms/launch remain integration gates owned by the batch.

**Paths.** content.rs plus colour/pattern/cell/image/run; colour Conversion/context
export and generation test; black_generation_paint test and black_generation_order
example; ADR 2198/2199, ledger 11.7.5.3, frontier todo 01/65. No golden/fixture edits.

**Proposed habit.** Keep source inputs at a mutable context boundary; prove the full
cache key and measure stable whole workloads before adding further invalidation state.
