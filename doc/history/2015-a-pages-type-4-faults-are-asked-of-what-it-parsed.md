# 2015 — A page's type 4 faults are asked of what it parsed

Session 2015, a single ledger round, the ledger lane. Row 7.10.5.3 (parents 7.10.5, 7.10); ADR 2833's construction; ADRs
2848, 2849; `doc/reviews/1983-specification-audit/fixes.md` section 4.

**The slice.** ADR 2833's first three steps, in its order; the fourth, the GPU evaluator, is named, unpriced (ADR 2849).

**The premise.** Held: `Faults::clone` copied the bits, and only shading ramps and a soft mask's `/TR` asked
`evaluation_fault()`; effort was the only obstacle, so no Q846. One part did not hold: ADR 2833's walk per kind. The
interpreter resolves a function-bearing resource in at least twelve places and keeps few, so the cell is registered
where every route meets, `Function::parse_within`, with a scope `interpreted` and `replace` open around the page (ADR
2848).

**Built.** `Faults(Arc<AtomicU8>)`; `function/watch.rs` (`watching`, `Watched`, folding cells whose functions are all
dropped); `Unsupported::FunctionEvaluation { fault }`, pushed once per distinct fault only where a cell holds one (trap
11); `Interpretation::functions` and `function_faults_after_drawing` for the type 1 grid; viewer-core's sentence and the
corpus gate's `Whose::TheFile` arm.

**Census** (a Sonnet subagent, every `Function::eval` caller and its wrappers): every evaluation is inside the
interpretation, images included (conversion is eager), except §8.7.4.5.2's type 1 grid, which a backend draws later.

**Tests.** `pdf-colour`: a clone's fault is the function's; a scope sees its parses, hands them outward, reads a cell
after closing, and folds a thousand dropped parses to under 64 cells. `pdf-model/tests/function_faults.rs`, each a
program erring for some inputs only: tint transform by `scn` (0.2 errs, 0.9 not), an image's tint transform, `/TR`,
`/BG` in a `DeviceCMYK` page group (overflow past 1 000), a type 4 mesh's `/Function`, a type 1 shading asked after
`sampled_at`. **Plant**: removing the registration fails eight (six here, two in `pdf-colour`; the clone test does not
rest on it); restored, all pass.

**Found.** `cs` converts §8.6.6.4's initial tint 1.0, so the tint fixture errs below 0.5; and an 8-bit or narrower
one-component image converts every level (`unpack::palette`), so its report can name a level no sample uses — written
down in the row and ADR 2848 as the one over-statement.

**Left.** A host asking `function_faults_after_drawing`; the GPU's `device_program` (ADR 2849's two remedies, needing a
`turn_path` clock arm and a corpus count); the palette over-statement. 7.10.5.3, 7.10.5, 7.10 stay `implemented`.

**Gates.** fmt `--check`, clippy `-D warnings --all-targets` on pdf-colour, pdf-model, viewer-core: exit 0; nextest of
the three: 3376 of 3376; fuzz `cargo check --bins`: exit 0; conformance after `git add -N`: exit 0 once this paragraph
was written; rustdoc `-D warnings` private: exit 0; `state.sh docs` 0; tier 2 `corpus` 101 and `raster_golden` 101 as
HEAD's (the same pair joined, 23 moved), the censuses 0, and `drawn_while_read` 101 until the merge gave `Watched` a
`Debug` of its faults alone (it printed cells, which a page read in pieces holds more of); then 0.
