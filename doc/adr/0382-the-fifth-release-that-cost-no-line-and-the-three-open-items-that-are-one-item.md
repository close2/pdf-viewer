# 0382 — The fifth release that cost no line, and the three open items that are one item

Date: 2026-08-16. Status: accepted.
Supersedes nothing. Amends [0377](0377-the-geometry-phase-divides-and-the-number-is-the-machines.md)'s
two magnified-lane rows, for the reason §4 gives.

## The situation

`Cargo.lock` pinned quorra at `619ef3b4`; `origin/main` in the sibling checkout carried
thirty-three commits past it, all of them from one day. This round takes them, and answers
`/home/cl/projects/render-lib/doc/feedback-answers-draft.md` — upstream answering this tree's
`QUORRA_FEEDBACK.md` §15, §19 and §22.5 and raising the encode cache from their side.

Two questions came with the round rather than with the release: what a *retained atlas* would owe a
presenter whose frame cost has to be predictable, and whether an already-rendered frame can be put
on the screen under a transform without a readback. The second is the five-hundred-and-forty-eighth
session's whole subject and is deliberately not built here.

## 1. The release: no public API change at all, and one pixel-moving change

`cargo update -p quorra-gpu -p quorra-scene` → `a4380e2c`. `build --workspace --all-targets` clean,
`clippy --workspace --all-targets` silent, `fmt --check` clean: **the fifth bump in a row that cost
this tree not one line of source.**

This one is different in kind from the previous four, and the difference is checkable rather than
felt. The four before it survived *breaking changes in general* — a `Paint` variant, a `ResourceId`
variant, an `Options` field — because nothing here scrutinised those types. This one had nothing to
survive: every `pub` line of both crates was diffed across the range and **not one public item was
added, removed, renamed or resignatured**, with `quorra-scene` untouched entirely. `Counters`,
`Frame`, `Timings`, `Options`, `DeviceError`, `RenderError`, `RetainedScene` and `Device` are the
same types; `frame.rs`, `error.rs` and `startup.rs` are untouched in the range.

The range is four groups: their CI documentation gate and a `#[cfg(test)]` uniform-layout deriver
that checks host-side field offsets against the WGSL structs they mirror; the encode walk split
into eleven private submodules and the retained-frame tests into five files; **their ADR 0055**, the
colour ramp's coincident stop offsets; and the tiling ceiling *measured and not fixed* — their notes
say in as many words that no `src/` file was changed by that round.

`QUORRA_UPGRADE.md`'s new section has the table and the group-by-group reading.

## 2. Why a pixel-moving change was accepted: the clause, not the corpus

Their `ramp_color_at` loop comparison goes from `<=` to `<`, so a `t` landing exactly on a stop's
offset belongs to the interval that **starts** there. This tree cannot take a change that moves a
mark because a corpus improved — principle 5 forbids exactly that — so the clause was read first.

A colour ramp is a shading's colour function already sampled onto stops, and a producer puts two
stops at one offset wherever that function jumps: a §7.10.4 stitching bound. The clause makes the
subdomains

> half-open intervals, closed on the left and open on the right

with two exceptions that point in **opposite** directions — the last interval closed on the right,
and the first closed on both sides where `Domain0 = Bounds0`. Three cases, three answers: the
interior bound to the later stop, the ramp's first offset to the earlier one, its last offset to the
later one. Their code now does all three and their ADR derives it from the clause rather than from
another renderer, which is the only ground on which this tree could take it.

## 3. The four lanes, and the two magnified ones A/B'd in one working copy

Because the release moves a mark, the two scale-4 lanes were run twice in one working copy flipping
only `Cargo.lock`, so that what moved is attributed rather than inferred.

| | `619ef3b4` | `a4380e2c` |
|---|---|---|
| scale 1, `cpu` | — | 931 / 23 / 2 / 18 |
| scale 1, `gpu` | — | 929 / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 11 / 4 / 23 | 936 / 11 / 4 / 23 |
| scale 4, `gpu` | 937 / 10 / 4 / 23 | 937 / 10 / 4 / 23 |

**One judged line moves, on both magnified lanes, and it is the same line to the digit:**

```text
- differs: issue10572.pdf: mean 0.1332 worst tile 7.97 at (256, 1792) differing 0.0005 ssim 0.99497
+ differs: issue10572.pdf: mean 0.1036 worst tile 7.97 at (256, 1792) differing 0.0004 ssim 0.99602
```

Every other judged line of both lanes is character-identical between the pins, and it moves toward
the oracle on all four numbers the comparison carries. Upstream predicted this page, this direction
and these figures from their own copy of this corpus; reproducing it here is the check that their
prediction deserved rather than the repetition of it.

**Nothing moves at scale 1, and that is proved rather than counted.** The scale-1 default lane holds
*both* its refusal list and its differing list to equality and it passed, so no page changed
category there. `issue10572.pdf` appears in neither scale-1 lane's judged output at either pin: at
one device pixel per point the ramp bound falls inside a pixel that was already right.

## 4. ADR 0377's two magnified rows are one page stale, and the page is ours

ADR 0377 and `QUORRA_FEEDBACK.md` §27.2 record `936 / 10 / 5 / 23` and `937 / 9 / 5 / 23`. Both were
correct for the session that wrote them and both are stale now, by one page in one direction:
`22060_A1_01_Plans.pdf` left `REFUSED_AT_FOUR` and joined the differing list in the
**five-hundred-and-forty-third** session, when `image::RasterCache` stopped a page decoding one
image `XObject` thirty-six times and the page fell under `max_resource_bytes` without it being
raised (ADR 0374). The ratchet moved in that same commit, which is why every run since has passed.

Upstream found this before we did — they re-ran our lane against an unchanged quorra and read
936 / 11 / 4 / 23 — and their reading of it was right. This round's contribution is the attribution:
the run at the old pin reads 936 / 11 / 4 / 23 too, so neither the release nor anything upstream did
is responsible.

**The shape is worth more than the correction.** A ratchet held to equality and a count written in
prose decay at different rates, and only one of them can go stale silently. `REFUSED_AT_FOUR` could
not have carried this error for a session; the summary line beside it did, through two documents and
into upstream's `doc/PLAN.md`. Where a round has both available, the assertion is the record and the
prose is a copy of it.

## 5. The draft answered: two sections close, one keeps its name

`QUORRA_FEEDBACK.md` §28 is the reply and has the argument. In brief:

- **§15 closes as *already handled*.** Their coverage lane intersects shape ∩ clip ∩ target before
  rasterising, and has since it was written. The correction we owe with the closure is that §15
  said we could not see this from here, and the doc comment on `visible_tile` states it verbatim.
- **§19 closes too, and their four lines closed it before we read the question.** Their ADR 0047
  wired the analytic rectangle lane to the *solid* fill arm at `a85cc47`, inside a release this tree
  took two bumps ago — checked in `encode/fill.rs` at the revision now pinned. So the saving §19
  asked us to buy with a translation-side recogniser is already delivered, and recognising the shape
  at upload once per outline is strictly better than recognising it per command in our translation.
- **§22.5: keep the name.** `layer_textures` never stopped meaning what its name says; what changed
  was a derived claim. Their three rules are accepted, and the third — an ADR that moves a counter's
  value names the counter — is the one that would have reached this tree, because the decisions that
  moved it (their ADRs 0036 and 0038) are documents we read and neither said the word.
- **The encode cache**: their pricing is accepted and their correction of `doc/todo/44` §3 lands. A
  zoom step is a different rasterisation of every glyph and no cache survives it; we had their
  ADR 0045 and asked anyway.

## 6. The decision this round actually took: three open items are one item

Nothing was built. What was found is that three things this tree has been carrying separately have a
single mechanism behind them, and the mechanism is a **target** rather than a scene.

Upstream's encode cache needs the page's `Scene` to be stable across frames, which needs the page
and the chrome to be two `render` calls. Session 548 needs an already-rendered frame re-presented
under a transform without a readback. Session 543's `capture_presented` costs 19–36 ms because this
crate never holds the swapchain texture. All three are answered by the presenter rendering the page
into a **texture it owns** instead of straight to the surface:

- two `render` calls into one target work today against `Target::Texture` and only against it — a
  non-empty `Viewport::damage` is honoured exactly there (`LoadOp::Load`), while a `Surface` has no
  retained contents to patch and is redrawn whole, which their device reports rather than hides;
- a texture created `RENDER_ATTACHMENT | TEXTURE_BINDING` is accepted by their `validate_texture`,
  which asks `contains` rather than equality, so the host may sample it afterwards;
- `Device::wgpu()` hands over the same device and queue, and `quorra-gpu` re-exports `wgpu`, so one
  `wgpu` is linked and there is no seam to mismatch on.

**The escape hatch is complete and needs nothing from upstream.** What it costs is a blit this tree
does not own — and that cost is a *startup* cost, not a rendering one: `CLAUDE.md` puts pipeline
compilation on the critical path by choice, so a blit pipeline built here is one more shader
compiled on the launch path, outside quorra's warm set and outside their ADR 0043's
negotiated-format keying. That is the argument for asking them for
`present_texture(&wgpu::Texture, Affine, into: Target)` rather than building it, and §28.6 asks it
with the reasoning and with an explicit "if the answer is no, we will build it here".

**Not built this round, deliberately.** 548 owns it, and a round that took a release should not also
rewrite the present path.

## 7. What a retained atlas would owe, recorded because the release said nothing about it

Nothing in the range touches atlas behaviour — the one commit that reaches `atlas.rs`'s
neighbourhood is the test-file split. So this is what this tree wants, written down while the
question is open rather than after 548 has hit it.

Their ADR 0050 bounds a *single page* at one repack, two encodes, then replays for ever. It also
states that **two pages alternating give one repack per frame for ever**, and a viewer is a machine
for alternating between two views. A repack invalidates the retained encode, and a re-encode is
150–460 ms on the document this tree measures against a 8–16 ms budget.

Wanted, in order, and §28.5 has the reasoning:

1. **A way to decline the repack** — an `Options` switch. The scratch path is a bounded extra cost
   per frame; a re-encode is an unbounded one. A host with a frame budget wants the first trade and
   today cannot ask for it.
2. **The working set of a `Scene` before it is rendered**, the way `function::admit` answers about a
   program without an adapter. `atlas_working_set_bytes` arrives in a `Frame` that has already been
   paid for, and a refused frame carries no `Frame` at all.
3. **Recency in the packer** — their own named answer, explicitly *not* asked for, because (1) would
   remove the need.

And one thing explicitly not to change: `atlas_repacked` reports on the frame that *caused* the
repack rather than the one that pays for it. That reads like an off-by-one and is the useful
direction — it is what makes `capture_presented`'s refusal correct rather than late.

## 8. Upstream's question about the two refusing pages, answered with the instrument

They asked, before spending a round on the tiling ceiling, whether `issue1905.pdf` refuses in the
product or only in the gate. **Only in the gate**, and the same for `bug1703683_page2_reduced.pdf`.

`crates/render-quorra/examples/zoom_ladder.rs` draws a 900 × 1100 window at a ladder of
magnifications with the page's placement in the transform — which is what this viewer does, since
`TargetSpec`'s extent is the window's — and both pages draw on every rung from 100 % to 6400 %, up
and back down, on both backends, with no refusal. At 6400 % `issue1905.pdf`'s page would be
79 808 × 126 976 if anyone rendered the whole of it.

The honest half: above 800 % the window holds a blank part of `bug1703683_page2_reduced.pdf`, so
both backends draw nothing and agree trivially. That is evidence of no refusal, not evidence of
agreement. And the gate is not wrong to refuse — a whole page at 4× is deliberately harsher than a
window, which is why it is the population the gate measures.

## Consequences

- The pin is `a4380e2c`. One corpus page line moved, toward the oracle, at scale 4 on both lanes.
- Two sections of `QUORRA_FEEDBACK.md` close; one counter keeps its name; one question goes back.
- ADR 0377's two magnified rows are corrected here rather than edited there, and the mechanism that
  let them go stale is written down.
- The present path is unchanged, and the argument for changing it is now written rather than
  discovered by the round that has to.
