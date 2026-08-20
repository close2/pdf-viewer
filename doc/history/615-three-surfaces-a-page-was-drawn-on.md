# 615 — Three surfaces a page was drawn on, and none of them its own

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the third round running. Seven whole
archives this time, and three defects — one at the negative end of the ranking and two at the
positive one, all three silent or reported-into-a-black-page.

Date: 2026-08-20.
ADR: [0451](../adr/0451-three-surfaces-a-page-was-drawn-on-and-none-of-them-its-own.md).

Touched: `crates/pdf-render/src/shading.rs`, `crates/pdf-model/src/content/pattern.rs`,
`crates/pdf-model/src/image.rs`, `crates/pdf-model/src/icc.rs`, `crates/pdf-model/src/shading.rs`,
`crates/pdf-model/tests/{shadings,image_masks,corpus}.rs`, `doc/conformance/ledger.toml`
(§8.7.4.2, §11.6.4.2, §8.9.6.3, §8.6.5.9), `doc/todo/03-more-corpora.md` §18,
`doc/todo/_image-codecs-and-the-sandbox.md` §7, the ADR and this file.

## The chunk

**`0423`, `1161`, `2268`, `3375`, `4482`, `5589`, `6696` — 7000 documents**, none of 603's two or
613's five. An archive is a hash bucket (ADR 0261), so any set is unbiased. 603's instrument
**reused rather than rewritten**: page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every
invocation explicit about the page box, ranked by our ink minus the lightest live reference's,
panel sizes beside each number.

**Checked before it was trusted.** This tree is at 613's commit, so 613's three named documents
must reproduce, and they do to the thousandth.

## What the three defects were

**`0423269.pdf` at −9.420** — a Japanese product sheet whose two coloured backgrounds were white
paper here, nothing reported. They are `ShadingType 4` meshes painted by `sh` inside a tiling
pattern's cell. §8.7.4.2 gives `sh` no path, so a display list stands one in, and this tree stood
in the **page rectangle** — which `Cell::repeat` displaces along with everything else in the cell,
while being no part of the figure. The site whose shading lands on the page is the site whose
rectangle has just left it. §11.6.4.2 says the surface is "the bounds of the shading's painti ng
geometry", which is a property of the shading, so it travels with it; `Shading::painting_bounds`
states it for the mesh types.

**Five of the six deepest positive rows** — +232.654, +215.370, +191.291, +175.316, +121.381 —
mixed-raster scans with a small colour layer under a full-page 600 dpi JBIG2 or CCITT `/Mask`,
each **reported** and each drawn as a solid black page, because refusing a `/Mask` draws the base
image unmasked. One number was deciding two questions: for a `/SMask`, exceeding it selects
§10.7.4's device-scale construction; for a `/Mask` it selects a refusal, always, because Table 87
gives a stencil no colour space. The preference and the ceiling are separate now, and the ceiling
is `MAX_SAMPLES` — a combined grid is a raster this crate allocates.

**`2268885.pdf` at +194.095** — a floor plan drawn as a photographic negative, one command and no
report. Black point compensation found the profile's black by pushing every component to 1.0,
which is full ink in `CMYK` and **white** in `RGB`; the guard let it through because the profile's
white corner misses D50 by a thousandth, and the stretch then divides by that thousandth. Both
ends of the device range are evaluated now and the darker taken.

## What moved

Fourteen archives re-ranked whole and diffed row by row: **15 rows of 14 000 move and every one of
them is a document one of the three fixes is about**, thirteen improving, the head going from
+232.654 to +0.035 and from −14.485 to −0.012. That the list is this short was the open question
for the black point change, which touches every ICC profile with a lookup table; the answer is
that the guard it corrects had been keeping almost every profile out of compensation. **Three of
the moved rows are in 613's own archives**, which is the second round running that a fix has
reached an earlier chunk. `doc/todo/00`'s step 7 reproduces session 598's head and tail to the
thousandth.

Each fix is pinned by a test that was **run against the defect first**: the surface by a tiling
fixture whose painting site is a copy rather than the interpreted cell, the ceiling by the
smallest stencil past the preference, the black point by the additive twin of the fixture that
already existed for the subtractive case.

## What the head still holds

Two silent rows below −8, both trap 9's family — one document of `DCTDecode` images under one
`ICCBased` space differing by a few levels over the whole page, one of `/DeviceCMYK` JPEGs. And
`doc/todo/_image-codecs` §7's `hayro-jbig2` release now has **three** documents waiting on it
rather than one: two of them arrived from behind the mask ceiling this round lifted, and both are
the two moved rows that do not close.

## Gates

The full §2 sequence, because the change is in `pdf-model` and `pdf-render`. §5's binaries were
**not** rebuilt: this is not a fifth round and nothing on the launch path was measured.
