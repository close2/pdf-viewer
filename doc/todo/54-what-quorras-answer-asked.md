# What quorra's answer asked of this tree

Status: **open** — two residues, both ours alone.
Priority: 54 — the same slot [`53`](53-what-hayros-tracker-asked.md) holds for the same reason: it
is a list extracted from *another project's* account of where it stands, so it arrives as a set of
questions rather than as one piece of work.

The project owner asked quorra where it stands and carried the answer back. This file is what that
answer owes *this* tree, with the items it already discharged struck out rather than dropped —
because "they said we still owe it" and "we did it two sessions ago" are two different states and
the difference is the point.

**Their account is evidence about their tree, not about ours.** `CLAUDE.md` principle 5 one
boundary over: the five-hundred-and-sixty-seventh session was *told* a name could come off
`REFUSED_AT_FOUR` and was right to leave it on until a run said so (ADR 0402 decision 3). Every
item below is written so that a round closes it by measuring rather than by believing.

## Already discharged, and by what

- **Take the bump.** They report this tree pinned 126 commits back and therefore blocked. It is not:
  the five-hundred-and-seventy-sixth session moved the pin `eada81ec` → `cad50156`, at zero source
  cost, with both coverage lanes at both scales measured. ADR 0411.
- **Drop `bug1703683_page2_reduced.pdf` from the scale-4 refusal list.** Done in the same session,
  and done *by a run*: their ADR 0057 sizes a clipped mark's coverage tile by its chain's own
  bounding box, the page now agrees with the CPU oracle at 4×, and `REFUSED_AT_FOUR` is three names.
- **Type 4's comparison operators on a boolean** — their ADR 0053 §3.2's question, and the item this
  file carried as number 1. Settled by the clause in the five-hundred-and-seventy-seventh session
  and settled the *third* way none of the three outcomes anticipated: §7.10.5.2 states no semantics
  itself and **defers** them to a document clause 2 makes normative, so the type restriction is
  ISO 32000-2's own requirement and the coercion was a silent departure. A program in which a
  boolean provably reaches `ge`, `gt`, `lt` or `le` is refused at parse time now and its caller
  reports it; where the compile-time walk cannot prove one, the conversion still answers. The
  population was measured first and is **zero** provable cases in 1 251 documents against two
  *containment* witnesses, both the owner's own files and neither actually doing it — trap 11 in one
  line. ADR 0412, and `doc/QUORRA_FEEDBACK.md` §26.3(b) carries the withdrawal for them to read.
- **~~`REFUSED_AT_FOUR` flattens two kinds of refusal.~~** Split in the five-hundred-and-seventy-eighth
  along the stage the refusal happens at: `REFUSED_BEFORE_THE_SCENE` is this tree's own two, raised
  while translating a display list and therefore scale-free, and `REFUSED_BY_THE_DEVICE` /
  `REFUSED_BY_THE_DEVICE_AT_FOUR` are the adapter's — empty at the page's own scale, which is a
  statement rather than an omission. Both scales are now held against **one** copy of this tree's
  names, which is what makes the five-hundred-and-twelfth session's stale second copy unrepeatable.
  ADR 0413.
- **~~The two lanes' differing sets.~~** Diagnosed in the same session, with an instrument rather
  than an argument: `crates/render-quorra/examples/lane_diff.rs` puts one display list through both
  lanes beside the oracle. All four pages are axis-aligned rules about a device pixel wide, the ink
  is right on all four and the placement is not — the default lane carries a per-command offset of
  up to ⅛ of a device pixel that the gpu lane does not have, and the gpu lane quantises some marks'
  y coverage where the default lane does not. **All four differences are quorra's**, because the
  display list is the same one; `doc/QUORRA_FEEDBACK.md` §31 is the write-up with the numbers.
  ADR 0413.
- **Re-baseline the scale-1 ratchet for `issue2177.pdf`.** Already done, and long before they wrote
  — the five-hundred-and-thirty-second session took it out of `DIFFERS_STRUCTURALLY` when their
  ADR 0049's `fill_mask` cut an edge piece at the tile border instead of clamping it. The comment on
  `DIFFERS_AT_THE_EDGES` in `crates/render-quorra/tests/corpus.rs` carries the geometry.

## 2. The two censuses they say are still ours

From `QUORRA_FEEDBACK.md` §27.4, restated in their answer and still not run:

- the **rectangular-fill census** — what share of this corpus's fills are axis-aligned rectangles;
- the **`(clip_residue_regions, clip_residue_tiles)` distribution** — which is the more interesting
  of the two on their own evidence, because their §5's `artwork` at 1.2× against the drawing's 6.6×
  makes it the number that says how much of the corpus is which.

**One walk over the corpus, not two**, and both are instrument work rather than a change to what
gets drawn. They have been open for several rounds and no round has spent the walk.

## 3. The fifth-frame tile-cache loss

Reported in our own §2 and declined as a defect at the time. `Counters::atlas_repacked` has been
wired here since the five-hundred-and-thirty-second session, which is where a round starts — the
question is whether the fifth frame's loss is a repack or something else, and the counter answers it
without a new probe.

## What is the owner's rather than a round's

Recorded here so that no round takes them for work, and so that they are not lost:

- **Carrying our answer back.** ADR 0411 §4 answers a question they left open and nobody asked us:
  `SceneError` and `RenderError` may be `#[non_exhaustive]`; `SurfaceProblem` may **not**, because
  that match covers all five variants with no wildcard on purpose, its completeness is `wgpu`'s, and
  a sixth arm should stop our build.
- **The presenting path.** Their ADR 0058's layer rectangle and `present.wgsl` need a real surface,
  and no gate in this tree has one. `doc/environment.md`'s measurement loop is the only route and it
  is for GPU measurements only.
- **Their three drafted answers** in `/home/cl/projects/render-lib/doc/feedback-answers-draft.md`
  (§15's clip bound, §19's rectangle-lane measurement, §22.5's rename-versus-document). **The draft
  is dated 2026-08-14 and §28 of `QUORRA_FEEDBACK.md` already answered §15 and §19 against it** —
  §15 closed as *already handled*. What is left of the draft is §22.5, and it is a naming decision
  rather than a piece of work.
- **Their ADR 0070's residual** — a 45° hairline given as a fill keeps the device lane and dots —
  is theirs, priced and declined upstream. This tree owes it a witness only if a corpus page shows
  it, which no round has looked for.
