# What quorra's answer asked of this tree

Status: **open** — five residues, three of them ours alone, one with a code witness.
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
- **Re-baseline the scale-1 ratchet for `issue2177.pdf`.** Already done, and long before they wrote
  — the five-hundred-and-thirty-second session took it out of `DIFFERS_STRUCTURALLY` when their
  ADR 0049's `fill_mask` cut an edge piece at the tile border instead of clamping it. The comment on
  `DIFFERS_AT_THE_EDGES` in `crates/render-quorra/tests/corpus.rs` carries the geometry.

## 1. Type 4's comparison operators on a boolean — ours, with a witness in the code

**This is the one with a defect behind it, and it is trap 5's shape exactly.**

Their ADR 0053 §3.2 asks what the contract is when `gt`, `ge`, `lt` or `le` compare booleans, where
PostScript raises `typecheck`. In this tree they do not compare booleans — they *coerce* them:

- `crates/pdf-model/src/function.rs:2158` — `Operator::Ge => binary(stack, |a, b| Value::Boolean(a.as_f64() >= b.as_f64()))`, and `Gt`, `Lt`, `Le` beside it.
- `Value::as_f64` at `function.rs:1105` maps `Boolean(value)` to `f64::from(u8::from(value))`.

So `true 0 gt` yields `true` where PostScript raises an error, **and nothing is reported**. A
type 4 function is §7.10.5, and what it says about the operand types of the four comparison
operators is the question — not what PLRM3 says, which is evidence about the language the clause
borrows from. §7.10.5.2's own list of operators and whatever it states about types decides it, and
`doc/md/ISO_32000-2_sponsored_EC3.md` is where to read it rather than from memory.

Three outcomes and they are different work: the clause states the type restriction, in which case
this is a silent departure to make loud (principle 3 and trap 5); the clause states nothing, in
which case it is a documented choice under `CLAUDE.md`'s "where the standard genuinely defines
nothing" rule — **and read the titles around it first**, because that claim decays; or the clause
states the coercion, in which case the code is right and the comment above it should say so with
the clause number.

**No corpus witness is known.** What would produce one is a grep for a `/FunctionType 4` stream
whose program pushes a comparison operator onto a value the program itself produced with `eq`,
`and`, `or` or `not` — the only way a boolean reaches the operand stack. Measure the population
before pricing the fix; a departure no document can reach is still a departure, but it ranks
differently.

## 2. `REFUSED_AT_FOUR` flattens two kinds of refusal — ours

Their note, and it is right on inspection. `crates/render-quorra/tests/corpus.rs:332` holds three
names and their reasons are not the same *kind*:

- `issue1905.pdf` is refused **at render time** by the device — the rasterised-coverage sheet
  exceeds this adapter's 16 384 × 16 384 texture. That is a capability, it is quorra's to move, and
  its message changed with `cad50156` (it names the sheet now, their ADR 0057 decision 2).
- `bug1721218_reduced.pdf` and `issue18032.pdf` are refused **before the scene is built**, by this
  tree, for §11.6.6/§11.7.2's four-component blending space and §11.4.6's non-isolated knockout
  group (ADR 0327). They refuse at every scale and no quorra release can move them.

One array holds both, so a name leaving it means two unrelated things and the ratchet cannot say
which. The fix is to split the constant along the stage the refusal happens at, with each half's
doc comment naming what a departure from it would mean. Small, and it makes the *next* release
round's report legible rather than needing prose to disambiguate it.

## 3. The two censuses they say are still ours

From `QUORRA_FEEDBACK.md` §27.4, restated in their answer and still not run:

- the **rectangular-fill census** — what share of this corpus's fills are axis-aligned rectangles;
- the **`(clip_residue_regions, clip_residue_tiles)` distribution** — which is the more interesting
  of the two on their own evidence, because their §5's `artwork` at 1.2× against the drawing's 6.6×
  makes it the number that says how much of the corpus is which.

**One walk over the corpus, not two**, and both are instrument work rather than a change to what
gets drawn. They have been open for several rounds and no round has spent the walk.

## 4. The fifth-frame tile-cache loss

Reported in our own §2 and declined as a defect at the time. `Counters::atlas_repacked` has been
wired here since the five-hundred-and-thirty-second session, which is where a round starts — the
question is whether the fifth frame's loss is a repack or something else, and the counter answers it
without a new probe.

## 5. The two lanes' differing sets — the five-hundred-and-seventy-sixth's own residue

Not from their answer but from the round that took their release, and it belongs with these because
it is the same instrument. The two coverage lanes' page counts converge and their *sets* do not:
each differs from the CPU oracle on 23 pages at page scale and two of each 23 are its own —
`bug1863910.pdf` and `issue16500.pdf` device-only, `bug1743245.pdf` and `issue21068.pdf`
processor-only. A count that agrees while its membership does not is the shape
`doc/todo/02` §7's second habit warns about. Four pages, four side-by-sides.

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
