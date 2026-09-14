# 1038 — Two machines held to each other, within what the survey walks

Date: 2026-09-14. Branch: `batch-1038-1043`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds. ADR: [1055](../adr/1055-two-machines-held-to-each-other-within-what-the-survey-walks.md); answers `A61`'s `Owes:` line.

## What was built

A corpus-scale gate, `crates/pdf-archive/tests/cross_check.rs`, that walks every page of the
validator's corpus with both state machines and asserts that within every content stream both ran,
the survey's named-resource selections and the interpreter's are the same set, both ways — plus a
reach check for a stream the interpreter ran that the survey never walked. Each machine records
through `pdf_model::content::ledger::Ledger`, written at each machine's one lookup site, with the
nested-stream sites naming route and object. A ledger of lookups, not the observer A61 declined.
Tier 3 in `doc/todo/02` §2, `tools/batch.sh gates` and `tools/state.sh archive`.

## What it found, and the calibrations (trap 13)

Zero disagreements over 12 542 paired places and 13 547 selections; every unpaired place is the
census the module comment predicts (unshown Type 3 glyphs, appearance states not shown, one chain
nested past the survey's depth, one constructed appearance). Three one-line plants, each reversed
by its exact inverse: the survey reading every form against its invoker's resources — **269
disagreements, exit 101**, each named by document, page, stream path, operator ordinal, category,
name and both outcomes; the survey walking no form — **83**, from the reach check; the interpreter
reading every soft mask's group against nothing — all 13 soft-mask selections flip to "nothing" in
the census and **0 disagreements, exit 0**, which is A61's scoping measured.

## Gates, and one moved golden that is not this round's

Tier 1 on the two crates is clean; the workspace `-D warnings` and `fmt --check` lines fail only in
siblings' files mid-edit. Tier 2 (rule 2) is green but for `raster_golden`, which moves
`bug1721218_reduced.pdf p1` list-only — fifty transparency groups, no pattern, shading, Type 3 font
or soft mask — on a tree where a sibling has 536 changed lines in `transparency.rs` and this round
builds no display-list command; the merge attributes it (rule 5). ADR 1055 §5 records two
`/Resources` fallback readings the corpus cannot witness.

## Files touched

`crates/pdf-model/src/content/{ledger,resources,run,xobject,pattern,text,annotations,transparency}.rs`,
`crates/pdf-model/src/{content,type3}.rs`, `crates/pdf-archive/{src/survey.rs,tests/cross_check.rs}`,
`doc/todo/02-every-round.md`, `tools/{batch,state}.sh`, `A61`'s `Owes:` line, ADR 1055, this file.
