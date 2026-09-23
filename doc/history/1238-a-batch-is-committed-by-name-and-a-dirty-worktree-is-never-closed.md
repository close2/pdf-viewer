# 1238 — A batch is committed by name, a dirty worktree is never closed, and raster's `§` means ISO

Date: 2026-09-23. Branch `batch-1233-1238`, shared worktree. ADRs 1313, 1314.

## The merge guard (ADR 1313)

`tools/batch.sh close` refuses a worktree holding anything uncommitted outside `scratchpad/`,
lists it, has no `--force`, and refuses a branch the worktree is not on. `tools/batch.sh commit
<message-file>` stages that population by name (a staged deletion counted, never named to
`git add`), prints population against index, refuses on any difference, a staged `scratchpad/`
path or a submodule staged as a blob, and never fast-forwards. `doc/todo/02` section 8 step 5 says
the three commands run separately; step 8 is the transcript-replay recovery.
`cargo test -p conformance --test batch` holds both against a throwaway repository. Planted: the
script of `058e77dd` closes a dirty worktree and its file is gone; this one refuses and it stays.
Against the live worktree (a copy with the removal commands replaced by `echo`), `close
batch-1233-1238` refused, exit 1, naming 227 paths; `close --force` refused, exit 1.

## The raster sweep (ADR 1314)

`cargo run -q --release -p conformance --bin cited` printed, before: `189 with no row at all`,
`135 of the no-row pair(s) lie under raster/`; after: `65` and `11`. 407 comment lines in 161
files under `raster/` changed; every non-comment line of each was asserted unchanged before the
write, and `rustfmt --check` passed on each. Besides the no-row pairs, `§6.1`–`§6.3` (brief
sections landing on ISO clause 6) and `§11.1`–`§11.5` used as the brief's questions were
rewritten; `§11.2`'s NOTE and every `§11.5` beside a soft mask are ISO's and stayed. The eleven
left are ten string literals (code, round 1235's) and `§11` in `scene/command.rs`, which is ISO.

The sweep cost sixteen blockquotes their attribution (the quotation gate failed), so
`citation::section_in_prose` now reads "`<our document>` section N" as one; another standard in
the same spelling still does not attribute. `tools/state.sh cited` prints the raster listing.

**The date.** `pdf_model::xmp::spelled_date` is `pdf_syntax::Date`'s `Display`; a test sweeps 25 dates (`Z`,
east, west, none, year only) for byte equality.

## Left

`the_ledger_agrees_with_the_standard_and_with_the_tree` fails on §F.3.1/§F.3.5's
`crates/pdf-transform/tests/linearize.rs` test name — round 1236's, mid-edit. Clippy on
`pdf-model` is blocked by `pdf-colour/src/colour.rs` (round 1237's, mid-edit).
