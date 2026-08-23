# 699 — The offset was ours, and the guard refused its own reason

2026-08-23/24. quorra taken to `3b105847`, their three questions answered, and two of the answers
turned out to be about this tree rather than about theirs.

ADRs: [0554](../adr/0554-a-knockout-stage-keeps-the-equality-a-union-keeps.md),
[0555](../adr/0555-the-offset-was-ours-and-the-lane-that-looked-exact-was-not.md),
[0556](../adr/0556-two-answers-a-dependency-asked-for-and-one-it-did-not.md).

Touched: `Cargo.lock`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/path.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/pdf-model/tests/text_render_modes.rs`,
`crates/pdf-render/src/display_list.rs`, `crates/pdf-model/examples/group_shape_census.rs` (new),
`crates/render-quorra/examples/sampled_lane_column.rs` (new), `doc/conformance/ledger.toml`
(§8.5.4, §10.7.4, §11.4.4, §11.4.6), `doc/QUORRA_FEEDBACK.md` (§38 new, §31 retracted, §33 and §36
marked answered), `doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` (their reply, brought into the tree).

## The push

`97ad95ac` → `3b105847`. The API claim was verified rather than taken: `quorra-scene` has no diff
at all, every `pub` item declaration in both crates' `src/` is byte-identical between the
revisions, and the one addition is the enum variant they named. The cross-backend gate's totals are
unmoved at 932/23/2/17 and **one page line of 957** moves — `22060_A1_01_Plans.pdf`, by +0.0411 of
mean and −0.00068 of ssim, which is their own §1.2's movement to the fourth decimal. The oracle
never invokes their device and did not move. `issue19083.pdf` stays on the differing list, as
expected: it is a mark's clip rather than a group's.

## The two findings

**Their question 2 was a falsifiable prediction and it fell the other way.** They predicted the
device transform this tree hands `render-quorra` differs from the one it hands `render-cpu`, and
said that if the two were equal to the bit their conclusion was wrong. They are equal to the bit,
on every one of `bug1743245.pdf`'s 536 rules. What moves those marks is `pdf_render::sub_pixel_bands`
— **this tree's own §10.7.4 substitution** — and switching it off swaps the two columns of §31's
table: our oracle then agrees with quorra's default lane and disagrees with the sampled one. Their
fitted scale of 0.998899 is `16.48164 / 16.5`, the ratio of that page's stated device pitch to a
whole pixel, which is the snap arithmetic and not a fit. `doc/QUORRA_FEEDBACK.md` §31 is retracted
in place; ADR 0555 carries the trap.

**Their question 1 was one line of data and the line pointed here too.** All ten groups on
`22060_A1_01_Plans.pdf`'s first page are isolated, opaque, unmasked, clipped **knockout** groups of
one fill and one stroke, and `alpha_is_shape` was `false` on every one — because `pdf-model` had a
`!knockout &&` guard in front of its proof whose stated reason (`Command::Shaped` elements) is
enforced by the per-element test one level down. Removing it converges the page onto quorra's
composite and moves its oracle line toward the three references, with every count in both gates
identical. ADR 0554.

## The third question, and the enum

`render-quorra/examples/sampled_lane_column` is the corpus column they asked for before changing
the sampled lane's routing: at the magnification `viewer-ui` actually takes that lane, diverting
every mark whose width is not a multiple of the pitch would move **88.31 %** of the lane's marks to
the processor, and would not remove the §10.7.4 non-conformance, which is a property of the lattice
rather than of the width. Declined, with the numbers.

`#[non_exhaustive]`: their claim that we hold no exhaustive match over `RenderError` or `SceneError`
is verified — the break costs this tree nothing — so the recommendation is to take it now, for
`DeviceError` as well. **And one enum they did not name should stay open**: `viewer-ui`'s
`swapchain()` holds a genuinely exhaustive match over `SurfaceProblem`, whose five variants mirror
`wgpu`'s closed set, and there the compiler noticing a new arm is the feature.

## What is owed

Nothing to quorra. Here: a `Command::Shaped` element's shape is still the one thing
`group_alpha_is_shape` declines, and 51 clipped groups on eight corpus documents sit in the hole
between our `/AIS` reading and a proof from a command list alone — measured, not urgent, and
`doc/QUORRA_FEEDBACK.md` §38.2a is the size of what closing it would buy.
