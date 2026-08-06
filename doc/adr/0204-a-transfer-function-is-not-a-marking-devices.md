# ADR 0204 — A transfer function is not a marking device's, and the standard says which are

Status: accepted, 2026-08-06 (sessions 357 and 358).

## Context

`doc/todo/00`'s **step 7** — our ink minus the lightest live reference's, over every ambiguous page
— produced `issue6931_reduced.pdf` at **+17.26 of 255**. The page says, in words, *The color should
be red*, and this tree drew a black square.

Three measurements, each checkable:

1. The image's samples really are near black. Our decode gives `[2, 2, 2]`, and `pdfimages -png`
   — *poppler's own* extractor — writes a PNG whose pixels are `srgb(2,2,2)`.
2. The `/ExtGState` in force sets `/TR` to three §7.10.2 type-0 sampled functions, and this tree's
   own `Function::parse` evaluates them: **f(0.008) = 0.992**, f(0) = f(0.5) = f(1) = 0. A
   permutation, not a gamma — which is what a file written to test §10.5 carries, because a
   renderer that ignores it cannot fake the answer.
3. `poppler`, `ghostscript` and `hayro` apply it and draw a red heart on white; ours and `mupdf`
   do not.

And nothing said a word. The corpus gate reported nothing, the oracle called the page `ambiguous`,
and §3a's ranking had it at 0.35 from the nearest reference — which accuses nobody.

**The reason this tree did not apply it was a sentence in `CLAUDE.md`**, not a cost:

> Halftones and transfer functions describe a **marking device**; those are *inapplicable*.

## Decision

### The standard does not use that phrase, and it draws the line itself

**"Marking device" occurs zero times in ISO 32000-2.** The term is this project's own. What the
standard says is **raster output device**, and §8.3.2.2 defines it in the same breath: "such as a
display or a printer". §8.1 says the facilities are "intended for both printer and display
applications", and §10.1 closes the escape route — "it is irrelevant whether a raster output device
physically exists".

And §10.1's own list of rendering steps separates §10.5 from §10.6 by exactly the criterion this
project had conflated:

> - For any object for which transfer functions are in effect, apply those transfer functions …
> - **If** the raster output device supports PDF-defined halftoning, apply halftoning according to
>   10.6, "Halftones".

One is conditional on the device and the other is not. §10.6.1 then says it for the case of a
screen outright:

> Some output devices can reproduce continuous-tone colours directly. Halftoning is not required
> for such devices; **after gamma correction by the transfer functions**, the colour components
> shall be transmitted directly to the device.

**So §10.6 is genuinely inapplicable to a screen and §10.5 is not**, and the sentence that excuses
the halftone is the one that keeps the transfer. The project owner amended `CLAUDE.md`'s scope line
to say so, splitting the two rather than dropping the entry — and added the general rule the
episode taught: the restrictions in that file were drawn early, around the most important
functionality, and they come off step by step as each is read against the standard.

### The census before the code

`doc/todo/13` said the number a round taking this owes first is how many documents state a transfer
at all. `examples/transfer_function_census` walks every page's `/ExtGState` and every form
XObject's over all 974:

| | |
|---|---|
| state a Table 57 `/TR` or `/TR2` | **13** |
| state anything but `/Identity` or `/Default` | **1** |
| `/TR2 /Default` occurrences | 165 |
| `/TR /Identity` occurrences | 13 |

So the clause is one page rather than a population, and implementing it could not move anything
else — which the corpus and oracle gates then confirmed to the digit. **Measuring first is what
made this a small change rather than a brave one.**

### Three answers, not two

`Transfer::read` returns `Stated::{Unsaid, None, Set}`, and the middle one is the decision worth
recording. "The state says nothing" and "the state says `/Identity`" are different instructions: the
first leaves whatever transfer is in force and the second turns it **off**. This file's own witness
uses both — one graphics state sets three functions and the next sets `/Identity` — so a reader that
folded them together would carry the transfer past the object it was written for and paint the rest
of the page wrong.

Table 57's precedence is the table's own: `/TR2` "shall be used in preference to `TR`".

### Where it is applied

`fill_paint`, `stroke_paint`, and an image's samples. That is where a colour becomes the value a
device receives, and §10.5 puts the transfer "after performing any needed conversions between
colour spaces" — by which point everything here is RGB, which is this device's native space.

**In `pdf-model` rather than in `pdf-render`**, deliberately: a transfer belongs to the *graphics
state an object is drawn under*, not to the object, and the same XObject drawn twice under two
states is two pictures. Doing it here also means both backends agree by construction, which is
trap 2's rule.

An RGB device uses the first three of four, which the clause states outright, and alpha is
untouched: §10.5 speaks of "the value of a colour component" and §11's opacity is another clause's
quantity.

The image path memoises over the 8-bit triple — a transfer is a pure function of a colour and a
photograph repeats its colours — which is the argument `image::Conversion` already records one
clause along. An image with no transfer is moved rather than touched.

## Consequences

- **`issue6931_reduced.pdf` goes from 20.3861 to 3.61878**, against `poppler` 3.12869,
  `ghostscript` 3.70614 and `hayro` 3.67272. **`mupdf` is now the only renderer of the five that
  does not apply it.** The page stays `ambiguous` — the remaining 0.58 of 255 among the four is the
  ordinary difference between five CMYK-to-RGB conversions — but it is no longer a page we are
  wrong about.
- **`silent` is zero again after exactly one round at one.** That row was zero for three hundred
  and twenty-two sessions, and the honest thing was to let it be one for the round it took to get
  an answer rather than to hide the finding inside `inapplicable`.
- **§11.7.5.2 stays inapplicable and its row now says why it is not this one's.** Tracking the
  transfer per *region* of the page needs a second transfer function competing with a first inside
  a transparency group; one transfer in force needs no tracking at all.
- **§8.4.5's Table 57 list loses two entries.** `/TR` and `/TR2` were on its "describes a marking
  device" list; `/BG`, `/BG2`, `/UCR`, `/UCR2`, `/HT` and `/HTO` remain, and each is now a claim
  that has to survive the same reading.
- **The instrument that found it is the one nobody was reading.** Step 7's *positive* side —
  content nobody else draws — had produced one name in its life before this round and has now
  produced two. A page 0.35 from the nearest reference is a page no ranking will ever accuse.
