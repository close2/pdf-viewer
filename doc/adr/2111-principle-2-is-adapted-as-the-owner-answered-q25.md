# 2111 — Principle 2 is adapted as the owner answered Q25

Session 1637, batch ninety-nine's slot 7, a docs round. Status: **accepted** and **applied**.
Context: `doc/questions/Q25` (round 921) and the owner's `A25` (2026-09-06); ADRs 0885, 0890,
0891 (the launch path measured), 1023 (a correction is applied, not appended); ADR 2110 (the
reading set compacted, which found this owed).

## 1. What was owed

Q25 named six sentences of `CLAUDE.md` principle 2 that the tree does not bear out and asked, of
each, whether it was a requirement still owed or a description that had decayed. The owner
answered on 2026-09-06: "For CI & Stack (1-3) we adapt the CLAUDE.md" and "for the startup
prohibitions (4-6) soften the wording ("deferred unless needed for page one",...)". Round 937
recorded that the answer asks for a change of wording and that acting on it was a later round's;
no round made it, so for forty-four days every round read six sentences the owner had already
ruled on. Each was checked again against the tree before it was changed.

## 2. The six, as changed

1. **"Perf gates run in CI … A regression fails the build."** `.github/workflows/ci.yml` still has
   no performance job, and GitHub's runners hold no clock worth judging. The four figures are held
   by the merge's `launch_path` and `turn_path` lines (`doc/todo/02` §2), each with bands under
   `doc/checks/`. Now: "Perf gates run at every merge: … (`launch_path` and `turn_path`,
   `doc/todo/02` §2). A regression fails the gate."
2. **"Targets are set once Spike A gives a real baseline, rather than invented now."** Spike A
   finished long ago on determinism (`crates/render-cpu/tests/headless_render.rs`), and the targets
   exist: `doc/checks/launch-path.toml` holds a band on each figure. Now: "… gates with numbers
   attached, measured with a cold page cache; `doc/checks/launch-path.toml` holds the band on each."
3. **"GPU offload (vello/wgpu)".** The window presents with quorra (`render-raster`, ADR 1831);
   vello's `render-gpu` is the cross-backend comparison (`doc/stack.md`). Now names both, in that
   order.
4. **"A 500-page document must open no slower than a 5-page one."** False of the open by about
   fifty times and not repairable by laziness (§7.5.6 makes the whole cross-reference data the price
   of any object; ADR 0891), true of the time to first page. Now: the open "defers what page one
   does not need: a 500-page document must reach its first page no slower than a 5-page one".
5. and 6. **"No system font enumeration, no full page-tree walk"** were flat prohibitions beside
   the general rule that qualifies them. Now the general rule leads and the two carry its
   qualifier: "No system font enumeration unless page one names a font it does not embed, no full
   page-tree walk unless page one needs it".

The owner's own sentences in principle 2 — the GPU-first choice and the 2026-10-12 quotation —
are unchanged.

## 3. What follows elsewhere

Four maintained citers quote the retired wording and are outside this slot:
`crates/pdf-model/src/page.rs`, `crates/pdf-syntax/examples/callgrind_open.rs`,
`crates/render-raster/tests/turn_path.rs` and the §7.7.3.2 ledger note. The hand-off patch is
`scratchpad/r1637/handoff/claude-md-citers.patch` (`-U0`, applies with `git apply --unidiff-zero`);
until it is applied `tools/governing-quotations.py` names the four as quoting `CLAUDE.md` text it
no longer has. Q25 and A25 are records and are not edited.
