# ADR 0173 — A soft mask is not a figure whose colour the caller supplies

Status: accepted, two-hundred-and-thirty-seventh session.

## Context

`issue19634.pdf` is a Skia rendering-correctness test called *blurSmallRadii*: a 100 × 100 page
that draws the word **guest** five times, each as a red blurred copy under a green sharp one,
with the blur radius growing down the page. It reached §3a's ranking at 0.85 from the nearest
reference and **5.96 from the furthest**, which is the shape that says the references are
disagreeing — and they were, five ways:

```text
ours 2.87    mupdf 7.63    hayro 8.11    poppler 16.64    ghostscript 47.98
```

The picture said what the numbers could not: **ours drew no red at all.** Five green words, and
nothing under them.

The red text is a Type 3 font whose glyph procedure is three lines —

```text
1536 0 0 -1632 1536 192 d1
32 0 0 32 0 -1632 cm
/G15 gs
0 0 48 57 re
f
```

— where `/G15` is an `/ExtGState` carrying `/SMask << /S /Luminosity /G … >>`, and the group it
names draws one `DCTDecode` greyscale image: the blurred glyph. So the blur is a *mask*, and the
glyph is a plain filled rectangle poured through it.

All 51 of the page's commands were being emitted, and nothing was reported. The marks were there
and invisible.

## What was wrong

§8.6.8 restricts a `d1` glyph description:

> Invoking operators that specify colours or other colour-related parameters in the graphics
> state is restricted in certain circumstances. This restriction occurs when defining graphical
> figures whose colours shall be specified separately each time they are used.

and lists the circumstances — "[i]n any glyph description that uses the d1 operator (see 9.6.4,
"Type 3 fonts") and to all other content streams invoked from within the same glyph
description" — and the consequences, of which one is:

> Unless painting an image mask, all image painting operators shall be ignored.

`Interpreter::uncoloured` implements that, and it was **still set while the soft mask's own
group was interpreted**. The group's one image was therefore skipped, the group drew nothing,
§11.5.2's NOTE 2 made the mask zero, and every glyph the font drew was masked away entirely.

## Decision

**Clear `uncoloured` for the duration of a soft mask's group, and restore it afterwards.**

The clause's own sentence says why. The restriction is for "graphical figures whose colours shall
be specified separately each time they are used" — a figure that will be painted in the caller's
colour. A soft mask is not painted at all: §11.6.5.2 composites the group, takes the luminosity
of the result and uses it as *alpha*. NOTE 1's reason for exempting a stencil transfers word for
word: an image mask is permitted "because it does not specify colours; instead, it designates
places where the current colour is painted", and that is precisely what a soft mask does, with a
value per pixel instead of a bit.

And the restriction is not merely inapplicable here, it is destructive in two directions. A
`/Luminosity` mask's values **are** its group's colours, so ignoring `rg` inside it changes the
mask rather than leaving it to the caller; and ignoring the group's images leaves a mask of zero,
which erases the very marks the glyph exists to make. A rule that exists so a figure can be
recoloured cannot be read as a rule that deletes it.

**What still propagates is unchanged.** A Form XObject invoked from the glyph *is* painted with
the glyph's colour, so `uncoloured` continues to reach it — that is the clause's "all other
content streams invoked from within the same glyph description", and it is why the flag is saved
and restored rather than cleared.

## Consequences

`issue19634.pdf` page 1: ink **2.87 → 8.03**, against `mupdf` 7.63 and `hayro` 8.11 — we now sit
between the two references that draw the same picture. The page left §3a's ranking.

No other corpus page moves: the corpus gate, the oracle's counts and the cross-backend gate are
unchanged, which is what a defect confined to `d1` + `/SMask` should look like.

`poppler` at 16.64 and `ghostscript` at 47.98 are still a long way off, and the second is worth
naming: `ghostscript` paints solid red blocks, which is §8.6.8's `gs` list read as though
`/SMask` were on it. It is not — the list is `TR`, `TR2`, `HT`, `BG`, `BG2`, `UCR`, `UCR2`,
`UseBlackPtComp`, and every one of them describes a marking device.

**The instrument is what found it, and the shape is worth recording.** Nothing reported anything;
the display list had every command; no gate could see it. What could was §3a's ranking plus the
rule that a wide nearest-to-furthest ratio means the *references* disagree — five renderers, five
answers, which is never scan conversion.
