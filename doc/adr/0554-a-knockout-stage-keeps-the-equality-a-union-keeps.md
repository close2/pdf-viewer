# ADR 0554 — A knockout stage keeps the equality a union keeps

Status: accepted, 2026-08-23. Session 699. Removes the `!knockout &&` guard in front of
`pdf-model`'s `group_alpha_is_shape`, adds `crates/pdf-model/examples/group_shape_census.rs`,
amends `pdf_render::Command::Group::alpha_is_shape`'s doc comment and §11.4.6's, §11.4.4's and
§8.5.4's ledger rows. **Pixels move**, on one corpus page.

## What was refused, and by what

ADR 0492 gave `Command::Group` an `alpha_is_shape` flag: the condition under which a group's
single premultiplied raster carries ISO 32000-2 §11.6.4.2's *shape* `f` and not only its alpha
`α`, so that §8.5.4's clip at the group's blit can be taken as §10.7.4's intersection of sets
rather than as a product. `pdf-model` proves it, because the display list holds a translucent
colour and not the reason it is translucent.

The proof had a guard in front of it that its own reasoning did not need:

```rust
alpha_is_shape: !knockout && group_alpha_is_shape(&commands, ais_inside.settled()),
```

and the doc comment beside `group_alpha_is_shape` gave the reason:

> **A knockout group.** Its accumulation is §11.4.6's `(1 − f) × P + f × E` rather than §11.4.4's
> union, and the equality holds there too for an opaque element — but its elements reach a backend
> as `Command::Shaped` pairs whose two halves are drawn by two separate composites, and proving
> the alpha those leave is the shape is a separate argument from this one.

The sentence answers itself. `element_alpha_is_shape` **already refuses a `Command::Shaped`
element**, wherever it appears and whatever kind of group holds it — its own next bullet says so.
So the guard refused the case it named twice, and every knockout group whose elements are ordinary
marks once.

## What the clause says

§11.3.7.1 defines `α = f × q`, and §11.6.4.2 supplies the base case:

> All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere.

What §11.4.6 adds is a different *recurrence*, not a different opacity. Its knockout stage
replaces the accumulation within an element's own shape where §11.3.7.3's union adds to it — and
the clause applies each recurrence to shape and to alpha alike, differing only in the opacity
inputs each carries. With every opacity input inside the group equal to 1.0, `α = f` at every step
of a knockout stage exactly as at every step of a union. The equality is a property of the
opacities, and knockout is not one of them.

quorra's ADR 0074 read the same clause independently and reached the same place: their
`encode::opacity::every_opacity_is_one` refuses a paint's alpha, a soft mask and a nested group
carrying either, and deliberately refuses **neither** blend mode nor knockout, "none of them is an
opacity input, and each applies its recurrence to shape and alpha alike".

## The witness, which is a corpus page and was found by their gate

`22060_A1_01_Plans.pdf`'s first page states **ten** transparency groups.
`crates/pdf-model/examples/group_shape_census` prints them:

```
depth 0  alpha 1.0000  clip yes  mask no  blend Normal  isolated true  knockout true
         alpha_is_shape false  2 element(s): 1 fill, 1 stroke
```

— ten of that line. Every one is isolated, opaque, unmasked, composited Normal, **clipped at its
blit**, and its two elements are an ordinary fill and an ordinary stroke, so not one of them is the
`Command::Shaped` pair the guard's reason named. This tree composited all ten by a product where
§8.5.4 and §10.7.4 ask for an intersection.

Nothing here found it. quorra's push of 2026-08-23 taught their compositor the same intersection,
their proof fired on those ten groups, and the cross-backend gate's *page line* for this document
moved while every total stayed still — the shape their own §1.2 predicted and asked us to resolve.

## What it moves

| | before | after |
|---|---|---|
| cross-backend gate, page one at scale 1 | 932 agree, 23 differ, 2 refused, 17 not comparable | identical |
| `22060_A1_01_Plans.pdf`'s line in it | mean 0.8164, ssim 0.98562 | **mean 0.7754, ssim 0.98630** |
| oracle, 1945 pages | 983 agrees, 65 contradicted, 832 ambiguous, 42 not comparable | identical |
| `22060_A1_01_Plans.pdf`'s oracle line | mean 6.09, ssim 0.7697 | **mean 6.07, ssim 0.7703** |

One page line in each gate, and both move toward the other reading — quorra's composite in the
first, and poppler, mupdf and ghostscript's consensus in the second. No other page in either
instrument changes by a printed digit.

## Isolation, which this flag does not ask and should not

The guard removed here was about knockout. There is no guard beside it about **isolation**, and
that is correct rather than an oversight found and left: a non-isolated group's buffer starts as a
copy of its backdrop (ADR 0237), so the alpha in it is the backdrop's unioned with the group's and
is not the group's shape whatever any proof about the commands says. That is a fact about the
*buffer*, so the backend that holds the buffer is what tests it — `render-cpu`'s `group_blit_mask`
opens with `if !group.isolated || !group.alpha_is_shape`, and `element_alpha_is_shape` asks it of a
*nested* group because there the buffer is an element the walk can see. Both doc comments now say
so, so that the next reader does not add the guard this one removed.

## What was considered and not done

**Proving a `Command::Shaped` element's shape.** It is the one remaining decline and it is a real
argument, not a formality: the pair's two halves reach a backend as two composites and what the
second leaves in the alpha channel is a question about the backend rather than about the display
list. No corpus page ranks it — the ten groups above hold none — so it stays owed rather than
guessed.
