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

---

## `a64a9084`, taken in the five-hundred-and-thirty-second session (2026-08-15)

Twenty-eight commits past `580fa4ac`, and **the first bump that arrived with a migration document
written from the other side**: `doc/QUORRA_API_2026_08_15.md` is upstream's own account of what a
`render-quorra` author has to do, and it is now in this tree beside the ask it answers. It covers
the first twenty of the twenty-eight; the last eight are the answer to
`doc/QUORRA_FUNCTION_PAINT.md`, which has a document of its own —
`doc/QUORRA_FUNCTION_PAINT_ANSWER.md` — and changes not one line of the library.

**It still cost nothing to compile**, which is three in a row: two hashes in `Cargo.lock`,
`build --workspace --all-targets` clean, `clippy --workspace --all-targets` silent. What it cost
was **one test line** (a name off a ratchet, because a page joined the oracle) and the adoption
in ADR 0367.

The range, oldest first, merges elided:

| | |
|---|---|
| `6ed67f0` | their `PLAN.md`'s history moves to a `doc/history/` a round adds a file to — the convention this tree adopted in ADR 0281, arriving there |
| `e3fb452` | `scene.rs` 1 216 lines → seven files: the vocabulary, the builder, the frame stack, §4.7's refusals, the cost walk |
| `d1a60dc` | `compose.rs` 1 014 → six: the region arithmetic, the content pass, §11.3.6's child, the three blits, §11.5's masks |
| `49cbebd` | `winding.rs` 844 → four: the sheet and its price, the per-frame buffers, the two passes |
| `4f6db8f` | a module comment that said four over a list of five |
| `141cfee` | **a repack that would rebuild the layout it replaces is not taken** (their ADR 0050) |
| `8e394e2` | a resource id this device has issued is never issued again — `DeviceError::ResourceIdsExhausted` |
| `d445f32`, `a788251`, `8aed8bf`, `dd6a71b` | those two merged, with their ADR 0051 on what a split *public* module costs |
| `a6bebbf` | their readback gate stops timing what it cannot time and counts allocations instead (ADR 0052) |
| `a135181` | the plan and handover absorb the split |
| `fa69833` | **a tile's border column gets the area instead of a smear of it** — the pixel-moving commit, 2 684 differing pixels of 2 863 228 at worst 185 of 255 against a wider region |
| `28348a0` | **a clip chain's residue is rasterised once over the region it occupies** (ADR 0049): the artwork archetype's 600 rasterisations become 185, encode 46.3 → 37.2 ms |
| `f7b5152`, `70dea4a`, `a39bd34`, `1246940` | those documents, and the round measured on *this tree's* corpus — with the warning that their own `934/20` re-runs at `930/24` here, so a baseline must be re-taken on both sides |
| `5b30dc7` | **§7.3.3 defers the precision of a number to the machine, and Annex C is informative** — the clause research under the function-paint answer |
| `76b4da9` | the rest of that clause work |
| `f7ab2c8` | a spike evaluating a §7.10.5 program per fragment, both shapes this tree's §4 left to them, on both adapters |
| `850369f` | the numbers: a generated shader draws a whole page of the seven-segment witness in **0.060 ms** on RADV against **4 988 ms** on this tree's processor; the interpreter loses on throughput *and* on cold compile, and at 4× took the device down |
| `82683f9`, `688449d` | **their ADR 0053, proposed: yes — type 4 only, generated shader only** |
| `eeb441d`, `a64a908` | that record's name and its opening line |

### What it required of this tree, and what it suggested

**Required: nothing.** `Counters` gained four fields and `DeviceError` one variant, and both are
breaking changes in general — this tree survives them because it never builds a `Counters` and
always matches `DeviceError` with a `_` arm. ADR 0051's three-file split is the one worth checking
rather than assuming, and it is invisible here: every path this crate names still resolves.

**Suggested: the four counters.** Two were taken and two were not, and ADR 0367 §2 is the
argument. `atlas_repacked` is the one event outside this crate that can kill a retained encode —
the gap ADR 0351 left when it enumerated every input on *this* side of the boundary — and
`atlas_working_set_bytes` is the only number that makes a repeated repack actionable.
`clip_residue_regions` and `clip_residue_tiles` answer a **census** question and the window's
per-frame line is the wrong place for one; the ask stays open in `QUORRA_FEEDBACK.md` §25.

### The four lanes, A/B in one working copy

| | `580fa4ac` | `a64a9084` |
|---|---|---|
| scale 1, `cpu` | 930 / 24 / 2 / 18 | **931** / 23 / 2 / 18 |
| scale 1, `gpu` | 928 / 26 / 2 / 18 | **929** / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | 936 / 10 / 5 / 23 |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | 937 / 9 / 5 / 23 |

Two pages move and no verdict does at 4×: `issue2177.pdf` joins the oracle on both scale-1 lanes
(it was listed for a worst tile of 7.14 against a bound of 7.0), `issue11473.pdf` moves by 0.0001
of a mean and stays listed, and `issue6081.pdf` at 4× goes from a worst tile of 9.17 to 8.86 on
both lanes. **No refusal moved at any scale**, which is what a region living in host memory
predicts: the coverage sheet is unchanged, so the two pages that outgrow the adapter's 16 384²
texture outgrow it exactly as before. ADR 0367 has the page-by-page reading.

### The answer this tree was waiting for

`doc/QUORRA_FUNCTION_PAINT_ANSWER.md` is the reply to the ask of the same date, and it is a
**yes** with a shape this tree did not propose: not one of §5.1's three answers to the
oracle-arithmetic problem but a *static classification* — accept a program that reaches only the
exactly-agreeing operators and keep the oracle relationship exact, refuse by name any program
that can reach a transcendental on a path into a comparison. **Nothing is built on either side**,
and this round deliberately did not start it: what the answer converts is not a dependency but
the meaning of this tree's own corpus gate. `QUORRA_FEEDBACK.md` §25 prices it and names the two
defects of this tree's `pdf-model::function` that came back with it.

## `05fadc52`, taken in the five-hundred-and-forty-first session (2026-08-15)

Sixteen commits past `a64a9084`, and the first bump this tree has taken **for a feature it
asked for**: `doc/QUORRA_FUNCTION_PAINT_BUILT.md` is upstream's account of what they built and
what it costs to adopt, and it supersedes the "not committing to yet" list of
`…_ANSWER.md` §7 while **correcting that document's §4**. ADR 0376 is this side's adoption.

**It still cost nothing to compile**, which is four in a row — two hashes in `Cargo.lock`,
`check --workspace --all-targets` clean — and that is despite two changes that are breaking in
general. What it cost instead was the boundary work to *use* it, which is the whole of ADR 0376.

The range, oldest first:

| | |
|---|---|
| `1ee1884` | `FnOp`: the vocabulary a compiled type 4 function arrives in — Table 42 closed, `if`/`ifelse` already lowered to forward-only jumps, and the three typed literals this tree's ADR 0371 made expressible |
| `17de1d3` | every Table 42 operator gets a case that says where its value came from |
| `6e61697` | their notes separate what the clause says from what they read into it |
| `a490b20` | the scene says which programs a function paint may carry (`check_program`, `MAX_PROGRAM_LENGTH`) |
| `c820449` | a function paint that reaches today's encoder is refused by name rather than drawn wrongly |
| `ae5bc0c` | a compiled program becomes a resource, and its domain a region — `FunctionId`, and `ResourceId` gains its fifth variant |
| `2d711cc` | **the admission**: `quorra_gpu::function::admit`, which needs no adapter, so a caller learns before it builds a scene |
| `a8414f4` | `FnRange` — one name for the bounds *and* the component count, so the two cannot disagree |
| `d7e4bf8` | the analyser admits a program and the generator writes its shader |
| `102ca0a` | their conformance corpus and the oracle that judges it |
| `80bf787` | **`Agreement::Exact`/`Approximate` becomes `Bounded`/`Unbounded`** — the correction, and the reason for it |
| `eb6af7e` | the paint reaches the pass that draws it |
| `dc8bf60` | the corpus judges the device the programs run on |
| `3fc7e07`, `05fadc5` | a function paint uploaded, admitted, compiled and drawn; and the documents |

### What it required of this tree

**To compile: nothing.** `Paint` gained a fourth variant and `ResourceId` a fifth, and both are
breaking changes in general; this tree survives them because **no `match` here scrutinises
either type** — `quorra_scene::Paint` is only ever constructed, and a `ResourceId` is produced
by `From` and consumed by `Device::release`. `DeviceError`, `RenderError` and `ReportKind` are
handled through `thiserror` and one single-variant pattern, so their new variants were free too.
`Paint` keeps its `Copy`, as their §2 promised.

**To *use*: a boundary.** `render-quorra` sees `pdf-render` and the compiled program lives in
`pdf-model`, so the display list had to gain a device-facing statement of a §7.10.5 program —
`pdf_render::program`, and `ShadingKind::Sampled { program: Option<ShadingProgram> }` beside the
producer that was already there. ADR 0376 §1 is why that form is not `pdf_model::function`'s.

### What it did on this machine

**Not one corpus verdict moved**, at either scale, on either coverage lane: 931 / 23 / 2 / 18 at
scale 1 on the `cpu` lane, and the other three lanes on their own previous numbers. The census
says why: of 1 479 corpus files, **three** pages carry a §8.7.4.5.2 program at all and **one**
carries a program the device accepts.

| `function_based_shading.pdf`, both arms in one working copy, RADV | grid | device |
|---|---:|---:|
| mean, scale 1 | 0.0178 | 0.0392 |
| worst tile, scale 1 (bound 7.0) | 1.171 | 1.201 |
| mean, scale 4 | 0.0047 | 0.0191 |
| worst tile, scale 4 | 1.555 | 1.582 |

Eight of that page's nine shadings are evaluated on the device; the ninth is refused for
`` `mod` was given a real, and requires two integers ``.

### The two witnesses the ask was written about are refused, and correctly

`pi_seven_segment.pdf` and `type4_pi.pdf` — the pages whose whole content is one `sh` and whose
scene build is a hundred milliseconds of function evaluation — are `Agreement::Unbounded`: a
`div` reaches a `truncate` in both, which is a digit of π decided by a last bit. So the frame
line on the document that prompted the ask is unchanged. That went back to them in
`QUORRA_FEEDBACK.md` §27; it is not a defect on either side, and it is worth their knowing that
the classification's cost is exactly the population that asked for the feature.
---

## `619ef3b4`, taken in the five-hundred-and-forty-second session (2026-08-15)

Thirty-two commits past `a64a9084` (thirty-three with the one merge), and **the fourth bump in a
row that cost this tree not one line of source to compile**: two hashes in `Cargo.lock`,
`build --workspace --all-targets` clean, `clippy --workspace --all-targets` silent. What this round
then *chose* to write is `render_quorra::options()` and ADR 0377 — the release's last two commits
are the phase this tree asked for in `doc/QUORRA_ENCODE_THREADS.md`, and a permission is not taken
by taking the release.

The range, oldest first, merges elided, in the three groups it actually falls into:

| | |
|---|---|
| `1ee1884` … `05fadc5` (12) | **the function paint, built**: a §7.10.5 type 4 program's vocabulary, Table 42 operator by operator; the scene's `Paint::Function`; a program uploaded, admitted or refused by name before any frame; a compiled program as a resource with its domain as a region; the generated WGSL through the same coverage, clip and mask weighting as every other paint; their own conformance corpus over it (their ADR 0053). `doc/QUORRA_FUNCTION_PAINT_BUILT.md` is what a host must do to *emit* one, and nothing here emits one yet. |
| `80bf787` | `Agreement::Exact`/`Approximate` become **`Bounded`/`Unbounded`** — their own correction, and the one line of that work this tree should read twice: WGSL §15.7.5 permits reassociation and fusion, so "exact" was overclaimed and withdrawn before we could build a gate on it. |
| `4ce6476` … `3f819c1` (13) | **the device file split**: the ramp arithmetic, the textures, a pass's bindings, the rare lane's quad, the target, the damage list, phase 2's staging, phase 3's routes, the resource's two forms, construction and its cost — one file each, in the order a device lives, plus the record of the seams. Public paths unchanged, which this tree is an independent check on rather than a beneficiary of. |
| `9a5a70f` | their debt list stops naming what was paid and names three things found while paying it |
| `8298a8c`, `a3f09f1`, `619ef3b` | **the geometry phase divides across the threads a host allows** (their ADR 0054) — `Options::encode_threads`, `std::thread::scope` inside `Device::render`, no dependency and no `unsafe`, a 4 096-segment floor under it, and byte equality at 1, 2, 3, 7 and 64 threads |

### What it required of this tree, and what it asked

**Required: nothing.** `Options` gained a field with a default, which is the one kind of addition a
struct without `#[non_exhaustive]` can make without breaking a caller, and every path
`render-quorra` names still resolves through the thirteen-file split.

**Asked: a number.** `encode_threads` defaults to 1 and upstream calls that *a permission rather
than a preference* — the host's decision because only the host knows what else is running. ADR
0377 takes it: `render_quorra::options()` is the one place it is chosen, the value is
`std::thread::available_parallelism`, and `crates/render-quorra/examples/encode_threads.rs` is the
instrument that chose it. The ladder on the owner's document, RADV, cold device per sample, minima
of five round-robin rounds:

| threads | quiet (load 3.8) | busy (load 10 → 16) | oversubscribed (load 22 → 33) |
|---:|---:|---:|---:|
| 1 | 467.2 ms | 849.8 ms | 1376.0 ms |
| 8 | 221.0 | 315.5 | 667.3 |
| 24 | **150.6** | **251.8** | **458.7** |

Upstream's caution — an earlier round of theirs read 24 threads as *worse* than 8 at load 25–33 —
**did not reproduce on this machine at any load it could be put under**, which is why the number is
read off `available_parallelism` at run time rather than written down as a constant here.

### The four lanes, and the eight runs the determinism claim needed

`doc/todo/02-every-round.md` §2's four-lane debt, run whole — and then run *again* at one thread,
because the property that matters about a thread count is that it changes nothing:

| | 24 threads | 1 thread |
|---|---|---|
| scale 1, `cpu` | 931 / 23 / 2 / 18 | identical, verdict line by verdict line |
| scale 1, `gpu` | 929 / 25 / 2 / 18 | identical |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | identical |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | identical |

Every row is also identical to ADR 0367's at `a64a9084`: **no page moved on the release, and no
page moved on the threads.** `REFUSED_AT_FOUR` is unchanged at both scales, which is the ratchet
`QUORRA_ENCODE_THREADS.md` §5 named as the one a parallel phase would break if it changed what a
frame commits.

### The frame path on the owner's document, before and after

`tmp/Entwurf.pdf` under `Xvfb`/llvmpipe, ADR 0368's script, arms alternated A A B B A around one
rebuild each way. The two magnification frames — identified by their cull counts, 8763 and 17986,
which reproduce ADR 0368's exactly — go from 608.2 and 514.6 ms at one thread to 295.0 and 274.7 at
twenty-four, and the return to the fit view from 937.8 to 314.1. **The structure is unchanged**:
`host` 0.0, `scene` 14–22 ms, `settle` 1–2 ms, 40 resources uploaded, the same cull counts; the
whole of the difference is inside `device`. **The launch table did not move** — `graphics device`
reads +30.4 and +27.9 ms at one thread against +35.3, +22.8 and +30.9 at twenty-four — which is the
check upstream's "nothing is built at construction" deserved rather than the repetition of it.
ADR 0377 has every row.

---

## `a4380e2c`, taken in the five-hundred-and-forty-seventh session (2026-08-16)

Thirty-three commits past `619ef3b4`, all of them from one day, and **the fifth bump in a row that
cost this tree not one line of source**: two hashes in `Cargo.lock`, `build --workspace
--all-targets` clean, `clippy --workspace --all-targets` silent, `fmt --check` clean. This one is
the easiest of the five to state, because upstream did not change the public surface at all —
checked from this side rather than taken on trust, by diffing every `pub` line of both crates
across the range: **not one public item was added, removed, renamed or resignatured, and
`quorra-scene` was not touched at all.**

The range, oldest first, in the four groups it falls into:

| | |
|---|---|
| `7c0a248`, `be324ca`, `fa29da1`, `f82ec08`, `063734b` (5) | the documentation gate and the uniform gate: `cargo doc` under `-D warnings` in their CI, and `src/shaders/layout.rs` — a `#[cfg(test)]` deriver that checks every host-side uniform writer's field offsets against the WGSL struct it mirrors, where wgpu had been checking the total size only |
| `3785b00`, `2f25397`, `48d25fa`, `4b1bbed`, `d2a48ad` … `3965c30`, `1015f32` (13) | **the encode walk split**, the sequel to the device split this tree took at `619ef3b4`: `encode.rs`'s 2 406 lines become 435 plus eleven private submodules named for the eleven things the walk does, and `tests/retained_frame.rs`'s 1 139 lines become five files. Public paths unchanged, again a claim this tree checks independently rather than benefits from |
| `d05036f`, `d61b6a2`, `1eba41f`, `e6d0e1d` (4) | **the one pixel-moving change**: a colour ramp's coincident stop offsets, read against ISO 32000-2 §7.10.4 rather than extrapolated (their ADR 0055) |
| `a2afb92`, `d64bc71`, `a4380e2` (3) | the tiling ceiling measured and **not** fixed — `doc/notes-tiling-ceiling.md` states in as many words that no `src/` file was changed by that round — plus `tests/tiling_ceiling.rs`, which witnesses the existing behaviour through the public API |

Also in the range and invisible here: `04b1c8f` and `97013f8` (a generated function shader's compile
cost measured — 8.25 ms on RADV for a 482-instruction program and **2.0–2.7 ms for a
one-instruction one**, which is the fixed `function_lane.wgsl` parse and build rather than the
generated part), `0e7923f` (their debt list stops naming five paid debts) and `f7f8785` … `c27fb43`
(the encode split's individual seams).

### What it required of this tree

**Nothing, and for once that is the whole story rather than the headline over a boundary.** No
`match` here had to grow an arm, no field had to be read, no option had to be chosen. `Counters`,
`Frame`, `Timings`, `Options`, `DeviceError`, `RenderError`, `RetainedScene` and `Device` are
byte-for-byte the same public types; `crates/quorra-gpu/src/frame.rs`, `error.rs` and `startup.rs`
are untouched in the range.

### The one pixel-moving change, read before it was taken

Their ADR 0055 is the kind of change this tree has to look at rather than accept, because it moves
a mark: `ramp_color_at`'s loop comparison goes from `<=` to `<`, so a `t` landing exactly on a
stop's offset now belongs to the interval that **starts** there. A colour ramp is a shading's
colour function already sampled onto stops, and a producer puts *two stops at one offset* wherever
that function jumps — which is §7.10.4's stitching bound. The clause makes the subdomains

> half-open intervals, closed on the left and open on the right

with two exceptions that point in **opposite** directions: the last interval is closed on the right
(so a coincident pair at the ramp's final offset takes the later colour), and where `Domain0 =
Bounds0` the first interval is closed on both sides (so a coincident pair at offset 0 takes the
earlier one). Their code now does all three, and the reasoning is the clause rather than another
renderer — which is the only ground on which this tree can take a pixel change at all.

### The four lanes, and this release A/B'd on two of them

`doc/todo/02-every-round.md` §2's four-lane debt, run whole; and because the release moves a mark,
the two scale-4 lanes were run **twice in one working copy**, flipping only `Cargo.lock`, so that
what moved is attributed rather than inferred.

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

Every other judged line of both lanes is character-identical between the two pins. That is upstream's
own measurement of our corpus reproduced here — they predicted this page, this direction and these
figures — and it moves **toward** the oracle on all four of the numbers the comparison carries.

**Nothing moves at scale 1**, and that is proved rather than counted: the scale-1 `cpu` lane holds
*both* the refusal list and the differing list to equality, and it passed, so no page changed
category there. `issue10572.pdf` does not appear in either scale-1 lane's judged output at either
pin — at one device pixel per point the ramp bound falls inside a pixel that was already right.

### The scale-4 rows are one page away from ADR 0377's, and the page is ours

ADR 0377 and `QUORRA_FEEDBACK.md` §27.2 record `936 / 10 / 5 / 23` and `937 / 9 / 5 / 23` for the
two magnified lanes. Both are correct **for the session that wrote them** and both are stale now,
by exactly one page in one direction: `22060_A1_01_Plans.pdf` left `REFUSED_AT_FOUR` and joined the
differing list in the **five-hundred-and-forty-third** session, when `image::RasterCache` stopped a
page decoding one `XObject` thirty-six times and the page stopped exceeding `max_resource_bytes`
(ADR 0374). The ratchet was moved in that same commit, which is why every run since has passed; it
is the *summary numbers* in the older documents that no round re-read.

Worth naming as a shape rather than as an erratum: **a ratchet held to equality and a count written
in prose decay at different rates**, and the ratchet is the one that cannot go stale silently. The
run at the old pin is what settles this — 936 / 11 / 4 / 23 at `619ef3b4` too — so the release is
not what moved it, and neither is anything upstream did.

## `eada81ec`, taken in the five-hundred-and-fifty-sixth session (2026-08-17)

Five commits past `a4380e2c`, and **the sixth bump in a row that cost this tree not one line to
take** — but the first in a long while that is not "nothing changed": it is the release that
answers `doc/QUORRA_NONBLOCKING_RENDER.md`, and this tree then spent a whole round *being its
caller*. The two facts are worth keeping apart. Taking the release is two hashes in `Cargo.lock`
and a `check --workspace --all-targets` that passes with no edit at all, because **the API change
is purely additive**. Using it is ADR 0391.

The range, oldest first:

| | |
|---|---|
| `44d7acf` | their release matrix — a record of what a push delivered, nothing in `src/` |
| `3073c7e` | **`recording` measured with callgrind and subdivided**, the answer to our §9. No `src/` change; `doc/notes-recording-shares.md` is the whole of it |
| `bf5044e` | **the surface leaves the device**: `Presenter`, `Layer`, `PresentCost`, `detach_presenter` / `attach_presenter`, `present.wgsl`, and `RenderError::PresenterDetached` / `PresenterUnsized` / `LayerRefused` (their ADR 0056) |
| `aead796` | `examples/present_thread/` under `Xvfb` in their CI — a page rendered on a second thread while the main thread presents, read back with `xwd`, verified able to fail three ways |
| `eada81e` | the reply to carry across, and their own record of the round |

### What the bump required: nothing, and the check is the diff rather than the build

`cargo check --workspace --all-targets` passes against `eada81ec` with **no source change in this
tree**, and the reason is in the `pub` surface: every item this tree already named is
byte-for-byte what it was, and everything new is new. `Device` gained two methods,
`RenderError` gained three variants, `LayerProblem` is a new enum, and `quorra-scene` was not
touched at all. Nothing was removed, renamed or resignatured.

**A `#[non_exhaustive]` note worth stating rather than discovering later**: `RenderError` gained
`PresenterDetached`, `PresenterUnsized` and `LayerRefused`, and this tree matches on it in
`viewer-ui`'s `surface.rs`. The match was already exhaustive by way of a catch-all arm — which is
why it compiled — and the arm's behaviour is right for all three: each becomes a
`Refusal::DeviceRefused` naming what quorra said, which is exactly what a host that has just
misused the presenting API should be told.

### The four lanes, unmoved to the digit

`doc/todo/02-every-round.md` §2's lanes, run whole on the real Radeon 890M under RADV. **A round
that takes a quorra release owes the magnified lane** (§2's note, and ADR 0283's precedent), and
here it is beside the two at scale 1:

| | `a4380e2c` (recorded) | `eada81ec` (this round) |
|---|---|---|
| scale 1, `cpu` | 931 / 23 / 2 / 18 | **931 / 23 / 2 / 18** |
| scale 1, `gpu` | 929 / 25 / 2 / 18 | **929 / 25 / 2 / 18** |
| scale 4, `gpu` | 937 / 10 / 4 / 23 | **937 / 10 / 4 / 23** |

Agree / differ / refused / not comparable, and **every judged line is character-identical** to the
pin before it. That is what a release with no pixel-moving change should look like, and it is
checked rather than assumed: `bf5044e` adds a `present.wgsl` no headless device can reach, and
`3073c7e` changed no `src/` file at all.

### The one thing this tree had to check for itself, and it is not a compile question

**Whether the presenting path can reach a golden.** quorra's own answer is in their reply's §6:
a `Presenter` is reachable only from a device built with a surface, and this tree's corpus gate
and both oracle lanes have none. This tree's half is `doc/todo/37` rule 2, and ADR 0391 §5 is the
audit: the offscreen rasteriser did not move, `render_quorra::present::build` gained an
`Option<Color>` medium whose every existing caller passes `Some(background)`, and the test that
walks every `.rs` outside `viewer-ui/src/bin` for the word *reprojection* still passes. The three
lanes above are what turns that argument into a measurement.

### What the release makes available and this round did **not** take

- **`PresentCost::compiled`**, which says when a present compiled the presenting pass inline
  because the warm-up thread had not reached it. This tree reads the three wall clocks and not
  this; a launch round that wants to know whether the *first* present paid a shader has the field
  waiting for it.
- **`Presenter::size()`**, an addition their answer says we can ignore, and we do: this host tells
  the presenter its size on every `Resized` and keeps its own copy for the layer arithmetic.
- **`ForeignPresenter::into_presenter`**, the recovery on a mis-attach. This tree detaches once and
  never attaches back, so the path is unreachable here — which is itself the reason it costs
  nothing to have.

## `cad50156`, taken in the five-hundred-and-seventy-sixth session (2026-08-18)

**121 commits past `eada81ec`**, 92 of them non-merge, with fourteen new ADRs of theirs — the
largest range this tree has ever taken in one bump, and **the seventh in a row that cost it not
one line to take**. `cargo build -p render-quorra --all-targets` passes against `cad50156` with
no source change at all: nothing was removed, renamed or resignatured, and everything new is new.
The only edit the release *required* is a ratchet coming off, which is the opposite of a cost.
ADR 0411 is the argument; this section is what a reader of the correspondence needs.

### What is in it, out of 32 000 changed lines

Most of the range is the library's own tests, notes, ADRs and module splits — `error.rs` into
seven files, `raster.rs` into three, `pipeline.rs`, `geom.rs`, `outline.rs`, all separately
verified upstream as pure code moves and confirmed null over this corpus by their own matrices.
Four things reach this tree:

| | |
|---|---|
| `cafadeb`, their ADR 0057 | a clipped mark's coverage tile is bounded by **its chain's own device box** rather than by the open clip rectangle, and a refused frame names the *sheet* it met rather than only the adapter's wall |
| `c443bc2`, their ADR 0070 | a mark whose thin axis is under the device coverage lane's sample-column spacing **keeps the processor lane** — §10.7.4's first requirement, on the lane that was breaking it |
| `1adf479`, their ADR 0066 | a soft mask is a knockout element's **opacity**, not its shape. A no-op for this translator *by construction*: `stated_shape` removes the mask and the constant before the shape half reaches quorra (ADRs 0234, 0327), and their matrix measured that the corpus reaches the path — 16 documents emit a knockout group, 5 a `Shaped` command, and in all six `Shaped` commands the shape half carries no soft mask |
| `b5a09d7`, their ADR 0069 | a *group* used as an element of a knockout group is refused by name. This tree refuses `issue18032.pdf` one crate earlier, in `render-quorra/src/scene.rs`, so their variant reaches no corpus page here |

Plus `SceneError::InvalidImageAlpha`, `RenderError::ViewportTransformTooLarge`,
`Counters::{coverage,lanes,atlas_overflow_tiles}`, `Limits::atlas_bytes`, ADR 0058's present
rectangle, and a new `quorra-pages` member crate this tree does not depend on.

### The four lanes, and the two that moved

Both revisions run here side by side, one sitting, real Radeon 890M under RADV, `--profile gates`:

| lane, scale | `eada81ec` | `cad50156` |
|---|---|---|
| scale 1, `cpu` (the default gate) | 932 / 23 / 2 / 17 | **932 / 23 / 2 / 17** |
| scale 1, `gpu` | 930 / 25 / 2 / 17 | **932 / 23 / 2 / 17** |
| scale 4, `cpu` | 937 / 11 / 4 / 22 | **938 / 11 / 3 / 22** |
| scale 4, `gpu` | 938 / 10 / 4 / 22 | **939 / 10 / 3 / 22** |

Agree / differ / refused / not comparable. The default lane is character-identical across the
whole bump — **which is exactly why `doc/todo/02` §2 asks a release round for the other lane**.
On the `gpu` lane at scale 1 the two columns were `diff`ed line by line: exactly two lines leave,
`bug1883609.pdf` and `vertical.pdf`, and every other line is identical to the character. At 4×
`issue12295.pdf` moves toward the oracle without reaching it (mean 0.9517 → 0.9201, differing
0.0490 → 0.0473, similarity 0.95585 → 0.95881, worst tile unmoved at 16.31), so that row's totals
alone would have read as null.

The refusal that moved is `bug1703683_page2_reduced.pdf`, off the 4× list on both lanes and
**agreeing with the CPU oracle** when rendered alone. `REFUSED_AT_FOUR` is three names now. Three
separate upstream messages had said this name could come off; ADR 0411 §2 is why none of them was
what took it off.

### The one question the release asks, answered

`doc/api-change-image-alpha.md` puts a decision to this tree rather than taking it: whether
`SceneError` should be `#[non_exhaustive]`, since it is not, and the next variant would break a
downstream `match` with no wildcard arm.

**Yes for `SceneError` and `RenderError`; no for `SurfaceProblem`.** This tree reaches the first
through one `#[from]` and matches it nowhere; it matches `RenderError` in `viewer-ui`'s
`surface.rs` behind a catch-all that reports whatever quorra said. Both are open-ended
vocabularies of refusals and marking them costs this caller nothing, now or later.
`SurfaceProblem` is the opposite: that same `match` covers all five variants **with no wildcard**,
because a swapchain state is a decision rather than a report — two ask for a redraw, two for
nothing, one for a failure the person is told about. Its own module comment says its completeness
is `wgpu`'s rather than quorra's, so if `wgpu` grows a sixth arm the right thing is that this tree
**fails to compile**, and `#[non_exhaustive]` is what would take that away.

The rule, if a third enum ever raises the question: mark the one whose variants a caller
*reports*; leave exhaustive the one whose variants a caller *decides on*.

### What this round did not take, and one thing only the owner can run

- **No timing.** Two lanes were run twice and this desktop was busy; which pages refuse is
  arithmetic, which lane is faster is not.
- **The presenting path.** ADR 0058's layer rectangle and `present.wgsl` are reachable only from
  a device built with a surface, which no gate in this tree has. `doc/environment.md` puts that
  in the owner's session.
- **`quorra-pages`.** A new member crate; this tree names `quorra-gpu` and `quorra-scene` and has
  no reason yet to name a third.
