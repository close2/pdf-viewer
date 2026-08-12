# ADR 0291 — A half that is not a group is drawn in one

Date: 2026-08-12 (session 456)
Status: accepted

## Context

Session 439 wrote §11.4.6's translation for `render-quorra` and found it was not writable (ADR
0275): `quorra_scene::Compose::DestOut` and `Plus` existed, and the one position
`pdf_render::Command::Shaped` occurs in — inside a knockout group — was one of the two positions
the builder refused, while `SceneBuilder::group`, `stroke` and `image` carried no compositing
operator at all. `doc/QUORRA_FEEDBACK.md` §14.2 asked for the two lifts. Both came back:
quorra's ADR 0032 deletes `StagedComposeReason::InsideKnockoutGroup`, and its ADR 0033 puts a
`compose` on `GroupSpec`, at `6f777e8` and `2c9bdd0`.

This round took the bump and wrote the translation. The clause's arithmetic was settled three
sessions ago and is not the decision here; **what to do with the halves that are not groups is.**

## What the clause states, and what the vocabulary carries

§11.4.6's per-element rule, on the transparent initial backdrop an isolated group is built on,
is one line per pixel in premultiplied form — with the accumulated result `P`, the element's
shape `f` and its premultiplied colour `S`:

```text
P' = (1 − f) × P + S
```

Destination-Out with the shape, then Plus with the object. `Compose::Src` cannot say it: that
operator reads the shape off the coverage a mark is drawn with, which is the assumption a
`Shaped` element exists to contradict.

Where the halves are groups, §11.3.7.2 is why:

> The shape of a group object shall be the union (as defined in 11.3.7.3, "Result shape and
> opacity") of the shapes of the objects it contains.

No fill states that, which is what quorra's ADR 0033 answers.

**But `Compose` sits on exactly two things in that vocabulary — `SceneBuilder::fill` and
`GroupSpec` — and a `Shaped` element's object may be neither.** It may be a stroke, an image, or
a fill whose paint is a sampled shading, which `render-quorra` draws as an *image* clipped to the
path (`Encoder::sampled_fill`). A translation that stated the operator per mark would have been
correct for a solid fill and silently wrong for those three, which is the failure mode this
project names in trap 5 and pays for most often.

## Decision

**One rule, not two constructions: a half that is already a group states the operator on its own
`GroupSpec`; every other half is drawn inside a group of one element that states it.**

An isolated group at alpha 1, blend Normal, no mask and no clip of its own holds exactly its one
element's premultiplied colour — so compositing that group *is* compositing the element, and the
two routes are the same arithmetic rather than two approximations of it. The element keeps its own
clip and mask, which the walk inside the group applies; restating them on the wrapper would
multiply each in twice, which is what §11.6.4.3's NOTE 2 warns a *file* against.

`Encoder::group` now takes a `GroupParts` and a `Compose`, and the page's own groups pass
`SrcOver` — so there is one place in this backend where a group becomes a scene, and the staged
halves are that place called with a different operator.

**The cost is one buffer per staged mark**, on four corpus pages, and it is what buys the
uniformity. Where the half is a group — three of the four pages — there is no wrapper and no extra
buffer at all.

**Both halves are emitted or neither is.** `Plus` alone drives a premultiplied channel past its
alpha and one mark cannot tell a library that the other is coming; quorra states that as the
caller's obligation and it is the one thing in that vocabulary a builder cannot check. The two
halves carry the same clip and the same geometry — `pdf_model::content::stated_shape` derives one
from the other — so every route that draws nothing removes both.

## What it draws, measured

`cargo test --profile gates -p render-quorra --test corpus -- --ignored`, 957 pages, this machine,
RADV:

| | agree | differ | refused |
|---|---:|---:|---:|
| before, at `a35dc70` | 915 | 37 | 5 |
| after, at `2c9bdd0` | **919** | 37 | **1** |

`knockout_smask.pdf`, `knockout_nested.pdf`, `knockout_nested_group_alpha.pdf` and
`knockout_inner_backdrop.pdf` all **agree with the CPU oracle** — two independent transcriptions of
`P' = (1 − f) × P + S`, one per backend, meeting on artwork that no fixture invented. Not one page
went the other way, and the differing list is unchanged page for page. The one refusal left is
`bug1721218_reduced.pdf`, which is a texture capacity rather than a clause.

`test-scenes::knockout_stated_shape` is drawn rather than refused, and
`cpu_and_quorra_agree_on_a_knockout_element_that_states_its_shape` checks a pixel against the
clause instead of against a tolerance: inside the shaped element and inside the red rectangle it
knocks out, the red is gone whole and what stands there is `½ × blue + ½ × green`, 127 or 128 per
channel. The half level is the group's alpha quantised to `128/255` on its way through the layer —
quorra's stated unorm rounding — and the assertion is written as a bound of one level for that
reason: pinning the byte would pin the rounding rather than the clause. Drawing the element with
`Compose::Src` instead leaves half the red standing, 64 of 255 on a channel the assertion reads.

## What the refusal that replaced it holds

`quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` was written in session 439 "so
that it fails the day you lift the restriction", and it did: it stopped compiling at the bump. Its
replacement, `quorra_states_what_it_will_not_stage`, holds the two constraints that *survived*,
because both are load-bearing here rather than incidental:

- **a staged half may carry no blend mode.** §11.3.5 composites such a mark through an implicit
  one-element group, which is the step the pair replaces. `pdf-model` already drops the blend from
  a shape half — §11.4.6 leaves a knockout element nothing to blend against — but an *object* half
  that blends is a display list this tree can build, and it is now refused by name rather than
  drawn.
- **a staged group must be isolated.** §11.4.4 seeds a non-isolated group's buffer with its own
  backdrop, so the alpha the erase half reads as a shape would carry that backdrop's too. Every
  wrapper this backend emits is isolated for that reason.

A test that fails when a dependency changes is worth more than a paragraph that says the same
thing, and the shape generalises: **the replacement for such a test is the constraint one step
along, not a deletion.**

## What this round did not take

Upstream had moved one commit further by the time the lock was written — `7a58ced`, a shelf layout
that makes the scratch sheet nearer square, aimed at the page §6 of the upgrade note names as half
empty. The pin is `2c9bdd0`, which is the revision the note describes and the revision every number
above was measured at. Taking an undescribed commit in the same round would have made the
measurement about two changes, and one of them documented nowhere on this side.

## Consequences

- `doc/todo/23` has no backend row left: what remains there is the interpreter's.
- §11.4.6's ledger row keeps `partial` for the three things it named — an element whose one alpha
  genuinely carries both quantities, a non-isolated knockout group whose elements blend, and
  §11.6.4.3's `/AIS` — and loses the backend sentence entirely.
- **All three backends draw this element now**, which they have not all done since it was
  introduced. `render-gpu` has since session 397: a Vello layer takes the operator and a group half
  is encoded inside it, which is the same shape as the wrapper decided here and arrived at for the
  same reason. `render-gpu`'s own documented residue is elsewhere and unchanged — where the shape
  *is* the coverage it still draws source-over after the Destination-Out, weighting the backdrop by
  `1 − f × opacity` a second time (`knock_out`'s comment, `doc/todo/23`).
