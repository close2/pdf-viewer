# 666 — The shape a group has when its opacity is one

`doc/todo/11` item 4's last open bullet, taken on `render-cpu`. Parallel round, worktree `r666`,
branch `round-666`. **Pixels move.** ADR 0492 has the argument, the clause reading and every number.

## What the clause requires, and what the price turned out to be

§11.4.4's Table 139 returns **two** results from a group — a computed shape `f` and a computed alpha
`α` — and §8.5.4 constrains the first of them by the clip in force at the group's blit, in a sentence
written for groups specifically. §10.7.4 makes that constraint an intersection of sets. This tree's
group blit took a product, so a group whose `/BBox` is exactly its content's rectangle painted its
own boundary at the **square** of that pixel's coverage.

Item 4 priced the answer at **a shape channel beside a group's raster**, which had stood since
ADR 0355. Re-derived: §11.3.7.1 makes alpha shape times opacity and §11.6.4.2 gives every elementary
object an opacity of 1.0, so the two results are one number exactly where nothing inside the group
states §11.6.4.3's mask or §11.6.4.4's constant **as opacity** — a question about `/AIS`, decidable
while the content stream runs. **For that case the price is one boolean and one linear pass**, no
channel at all. The shape channel is the price of the *other* case, and item 4 had quoted one price
for two.

The other half of the re-derivation is that the layers already held more than the item assumed:
`pdf-model` has built a command that draws a *group's* shape since ADR 0234, `Command::Shaped` carries
one, `render-cpu` draws one, and `scan::intersected` already composes `min(M·S, P)` for a fill.

## The eighth rung

662's `coincident_edge_probe` is the discriminator, and the requirement was that a fix move it and
leave the other seven alone:

```text
  restated as     no soft mask     soft mask       and in 662
  fill alone            0.5059        0.5059          0.5059  0.5059
  W n clip              0.5059        0.5059          0.5059  0.5059
  form /BBox            0.5059        0.5059          0.5059  0.5059
  group /BBox           0.5059        0.5059          0.5059  0.2549
```

`issue7891_bc1.pdf`'s two boundary rows went 0.2549 and 0.2079 to **0.5059 and 0.4549** — their own
coverage. `issue21346.pdf`'s device column 14 of row 89 went 0.469 to **0.694** of the mark, so
`0.827^4.0` became `0.827^1.9`.

## Changed

- `crates/pdf-render/src/display_list.rs` — `Command::Group::alpha_is_shape`, with the clause reading.
- `crates/pdf-model/src/content/transparency.rs` — `group_alpha_is_shape` and `element_alpha_is_shape`;
  every group emission site states the field, `pattern.rs`, `path.rs` and `text.rs` included.
- `crates/render-cpu/src/scan.rs` — `intersect_group`, and `Reach::whole`.
- `crates/render-cpu/src/lib.rs` — `group_blit_mask`, and the stale comment on the non-isolated
  interpolation that said a group has no shape for §10.7.4 to intersect with.
- `crates/render-cpu/tests/group_clip_intersection.rs` — new, five tests, one of them pinning that a
  group whose alpha is *not* its shape still gets the product.
- `crates/render-quorra/src/scene.rs`, `crates/render-gpu/src/scene.rs` — why the flag is unread there.
- `crates/test-scenes`, three test files — the field, stated truthfully rather than as `false`.
- `doc/conformance/ledger.toml` — §11.4.4, §11.3.6, §11.3.7.2, §10.7.4.
- `doc/QUORRA_FEEDBACK.md` section 36 — the ask, for a flag plus a `min` rather than a shape channel.
- `doc/todo/11` item 4 — the price corrected, the ladder, the witnesses, what is still owed.

## Owed

- A stroke's coverage and an image's edge, still without a witness on this construction.
- A **non-isolated** group's raster, whose buffer starts as a copy of its backdrop.
- A group whose opacity is not 1.0 everywhere — where the shape channel is genuinely the price, and
  where no corpus document has been shown to need it.
- Two backends that cannot take the intersection because the composite is inside their libraries.
