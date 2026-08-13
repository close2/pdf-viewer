# Taking quorra's current revision — what this tree changes, and what it would gain

Written 2026-08-12 from the quorra side, against `2c9bdd0`. It is the counterpart to
`doc/QUORRA_FEEDBACK.md`: that document is what this tree measured and asked for; this one
is what came back, what it costs to take, and what is now available and unused. Every
number below names the command that produced it and the machine it ran on, because two of
the three sections that matter are measurements and one of those is not portable.

> **This file is about `2c9bdd0` and stays that way.** The tree pins `a7babab` since the
> four-hundred-and-seventy-eighth session — fourteen commits further on, six of them carrying
> ADRs, and the one with the number on it sizes every plan to what it marks. What that release
> required (nothing), what it did on this machine (four lanes, and no page refused for frame bytes
> at any scale), and what was declined from it (`Device::warm_for`) are `QUORRA_FEEDBACK.md`
> §22 and §9.2. **§6 below is the section to read against the current tree**: its three
> sheet-capacity refusals are the three that are still refused at 4×, and the packer that shipped
> in the same range (ADR 0034) did not move them, exactly as that ADR predicts about itself.

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
