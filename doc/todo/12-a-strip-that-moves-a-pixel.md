# A page drawn in one strip is not the page drawn in two

Status: **open**, found in the three-hundred-and-eighty-first session.
Priority: 12
Clauses: — this is a property of `render-cpu`, not of the standard
Code: `crates/render-cpu/src/lib.rs` (`encode_in_strips`), `crates/render-cpu/tests/strip_parallelism.rs`

## The claim, and the counter-example

ADR 0139's property, which `strip_parallelism.rs` exists to guard and which the crate map states
as a fact:

> **the picture does not depend on how it was divided**

It is false by one pixel on a document committed in this repository.

`doc/PDF20_AN001-BPC.pdf` page 1, viewport 500×700 at scale 1 — the page as
`viewer_core::Viewer` fits it, 500×708 — drawn with `CpuRasterizer::new().with_strips(1)` against
`with_strips(n)` for **n = 2, 3, 4, 5, 8, 12, 16, 24, 32**:

| | |
|---|---|
| differing pixels | **1**, at (117, 636) |
| its value in one strip | 127, 127, 127 |
| its value in every division above one | 111, 111, 111 |
| alpha | unmoved |

**Every n from 2 to 32 gives the identical answer**, so this is *one strip against more than one*
and not a question of where a cut landed. The pixel is nowhere near any cut for most of those
divisions.

## Why the existing guard does not see it

`strip_parallelism.rs` compares the same divisions over six `test-scenes` fixtures and passes.
ADR 0138's defect — the one the guard was written for — was a **curve chopped at a strip's edge
and re-parameterised**, and ADR 0139's answer was to cut only at rows no curve crosses
(`pdf_render::unsplittable_rows`). That answer is intact; this is a different mechanism.

`encode_in_strips` composes each strip's transform as

```rust
transform: target.transform.then(Transform::translate(0.0, -(top as f32)))
```

so **every strip below the first is drawn under a multiplied matrix**, whatever row it starts at.
A matrix that has been composed is not bit-for-bit the matrix that has not, and a mark whose
coverage lands near a rounding boundary picks up the difference. One strip takes the early path
and is drawn under `target.transform` itself, which is why one strip is the odd one out and every
division above it agrees.

Trap 12b, exactly: a test suite made of small scenes tests small scenes.

## Why it was found now

The three-hundred-and-eighty-first session put the interpreter and rasteriser in a confined
process (ADR 0218), which for `glibc` reasons rasterises on **one** thread and therefore in one
strip. Its byte-for-byte comparison against the same page drawn in this process failed by three
bytes, and the three bytes were this pixel's channels. Nothing else in the tree draws a real page
in one strip on purpose.

## What it costs today

Little, and the size is worth being precise about rather than reassuring about:

- **No gate moves.** The oracle, the corpus, the quorra comparison and `doc/todo/00`'s step 7 all
  render with the planner's own strip count, which is greater than one on every machine this
  project has run on. All were re-run in the three-hundred-and-eighty-first and are identical.
- **A single-core machine would draw a different pixel** from this one, on this page, and no gate
  in this tree would notice — because every one of them runs on one machine.
- `viewer-confined` draws in one strip, so the confined path and the window differ by this pixel
  on this document. `tests/confined.rs` pins its comparison to one strip and says why.

## What to do

1. **Reproduce it as a test.** The cheapest form: `render-cpu`'s existing gate, given a display
   list built from a real page rather than from `test-scenes` — the fixtures are what missed it,
   and the property is worth asserting against the thing it is claimed for.
2. **Find whether it is the composition or the translation.** `Transform::then` with a translation
   of zero is already applied to the first strip, which is a matrix multiply the one-strip path
   does not do; if the first strip alone already differs from a whole-page render, the answer is
   "a composed matrix is not the same matrix" and the fix is to skip the composition where `top`
   is zero — which would leave the *other* strips still departing, so it is a diagnosis rather
   than a fix.
3. **Decide what the property is.** Either `encode_in_strips` is made to draw each strip under the
   same arithmetic as a whole page — which may not be possible while a strip has its own origin —
   or ADR 0139's sentence is narrowed to what it actually delivers, with this counter-example in
   it. A claim this tree makes about itself that is false is worse than a narrower true one.
