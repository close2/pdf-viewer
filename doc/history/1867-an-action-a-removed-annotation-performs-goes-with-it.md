# 1867 — An action a removed annotation performs goes with the annotation

Batch 127, slot 7 (archive, robustness), one session. ADR 2568; ADR 2569 and Q733 unused. Part 2
(§8.6.8's `/BG` and `/UCR` in an uncoloured figure) not started: part 1 closed past call 60.

**Built.** `archive/actions.rs`: `prepare` takes the objects the section 6.3.1 subtype rows remove
(`prepare.rs` passes `ForbiddenAnnotations::at`) and walks none of those holders, so nothing in a
removed Screen's tree is listed or edited. A bounded `Reach` walks the validator's roots (`/A`,
`/AA` entries, `/OpenAction`, the `/JavaScript` tree; each object once, 32 `/Next` links) from
the removed and from the kept holders; `met_by_the_removal` (called beside `met_by_the_replacement`
in `mod.rs`) gives an action-type, `/AA` or widget-entry row whose `named` set lies wholly in
removed-only reach the subtype rows' decision, and only where that removal proceeds — a refused
removal leaves the row its own decision, so its refusal is reported once.

**Premise.** The brief's "one filter on the walk's holders" held for the walk but not for the loss:
the loss is charged by the decision per requirement, not by the walk, and the validator names an
action by its own object and a shared one once, so the decision needs the two reaches. A first
draft that took ADR 2442's "the stopping decision at every row" added the subtype row's refusal
under the action row in three refused census documents; taken back.

**Finish line.** A Screen's Rendition at 4, and its `/AA` at 2b and 4, convert under
`forbidden-annotation` alone, veraPDF PASS at each; the CLI converts a 130-Screen probe HEAD
refuses (exit 4 to 0, veraPDF PASS at 4). **Plants**: the same action on a kept Link still costs
the loss; a shared action object is the kept holder's (fails with the kept reach dropped); a chain
of 33 is charged and 32 is not (fails with the overlong guard dropped); veraPDF FAILs both sources.
Memory, 4000 Screens of 31-link chains (124 000 actions, past `MAX_NAMED`, so charged as before):
`getrusage` high-water 234 MiB beside HEAD's 236 MiB.

**Census.** Without and with the CJK pack, every census line equal to HEAD's; no document moves —
the two census profiles answer the action rows with `discard`.

**Gates.** In the export (the shared build broke on a sibling's mid-edit, habit 138): `cargo test
-p pdf-transform --test archive --test profiles` exit 0 (485 and 12 passed); `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-transform --all-targets` exit 0; `cargo fmt --check -p
pdf-transform` exit 0; `archive_corpus -- --ignored` exit 0 (7 passed); census twice each lane,
exit 0. In the worktree: `cargo check --tests -p pdf-transform` exit 0; `--bin cited` exit 0;
`--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 0.
