# Taking quorra's current revision — what this tree changes, and what it would gain

Written 2026-08-12 from the quorra side, against `2c9bdd0`. It is the counterpart to
`doc/QUORRA_FEEDBACK.md`: that document is what this tree measured and asked for; this one
is what came back, what it costs to take, and what is now available and unused. Every
number below names the command that produced it and the machine it ran on, because two of
the three sections that matter are measurements and one of those is not portable.

> **This file's original body is about `2c9bdd0` and stays that way.** The tree pinned `a7babab`
> in the four-hundred-and-seventy-eighth session (`QUORRA_FEEDBACK.md` §22 and §9.2 record what
> that release required, did, and declined) and pins **`87898c69`** since the five-hundred-and-
> twelfth — twenty-five commits further on, whose own section is at the end of this file rather
> than woven into a record that was true when written. **§6 below decayed and its successor is in
> that section**: of its three sheet-capacity refusals, upstream measured the multi-sheet fix at
> `5483996` and declined it with the numbers written down, and two of the three pages now refuse
> for this tree's own §11 constructions before any sheet is packed (ADR 0327).

**Where the revisions sit.** `a35dc70` is pushed. The two commits after it — `6f777e8` and
`2c9bdd0` — are the answer to §14.2 and are the reason to take a second bump rather than
wait.

| | |
|---|---|
| `6ef954e` | panes; the blank-band defect; the coverage lane's criterion (ADR 0028) |
| `74c4994` | the placement census (ADR 0029) |
| `ab219d0` | the scratch sheet's stale tail — your §20.4.1 |
| `0ddaa40` | clip chains intersect — your §18 (ADR 0030) |
| `a35dc70` | the timestamp instrument is per device — part of your §9 (ADR 0031) |
| `6f777e8` | the staged pair inside a knockout group — your §14.2, first obstacle (ADR 0032) |
| `2c9bdd0` | a group as one stage — your §14.2, second obstacle (ADR 0033) |

## 1. What the bump requires: one line

`GroupSpec` gained a `compose` field, so every literal construction of it needs one more
entry. In this tree that is **one site**, `crates/render-quorra/src/scene.rs:155`:

```rust
    let spec = quorra_scene::GroupSpec {
        alpha: *alpha,
        blend: blend_mode(*blend),
        clip,
        knockout: *knockout,
        mask,
        // An ordinary group; §11.4.6's two stages are asked for by name (ADR 0033).
        compose: quorra_scene::Compose::SrcOver,
        isolated: *isolated,
    };
```

Verified rather than assumed: with that line added and nothing else changed, this tree's
corpus gate reads **915 agree, 37 differ, 5 refused** — the same verdicts as before the
bump. (`cargo test --release -p render-quorra --test corpus -- --ignored`, RADV.)

`StagedComposeReason::InsideKnockoutGroup` is **deleted**, so
`render-quorra/tests/headless_quorra.rs::quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over`
stops compiling. That is the test doing its job: §14.2 wrote it "so that it fails the day
you lift the restriction", and this is that day.

## 2. The four refusals are now writable

Both obstacles §14.2 named are gone. Nothing on our side draws those pages until this tree
emits them, because `render-quorra`'s `Command::Shaped` arm refuses before the scene
builder is reached.

**When both halves are fills** — `knockout_smask.pdf`'s shape:

```rust
// The erase: the object with every source of opacity removed, so its coverage is
// §11.6.4.2's shape. The paint is ignored by DestOut; the colour below is arbitrary.
builder.fill(outline, transform, rule, Paint::Solid(BLACK), clip,
             BlendMode::Normal, Compose::DestOut, /* mask */ None)?;
// The deposit: the object as it is, soft mask and constant alpha included.
builder.fill(outline, transform, rule, paint, clip,
             BlendMode::Normal, Compose::Plus, mask)?;
```

**When either half is a group** — the other three pages, because §11.3.7.2 makes a group's
shape the union of its elements' and no fill can state that:

```rust
let stage = |compose| GroupSpec {
    alpha: 1.0, blend: BlendMode::Normal, clip, knockout: false,
    mask: None, isolated: true, compose,
};
// The erase half: the same content drawn opaque, so the group's alpha *is* its shape.
builder.group(stage(Compose::DestOut), |body| shape_content(body))?;
// The deposit half: the content itself, at its own alpha and under its own mask.
builder.group(stage(Compose::Plus), |body| object_content(body))?;
```

Three constraints, each refused at the builder with a named reason rather than drawn
wrongly:

- **both halves, or neither.** `Plus` alone drives a premultiplied channel past its alpha;
  one mark cannot tell the library the other is coming, so this is the caller's obligation
  and always has been (`Compose::Plus`'s own documentation).
- **a staged group must be isolated.** §11.4.4 seeds a non-isolated group's buffer with its
  own backdrop, so the alpha the erase half reads as a shape would carry the backdrop's
  too. `GroupComposeReason::NonIsolated`.
- **a staged mark or group may not also carry a blend mode.** §11.3.5 composites it through
  an implicit group, which is the step the pair replaces. `BlendNotNormal`.

What it is worth, measured on quorra's side against the clause's own line
`P' = (1 − f) × P + S`, worst premultiplied deviation over every pixel of a fixture with a
diagonal edge:

| | deviation |
|---|---|
| the pair, element inside a knockout group | **0.77 of 255** (unorm rounding) |
| the same element as one soft-masked mark | 108.29 |
| the pair, a group as each half | **0.77** |
| the same group composited ordinarily | 114.95 |

## 3. The coverage lane is a per-frame choice, and its crossover is *your machine's*

`Device::set_coverage` may be called between frames, and ADR 0016's argument for the GPU
lane is that its cost does not grow with magnification — which makes a zoom gesture the
case for it and a page of body text the case against.

**Do not take a scale threshold from us.** The comparison is host rasterisation against
this adapter: our CPU lane is single-threaded per frame on your processor, the GPU lane is
whatever card the host has. Both sides move with the machine, so the crossover is a
property of the *pair* and any constant we published would be this laptop's. What we can
give you is the instrument and the invariants.

**The instrument.** Your gate already takes `PDFVIEWER_QUORRA_COVERAGE=cpu|gpu` and
`PDFVIEWER_QUORRA_SCALE`, which is the right shape — run it on the machine you care about.
On quorra's side `tests/lane_crossover.rs` is the same question at tile granularity
(`cargo test --release -p quorra-gpu --test lane_crossover -- --ignored --nocapture`), and
it reports first-frame and warm separately because the atlas moves them apart by an order
of magnitude.

**What does not move with the machine**, and can therefore be reasoned about once:

- **Which pages refuse.** A refusal is arithmetic against a stated budget, not a clock.
- **The fidelity envelope.** The GPU lane samples on an ordered grid that is ours rather
  than the driver's (ADR 0006), so its disagreement with the analytic lane is the same on
  any adapter: bounded at 32 of 255 on a straight edge and 96 on a curve, and *measured* at
  1.5 to 12.2 in the pages of your corpus that moved.
- **Which lane a command takes.** As of ADR 0029 the encoder decides per command by what
  the cache will do with the tile — refused by size, already resident, placed once, placed
  again — with ADR 0026's triangle floor beneath it. You do not need a per-page heuristic;
  asking for `Coverage::Gpu` means "where it pays", and on a page of cached glyphs it pays
  nowhere and the lane declines.

**The fidelity cost is a choice, not a defect.** On this machine's corpus at scale 1, the
GPU lane moves nine pages from *agree* to *differ* (all within the envelope above) and
takes one refusal away.

**And here is the pair, on this machine, so that the shape of the dependency is visible
rather than asserted** — AMD Ryzen AI 9 HX 370 (24 threads) with a Radeon 890M under RADV,
957 pages, one working copy, `2c9bdd0`:

| | agree | differ | refused | rasterisation |
|---|---:|---:|---:|---|
| scale 1, `cpu` | 915 | 37 | 5 | 7.24 s |
| scale 1, `gpu` | 914 | 38 | 5 | 7.22 s |
| scale 4, `cpu` | 925 | 16 | 11 | 34.28 s |
| scale 4, `gpu` | 926 | 14 | 12 | 23.40 s |

At page scale the two are indistinguishable in time, which is the atlas doing its job and
the lane declining to take work it would lose on. At 4× the device is **a third faster
over the whole corpus** and disagrees with the oracle on two fewer pages, at the cost of
one refusal the CPU lane does not make. Those four rows are one processor against one
integrated GPU: a discrete card moves the second column down, more cores move the first,
and neither moves the *refused* column at all. Run them where it matters before deciding —
and note that `Device::set_coverage` makes this a per-frame decision, so "the CPU lane at
page scale, the device while zoomed" is expressible without rebuilding anything.

## 4. Two ratchets to re-baseline

- **The GPU-lane lists moved in your favour.** The sheet fix (§20.4.1) takes five pages at
  scale 1 and six at scale 4 from *differs* to *agrees*: 909 → 914 agreeing at 1×, and
  920 → 926 at 4×, measured in one working copy across the two revisions.
- **Clip chains now intersect.** Your §18 asked what rule composes a chain here; it was a
  product and is now `min`, from §8.5.4's own sentence — the graphics state holds *one*
  clipping path, set to the intersection of the current path and the new one. Your ADR 0280
  made the same change on your side, so the cross-backend gate should stop carrying that
  difference. Note that where a clip meets the *mark*, quorra still multiplies, and ADR 0030
  records that as a choice with its reasoning.

## 5. §13's instrument already exists

`doc/QUORRA_FEEDBACK.md` §13 is still marked **open**; it was answered on 2026-08-11 by
quorra's ADR 0023, three days after §13 was written. Both halves of the ask:

- `Options::instrument_encode` turns on a subdivision of encode, reported through
  `Timings::phases` — geometry, staging and the rest, so 3.86 µs a command can be
  attributed rather than guessed at.
- `Timings::execute_provenance` says which clock `execute` came from, and `phases` now
  names `"target acquire"` and `"present"` — the two things your `elsewhere` row was
  standing in for.

## 6. What still refuses, and what the fallback is still for

**Of the twelve refusals your §20.4.2 counts at scale 4, three are the coverage sheet
against the adapter's 16 384 limit** — `bug1703683_page2_reduced`, `bug1721218_reduced` and
`issue1905` — a different ceiling from the frame's byte budget, and one the pane work
cannot reach. (Four more are §14.2's knockout pages, which §2 above makes writable; five
are budget refusals, four of them over the 256 MiB frame budget by 4 % to 20 %.) `transparency_group.pdf` at 4×
packs a 10 853 × 3 070 sheet holding six full-page 3 200 × 2 400 tiles, which is the shape
of the problem: a page needing several page-sized coverage tiles exhausts the sheet by
shelf height. The fix is a frame using more than one sheet pass, and it is the next design
question on our side rather than something you can work around — except as your §15 already
does, by clipping a shading to the region that survives the clip before handing it over.

## 7. What we would still like from you

- **A decision on a size hint.** ADR 0031 took 2.4 ms off the first frame by making the
  timestamp instrument once per device instead of once per frame. About 6 ms remain, and
  they scale with the target — page-sized textures and the driver's first touch of a heap
  that size. A warm-up thread cannot allocate those before the viewport exists, so the only
  way to move them off the critical path is for the device to be told the size it is about
  to be asked for: `Device::warm_for(width, height)`, or a hint on `Options`. That is a
  change to the contract between us, so it is yours to want before it is ours to build.
- **§19 is still yours**, and unchanged: no page of the corpus emits a `Command::Rect`.

---

## `87898c69`, taken in the five-hundred-and-twelfth session (2026-08-14)

Twenty-five commits past `a7babab`, and the second bump in a row that cost this tree **not one
line of source**: `Cargo.lock`'s two hashes, `clippy --workspace --all-targets` silent, every
test compiling. (`SceneError` and its three reason enums moved to `quorra-scene/src/error.rs`
upstream, with their crate-root paths kept — which is why the move is invisible here.)

The range, oldest first, merges elided:

| | |
|---|---|
| `1fdbceb` | a child the clip leaves nothing to contribute is not emitted (quorra ADR 0041) |
| `2c78b5f` | `warm_for`'s 24.7 → 10.3 ms **retracted** — the first frame was paying two in-frame pipeline compiles, now warmed (ADR 0040) |
| `d594566` | **the §21.1 round cap**: the near cap was the inward half-disc wound against the body — a hole, not absent ink |
| `4e6246d` | a WGSL compile failure is a typed refusal naming its span, not a silent test hang; `Device::warm_up() -> WarmUp` (ADR 0042) |
| `5483996` | multi-sheet passes **measured and declined**: a second sheet re-refuses one page on bytes and prices the others at ¼ GB of per-frame upload |
| `1af2722` | the warm set compiles the presenting lanes in the surface's negotiated format (ADR 0043) |
| `f1873da` | `surface_measure.rs`: the brief's §6.2 measured presenting for the first time — execute is ~4% of a dense-text frame |
| `0b60cb9` | §19 priced: a rectangle costs 0.13–0.49 µs more as a `Fill` than as a `Rect`; recogniser found wired to shaded fills only, upstream |
| `7cf99f0` | the scene fuzzer learns the post-M5 vocabulary; found nothing over 4 000 seeds, with invariants beyond "did not panic" |
| `89f034e`–`2f99117` | shader-copy enforcement, `encode.rs` split into `scratch`/`clips`/`rare`, two `bool`s become named enums — internal |
| `4839b52` | §11.3.5's implicit blend group written once (`ChildOp::implicit_blend_group`); found a blended-stroke-in-knockout asymmetry, left open and named |
| `33ce2b8` | **the §21.2 chord floor** (ADR 0044): a cubic's flattening bound is the tighter of ¼ px and 1/32 of its own extent — 16 chords a turn, on §10.7.2 NOTE 2 |
| `195a89d` | the glyph key probed once instead of three times; −33% on the zoom example's encode (correction to ADR 0024) |
| `7896874` | **encode reuse priced** (ADR 0045): replaying a retained `Encoded` is 0.154 ms against 1.538 re-encoded — not built, one question asked back |
| `87898c6` | closing round: §6.2's presenting bar met at 1.816 ms minimum on their machine; documents reconciled |

### What it did on this machine, all four lanes

The four-lane debt of `doc/todo/02-every-round.md` §2, run whole; verdicts only, no clocks — the
machine carried parallel rounds throughout.

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| scale 1, `cpu` | 934 | 20 | 2 | 18 |
| scale 1, `gpu` | 933 | 21 | 2 | 18 |
| scale 4, `cpu` | 936 | 10 | 5 | 23 |
| scale 4, `gpu` | 937 | 9 | 5 | 23 |

Against §22.2's table at `a7babab` (918/917/931/932 agreeing): **seventeen pages joined the CPU
oracle at scale 1 and none left, at any scale, on either lane** — fifteen prose pages from the
chord floor (the whole `tracemonkey` family among them: the population a flattening floor reaches
is glyph bowls, cubics two to five device pixels across), plus `extgstate.pdf` and
`inks_basic.pdf` from the round cap. `DIFFERS_AT_THE_EDGES` is 7 names from 24. The refusal
deltas are **not** this release's: `issue18032.pdf` (scale 1: 1 → 2; scale 4: 4 → 5) is session
492's own §11.4.6 refusal, stated before any scene is built and therefore at every scale — the
4× ratchet simply had not been run since 492 wrote it. Both lanes refuse the same five pages at
4×, which continues §22.2's finding.

### The two §21 gates are written, which is what §22.7 said to do first

`sub_pixel_marks` re-run at this pin: the 40 × 5 round-capped rule reads −0.1% from Table 53's
own area (was −8.9%), the one-pixel dot −2.1% (was the inscribed square at −36.1%).
`render-quorra/tests/sub_pixel_coverage.rs`'s round-cap and dot rows now gate **both** backends —
the rows held against the processor only since the four-hundred-and-fifty-fifth session.
`QUORRA_FEEDBACK.md` §21.4 has the numbers and the two corrections that came back with them.

### What it does *not* carry: `doc/todo/44` §3's two asks, answered rather than shipped

Quorra's ADR 0045 priced the retained encode (see the table row above) and **built neither ask**:

- **Scene-fragment composition is deliberately unbuilt**, pending one question this tree now owes
  an answer to: *can the host draw the page and the overlays as two `render` calls into the same
  target?* If yes, a device-side replay keyed on scene identity needs no new vocabulary; if no,
  the reason why is the specification for fragment composition. `doc/todo/44` §3 carries the
  question and what an answer must check.
- **The root affine does not buy zoom reuse, at any price** — their correction of our §3, not a
  refusal of it: the linear part of the device transform is inside every atlas key, the
  flattening and the lane choice, and the sub-pixel phase is the fractional translation. Building
  the page scene in page space under `Viewport`'s existing affine buys the *scene* phase only
  (median 50.2 ms on the owner's document), which needs nothing from upstream.

`Options::instrument_encode` stays unused here, rightly: upstream attributed `recording` at 78.3%
of a steady encode with callgrind on their side, which is the finer number the instrument existed
to reach.

### The frame path on the owner's document, before and after

`tmp/Entwurf.pdf` (58 009 commands) under `Xvfb`/llvmpipe, both pins alternated A/B/A/B, structure
only: the shares are unchanged — `encode` is ~90% of `device` at the median on both pins,
`transfer` ~0.1 ms, `execute` single-digit, `elsewhere` small; steady frames upload 40 resources
and cull identically. The quiet runs' medians agree between pins to within run-to-run spread, and
the one discordant run was a load spike (its own arm's other run disagrees with it by 3×). No
wall-clock claim; the machine was shared.

## 8. Added 2026-08-14, from the quorra side: an API addition, in its own document

**This section is a pointer, not a revision of anything above it.** Sections 1 to 7 are
about `2c9bdd0` and stay that way; this one names a change that landed later and has a
document of its own, because it is the first thing quorra has asked this tree to *adopt*
rather than merely to take.

**`Device::render_retained` and `RetainedScene`** — a frame whose scene, viewport and
device state are unchanged replays the encode of the previous frame instead of walking
your commands again. It is the upstream half of `doc/todo/44` §3's ask, it is quorra's
ADR 0048 (built on ADR 0045's pricing), and **nothing changes for a caller who ignores
it**: `Device::render` is untouched.

What it is worth, what it cannot do (a zoom step reuses nothing per command, and the
reason is not fixable by any design), and what this tree has to change in
`QuorraPresenter::present` to get it — including three things `present` does on every
frame today that would defeat it — are in **`doc/QUORRA_RETAINED_FRAME.md`**, written the
same day and against the same revision.

One line from it worth having here, because it is the part that costs work: the frame's
scene must stop being rebuilt when nothing changed. Today `present` builds a fresh
`SceneBuilder`, re-runs `Encoder` over the page's display list and rebuilds every overlay
on every frame; the retained handle can only replay an encode of a scene that is still
the same scene.

---

## `580fa4ac`, taken in the five-hundred-and-sixteenth session (2026-08-14)

Eight commits past `87898c69`, and **the first bump this tree has had to reach for rather than
merely take** — nothing broke, and nothing in the release is compulsory; what it carries is an
addition that does nothing at all for a caller who ignores it.
The bump itself is still two hashes: `cargo update -p quorra-gpu -p quorra-scene`, then
`cargo build --workspace --all-targets` clean and `clippy --workspace --all-targets` silent with
not one line of this tree touched. Every line that moved, moved to **take** `render_retained`,
and ADR 0351 is that adoption.

The range, oldest first, merges elided:

| | |
|---|---|
| `a906359` | `--remap-path-prefix` measured against sccache and **declined**: the cache key already carries the paths cargo derives from the target directory, and a per-checkout `RUSTFLAGS` un-shares every registry dependency (their traps section; this tree's ADR 0344 found the same thing from the other end) |
| `a22442e` | **`RetainedScene` and `Device::render_retained`** (quorra ADR 0048): the handle owns the `Scene` and the encode of its last frame; `EncodeKey` enumerates every other input an encode reads and compares it by bits; `Frame::encode_source` is the observable; nineteen tests, one per entry of the invalidation list |
| `8d74c41` | that release's numbers and survival table into their `PLAN.md` and `HANDOVER.md` |
| `6b75e00` | **a blended stroke inside a knockout group is replaced, not blended**: `encode_stroke` wrapped a non-Normal blend in §11.3.5's implicit group on the blend mode alone, where the fill and image arms also require the enclosing style to be `Over`. Worst deviation from §11.4.6's own line, on their fixture: 112.95 of 255 → 0.87 |
| `cff170e` | that fix's documents |
| `a85cc47` | **the rectangle lane stops depending on which command drew it** (their ADR 0047): `rect_hint` was computed for every outline and read on the shaded-fill arm only, so a *solid* fill of a rectangle — the only form a document rectangle arrives in, which is this tree's §19 — took the atlas or the coverage sheet. 0.466 → 0.210 ms of encode on their p99 rectangle page, 0 of 8 022 576 bytes differing |
| `5f7c8c8` | that fix's documents, and a shared-target-directory trap |
| `580fa4a` | closing round: `surface_measure` re-run post-merge on RADV |

**Two of these move pixels and neither moved this tree's**, which upstream measured on this
tree's own corpus before publishing (one copy, flipping only the `[patch]`): identical verdicts at
both scales for `6b75e00` — the corpus does not reach a blended stroke overlapping what a knockout
group already holds, so their fixture had to be built rather than found — and five pages in 951
moving a mean by 0.0001–0.0021 with every worst tile unchanged for `a85cc47`. This tree's own four
lanes at this pin are in ADR 0351.

### What it required of this tree

Nothing, to compile. Everything in ADR 0351, to be worth taking. The three obstacles
`QUORRA_RETAINED_FRAME.md` §3 names were all real; **one of them is stronger than that document
states**, and the correction has gone back in `QUORRA_FEEDBACK.md` §23: it is not only that a
reused frame has nothing to release, it is that the *rebuild* frame must not release its own
transients either, because the retained handle names them until something replaces the scene.

`Options::instrument_encode` stays unused, and now for a second reason: a replayed frame's encode
subdivision is zero by construction.
