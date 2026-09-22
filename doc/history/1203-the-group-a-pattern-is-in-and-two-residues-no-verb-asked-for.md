# 1203 — The group a pattern is enclosed in, and two residues no modal verb asked for

2026-09-22. ADRs 1243 and 1244.
Rows: §11.3.6 and §11.4.8 `partial` → `implemented`; §11.6.7 built and narrowed; §11.4.3,
§11.6.6 and §11.4.7 narrowed to their exact residues.

## What moved

**§11.6.7's implicit group is now the one the clause names.** A tiling pattern's cell was given an
isolated group wherever the *mark's* blend mode was not Normal, which was never NOTE 1's condition —
it was what ADR 0237's collapse could draw. ADR 1107 removed that premise: `render-cpu` performs
§11.4.4's result step for itself under any mode at the `Do`, and the other two backends refuse the
combination by name. `compose_tiling`'s condition is now
`inside_knockout || !any_command(&parts, &command_blends)`, and what stays reported is a backdrop —
§11.4.6's NOTE 6, a cell inside a knockout group whose own initial backdrop is not transparent.

**And the shading pattern's implicit knockout group is unobservable rather than unbuilt.** It holds
at most two elements, Table 77's wash and the shading, and §11.6.7's first bullet initialises the
blend mode, the soft mask and the alpha constant to their defaults with `PatternInitial::augmented`
admitting none of them back — so both are opaque and Normal, §11.3.3 gives such a source `Cs`
against any backdrop, and the two constructions coincide at every point.

**Two rows were `partial` on a residue that is not a requirement.** §11.3.6 states exactly one
`shall` — the result colour normalised by the result alpha — and all three evaluators execute it,
held with no slack at all 256 mask values; *the formula is a library's* is about authorship.
§11.4.8 states no `shall` and no `should` at all: it restates §11.4.4's and §11.4.6's formulas, and
those rows carry them. That is §11.3.8's shape one clause down.

**§11.4.3 was read the same way and did not move**, which is the useful half: its deferral to
§11.4.4 had expired, but the clause has a residue in its own sentence — a group's colour, shape and
opacity treated as one object's, where a premultiplied raster carries the alpha and not the shape.

## Measured, and touched

Both fixtures are hand-built and say so: all 122 corpus tiling paints leave the three transparency
parameters at their defaults, so no corpus page builds the group at all (trap 8). Each is calibrated
by planting its own construction away (trap 13) — the old isolation condition, and
`Shading::with_colours`' mapping of the wash. Files: `crates/pdf-model/src/content/pattern.rs`,
`crates/pdf-model/tests/tiling.rs`, `crates/pdf-model/tests/transparency.rs`,
`doc/conformance/ledger.toml`, `doc/todo/23`, `doc/todo/65`.
