# 0367 — The release that answered an ask, and the counter a retained frame was missing

**Status.** Accepted.

## Context

Upstream moved twenty-eight commits past `580fa4ac`, the revision ADR 0351 pinned. The project
owner reported the release and said it **may require or suggest an API move**, which the two
bumps before it did not: `87898c69` and `580fa4ac` each cost this tree not one line of source.

Three things arrived at once and they are separable, which is why this ADR treats them
separately:

1. a library release with four ADRs of its own (0049–0052), one of which **moves pixels**;
2. `doc/QUORRA_API_2026_08_15.md`, written by the renderer side *for* this side — the first
   time a bump has arrived with its own migration document rather than a section of
   `QUORRA_UPGRADE.md` written here afterwards;
3. **the answer to `doc/QUORRA_FUNCTION_PAINT.md`**, this tree's ask of 2026-08-15, as
   `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` and upstream's ADR 0053. That answer is *yes*, and it
   is not built on either side.

## Decision

**Take `a64a9084`. Adopt two of the four counters it adds. Build nothing toward the function
paint in this round, and record the answer where the ask lives.**

### 1. The bump itself required nothing, for the third time running

`cargo update -p quorra-gpu -p quorra-scene`, two hashes in `Cargo.lock`, then
`cargo build --workspace --all-targets` clean and `clippy --workspace --all-targets` silent with
no source touched. This is worth stating rather than assuming, because the release contains
upstream's ADR 0051 — `scene.rs` split into seven files, `compose.rs` into six, `winding.rs`
into four. A split that re-exports its parts is invisible to a caller by construction, and this
tree is the check on that claim rather than the beneficiary of it: **the public item set is the
same and every path this crate names still resolves.**

`DeviceError` gained `ResourceIdsExhausted` and `Counters` gained four fields. Neither breaks
here, and the reason each does not is worth naming, because both *are* breaking changes in the
general case: this tree matches `DeviceError` with a `_` arm and never constructs a `Counters`.

### 2. Two of the four counters are taken, and two are deliberately not

Upstream's §1 says "these four belong in the same place" as the three `present.rs` already
copies. **Two of them do, and the argument is ADR 0351's rather than upstream's.**

`FrameSlot` is the only thing in this crate that can draw a page other than the one it was
handed, and ADR 0351's consequences section says why every entry of its key is enumerated and
separately tested: *a frame loop that means to reuse and does not is the one defect it can
have.* That enumeration was complete for **this** side of the boundary and silently incomplete
for the other. An atlas repack throws every tile placement away, which makes the retained
encode stale — and until this release there was no way to observe it. A window whose frames
re-encode for ever looked exactly like a scene key that keeps missing, and the two have
opposite fixes.

So:

- **`atlas_repacked` is taken.** It joins the frame line as a word appended only when true —
  the same rule `fallback` and `attend` already follow, and a repack is exactly the strange
  frame that rule exists for — and the summary counts it beside the replay count it explains.
- **`atlas_working_set_bytes` is taken**, and only because the repack is not actionable
  without it. A repack repeating frame after frame means either that the atlas is holding
  another page's tiles or that this page has never fitted the budget the device was built
  with; the working set is the number that tells those apart, and `Options::atlas_budget` is
  the lever it points at.
- **`clip_residue_regions` and `clip_residue_tiles` are not taken**, with the cost written
  down. They answer `QUORRA_FEEDBACK.md` §15 from upstream's side and upstream explicitly
  wants the shape back — a page reporting `0` regions and `40` tiles is what their next lever
  is for. But that is a **census of the corpus**, and the place a census belongs in this tree
  is the walk that produced their `doc/corpus-profile.md`, not the window's per-frame line. A
  field read into `FrameCost` and never printed is dead weight, and a field printed on every
  frame to answer a question about the corpus is the wrong instrument in the wrong place. The
  cost of declining is that upstream's ask stays open one more round; it is recorded in
  `QUORRA_FEEDBACK.md` §25 with the two numbers it wants.

`Timing::summary` grew past `clippy::pedantic`'s hundred lines when the repack line landed, and
the seam it was split on is the one that had been there all along: everything about what the
*retained frame* did over a run is now `Timing::retention`, which is the paragraph the new
line belongs to.

### 3. One page moves, on both lanes at page scale, and it moves toward the oracle

Upstream's ADR 0049 fixed a defect in their coverage rasteriser. `fill_mask` computes a shape's
coverage over a rectangle of device pixels; where an edge entered that rectangle from outside,
they clamped the piece's two endpoints to the border and interpolated between them. That
preserves the row's *total* winding — every column past the crossing reads the right value,
which is why neither side's tests saw it — while handing the columns *at* the border the height
the piece spent outside. They cut the piece at the border now.

**Only a tile that a clip or the page edge cuts in x can move at all**, and the corpus says so:
one page of 956 moves at page scale on **both** coverage lanes, and nothing at all moves at 4×.
`issue2177.pdf` joins the CPU oracle — it was listed for its worst tile, 7.14 against a bound
of 7.0, which is the churn-at-the-bound `DIFFERS_AT_THE_EDGES` was written to hold. It leaves
that list, which is the direction the list is allowed to move in.

The measurement is an A/B in one working copy with only `Cargo.lock` between the arms, because
this tree's own last recorded numbers are two sessions old and upstream warned in as many words
not to compare against theirs: their `PLAN.md`'s `934/20` re-runs at `930/24` against this tree
on this date, and *this tree* moved, not theirs. Both arms of every table in the evidence below
were run within the same hour.

### 4. The function paint: the answer is recorded and priced, and nothing is built

`doc/QUORRA_FUNCTION_PAINT.md` asked for a device-evaluated §7.10.5 program. The answer is
**yes — type 4 only, generated shader only, nothing built yet**, and it resolves §5.1's
arithmetic question in a way this tree did not offer: not one of the three answers §5.1 listed,
but a **static classification at admission** — a program reaching only the exactly-agreeing
operators is accepted and the oracle relationship stays *exact*, and a program that can reach a
transcendental on any path into a comparison is refused by name and falls back to the raster
this tree builds today.

**That is a better answer than any of the three, and it is not this round's to take.** §5.1 of
the ask says a device that evaluates the function raises the oracle-arithmetic question, and
the answer converts that question from a tolerance into a *classifier* — which is a decision
about what this tree's corpus gate means, not a dependency bump. The price is recorded in
`QUORRA_FUNCTION_PAINT.md`'s own record and in `QUORRA_FEEDBACK.md` §25; the work is not
started.

**Two defects in this tree came back with it, and they are not this round's either.** Upstream
read `pdf-model/src/function.rs` to learn the compiled form and found `Operator::Round` using
Rust's half-away-from-zero rounding where PLRM3 — normative through §7.10.5.2 — requires
half-toward-greater, and `Operator::Eq`/`Ne` comparing with an `f32::EPSILON` tolerance where
PostScript `eq` is exact. Both are real and both are cheap. They are left out of this round for
one reason and it is a measurement reason rather than a scheduling one: **each of them changes
what a function evaluates to, and this round's whole evidence is that exactly one page moved
and that upstream's rasteriser is why.** A semantic change to `pdf-model` in the same commit
would make that attribution unrecoverable. They are written into `QUORRA_FEEDBACK.md` §25 as
this tree's own, with the clause each is judged against.

## Evidence

### The upstream range, oldest first, merges elided

In `doc/QUORRA_UPGRADE.md`'s `a64a9084` section, one line per commit.

### The four lanes, A/B in one working copy, `Cargo.lock` the only variable

| | `580fa4ac` | `a64a9084` |
|---|---|---|
| scale 1, `cpu` | 930 / 24 / 2 / 18 | **931** / 23 / 2 / 18 |
| scale 1, `gpu` | 928 / 26 / 2 / 18 | **929** / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | 936 / 10 / 5 / 23 |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | 937 / 9 / 5 / 23 |

agree / differ / refused / not comparable. **No refusal moved at any scale on either lane**,
which the mechanism predicts: a region is host memory and never reaches the coverage sheet, so
the two pages that outgrow the adapter's 16 384² scratch image outgrow it exactly as before.

Page by page, and this is the whole of it:

- **`issue2177.pdf` leaves the differing list on both scale-1 lanes.** It read mean 1.1168 /
  worst tile 7.14 on the `cpu` lane and 1.0992 / 7.14 on the `gpu` one; it is absent from both
  afterwards. The `cpu` lane's ratchet was updated by removing the name, which is the one source
  line this release cost a *test*.
- **`issue11473.pdf` moves and stays listed**: mean 0.1003 → 0.1004, worst tile 10.04 → 10.07,
  identically on both lanes. Upstream predicted this page by name and by magnitude.
- **Every other differing page is identical to the character on both lanes**, mean, worst tile,
  differing fraction and similarity alike.
- **At 4× the differing lists are unchanged.** `issue6081.pdf` reads worst tile 8.86 on the
  `gpu` lane in both arms — upstream reported it moving 9.17 → 8.86 against *their* baseline,
  which this tree passed through before this round.

The 1× `gpu` lane is a survey rather than a ratchet (the two lanes deliberately do not draw the
same pixels, quorra's ADR 0016), so its base had to be measured rather than read: 928 agreeing
at `580fa4ac`, not ADR 0351's 933, because ADR 0355 and its successor moved four pages on both
lanes in the two sessions between. That is why the A/B is the instrument and the documents are
not.

### The retained frame still replays

`crates/render-quorra/tests/retained_frame.rs`, eight tests, on the real device: byte identity
against the encode replaced and against a cold device, one test per key input asserting the miss
*and* that what is drawn afterwards is the new page, chrome merely rebuilt still hitting, and a
raster stand-in never replaying. All eight pass at this pin.

ADR 0351's frame-structure check on `tmp/Entwurf.pdf` under `Xvfb`/llvmpipe, `--trace`,
structure only — no wall-clock claim, the machine is shared — is in the history file with its
counters. The three that carry the claim are the ones ADR 0351 named: frames replayed of frames
presented, uploads, and the bytes the handle held.

### What did not move

`cargo fmt --all --check`, `clippy --workspace --all-targets` silent, `cargo nextest run
--workspace`, `cargo test --workspace --doc`, `conformance`, the corpus, the oracle, both text
gates, and all four quorra lanes — the numbers are the history file's, not this one's.

## Consequences

- **The window can now say why a retained frame died.** Before this, the only two states a trace
  could distinguish were `encoded` and `replayed`; the cause of an `encoded` that should have
  been a `replayed` was either in `FrameSlot`'s key — where every entry has a test — or invisible.
  It has one more name now, and it is the device's.
- **The 1× `gpu` lane has a measured base again**, which it had not since ADR 0351. It is still
  a survey and still not a ratchet, and that is the right shape: the two lanes' differing lists
  are properties of the lanes.
- **Upstream is owed two answers and this round sends neither**: the rectangle census their §6.1
  asks for, and whether this tree can draw the page and the overlays as two `render` calls into
  one target. Both are in `QUORRA_FEEDBACK.md` §25 as open, with what an answer has to check.
- **`Options::instrument_encode` stays unused**, now for the third recorded reason and none of
  them new: upstream attributed the encode with callgrind on their side, a replayed frame's
  subdivision is zero by construction, and nothing in this tree's frame path is waiting on it.
