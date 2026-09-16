# 1134 — the transfer function is a colour primitive, and it leaves the interpreter

ADR 1131 step 1: relocate the §10.5 `Transfer`/`Stated` cluster out of `pdf-model`'s content
interpreter so the colour stack stops depending on it. Groundwork for the `pdf-colour` extraction,
not the extraction (steps 2–4 remain).

## The coupling, re-measured
`shading.rs:24` and `mesh.rs:50` did `use crate::content::Transfer`; `Transfer`
(`content/ext_gstate.rs`) is `[Option<Arc<function::Function>>; 3]`. So the colour stack depended on
the content interpreter for a type built on the colour stack's own `function::Function` — the future
crate cycle ADR 1131 named. (A second, unrelated `pdf_render::Transfer` in `soft_mask.rs` is a
different type and untouched.)

## What moved
`Transfer` (with `read`, `apply`), the `Stated` answer, and their `#[cfg(test)]` tests now live in a
new `crates/pdf-model/src/transfer.rs` (`mod transfer;`), a colour-stack sibling of `function`.
`read` and `Stated` widen to `pub(crate)`; two `pub(crate)` helpers — `from_channels`, `channel` —
let the interpreter build and read one without the private field. `TransferState`, `Component` and
`compose` stay in `content/ext_gstate.rs`, now depending on `crate::transfer`. `content::Transfer`
and `ext_gstate::Transfer` stay as re-exports, so `pattern.rs`, `transparency.rs`, `image.rs` and
every external consumer are unchanged. `shading`/`mesh` repoint to `crate::transfer::Transfer`.

## Cycle check
`colour`, `icc`, `function`, `shading`, `mesh`, `transfer` now hold **zero** `use crate::content`
(remaining mentions are doc prose, matching the stack's code-span convention). `cargo metadata`
resolves (exit 0); no crate-level dependency changed — the move is intra-crate.

## Gates
- `cargo build -p pdf-model` — exit 0; `cargo clippy -p pdf-model --all-targets` — exit 0 (lone
  warning is a sibling's untracked example).
- `cargo nextest run -p pdf-model` — 1516 passed, 0 failed; the moved unit test runs as
  `transfer::tests::a_transfer_function_maps_every_colour_component`.
- `raster_golden` — held 974, **moved 0**.
- `cargo test -p conformance` — exit 0, after repointing the §10.5 row's one stale test citation
  from `content/ext_gstate.rs` to `transfer.rs` (the only ledger touch; the code-list refinement is
  step 4's rewrite). Scoped to `pdf-model` as five siblings share the worktree.
