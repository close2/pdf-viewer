# A transfer function that changes what a screen shows

Status: **found in the three-hundred-and-fifty-seventh session and implemented in the
three-hundred-and-fifty-eighth**, after the project owner amended `CLAUDE.md`'s scope line on the
evidence below. §10.5's ledger row was `silent` for one round — the first since the thirty-fifth
session — and is `implemented` now. Kept because the *argument* is what took the round, and because
`doc/todo/01`'s sweeps look here.
Priority: 13 — a defect: a wrong picture with nothing said about it
Corpus: **1 document draws wrong**; how many state a `/TR` at all is unmeasured (see below)
Clauses: §10.5, §8.4.5 (Table 57's `/TR` and `/TR2`), §11.7.5.2
Code: `crates/pdf-model/src/content.rs` (where `/ExtGState` is read),
`crates/pdf-model/src/function.rs` (the functions already parse and evaluate)

## The witness

`issue6931_reduced.pdf` page 1 — a pdf.js fixture whose page says, in words, **The color should be
red**.

Its content stream sets an `/ExtGState` before drawing the image:

```text
/R14 gs        %  << /Type /ExtGState /TR [11 0 R 12 0 R 13 0 R 13 0 R] >>
q 6865.2 0 0 2325.6 4514.4 3087.6 cm
/R15 Do        %  1608 × 546, /ICCBased 3-component, FlateDecode, /Predictor 15
/R16 gs        %  << /Type /ExtGState /TR /Identity >>
```

Three facts, each measured:

1. **The image's samples really are near black.** Our decode gives `[2, 2, 2]` at the first pixel
   and at the middle one, and `pdfimages -png` — *poppler's own* extractor — writes a 1608 × 546
   PNG whose mean is 4.92 and whose pixels are `srgb(2,2,2)`. So the raster is not misdecoded by
   anybody; it is a dark image.
2. **The transfer functions turn that into white.** Objects 11, 12 and 13 are §7.10.2 type-0
   sampled functions, 256 samples, domain and range `[0 1]`, and this tree's own
   `Function::parse` evaluates them:

   ```text
   f(0.000) = 0.000    f(0.008) = 0.992    f(0.500) = 0.000    f(1.000) = 0.000
   ```

   `0.008` is 2/255. It is a *permutation*, not a gamma — which is exactly what a file written to
   test §10.5 would carry, because a renderer that ignores the function cannot fake the answer.
3. **The five renderers split three to two, and the three are the ones that apply it.**

   ```text
   ours 20.3861 │ mupdf 20.6726 │ poppler 3.12869 │ ghostscript 3.70614 │ hayro 3.67272
   ```

   At the pixel inside the image: ours `srgb(2,2,2)`, `mupdf` `srgb(2,2,2)`, `poppler`
   `srgb(255,255,255)`, `ghostscript` and `hayro` `srgb(253,253,253)`. The four-panel strip shows
   it without a number — a black square in two panels and a red heart on white in three.

**And nothing says a word.** The corpus gate reports nothing for this document, the oracle calls
the page `ambiguous`, and §3a's ranking put it nowhere near the top. It came off `doc/todo/00`'s
**step 7** — our ink minus the lightest live reference's, at **+17.26** — which is the instrument
built for exactly this: a page nobody is far from, drawing something nobody else draws.

## What the clause says, and what "marking device" turned out to mean

**The standard does not use the phrase "marking device" once.** `grep` over the whole of ISO
32000-2 finds zero occurrences; the term is this project's own. What the standard says is **raster
output device**, and §8.3.2.2 defines it in the same breath:

> The contents of a page ultimately appear on a raster output device **such as a display or a
> printer**.

§8.1 says the same from the other end — "[t]he facilities described in this clause are intended for
both printer and display applications" — and §10.1 closes the door on the escape route:

> For the purpose of clause 10, it is irrelevant whether a raster output device physically exists
> and is actually used for rendering, or is just assumed.

**And §10.1 separates §10.5 from §10.6 by exactly the criterion this project conflated.** Its list
of rendering steps reads:

> - For any object for which transfer functions are in effect, apply those transfer functions …
> - **If the raster output device supports PDF-defined halftoning**, apply halftoning according to
>   10.6, "Halftones".

One is conditional on the device and the other is not. §10.6.1 then says it outright for the case
of a screen:

> Some output devices can reproduce continuous-tone colours directly. Halftoning is not required
> for such devices; **after gamma correction by the transfer functions**, the colour components
> shall be transmitted directly to the device.

So on a device that needs no halftone the transfer functions still run, and the clause says so in
the sentence that excuses the halftone. **§10.6 is genuinely inapplicable to a screen and §10.5 is
not**, and the standard draws that line itself.

## What §10.5 says



§10.5 is `shall` throughout and never says *printer*:

> In the sequence of steps for processing colours, the PDF processor shall apply the transfer
> function after performing any needed conversions between colour spaces.

> Transfer functions shall always operate in the native colour space of the output device …
> The output shall be the transformed component value to be transmitted to the device (after
> halftoning, if necessary).

The only sentence that lets anybody off is addressed to a **producer**:

> Because transfer functions produce device-dependent effects, a page description that is intended
> to be device-independent should not define a current transfer function in the graphics state, or
> define `TransferFunction` in any halftone dictionaries.

`should`, and to the writer. Table 57's `/TR` and `/TR2` are deprecated in PDF 2.0, which binds a
producer too and says nothing about a reader's obligation to a file that already exists.

**So the ledger's old reason was wrong in every clause of itself.** It read: a transfer function
"compensates for the device's actual behaviour", operates "in the native colour space of the
output device", is deprecated, and "[a] display's calibration is not a property this document may
state". This document states one; it is not a calibration; and it decides what a reader sees.

## What was done

The project owner's answer was to split the scope line rather than to drop it: §10.6's halftones
stay inapplicable **on the standard's own condition**, and §10.5's transfer functions are in scope.
`CLAUDE.md` says so now, with §10.1's two bullets and §10.6.1's sentence as the reason.

**The census first**, which this file said a round taking the clause owes:
`examples/transfer_function_census` walks every page's `/ExtGState` and every form `XObject`'s over
all 974 documents. **13 state a `/TR` or `/TR2`; exactly one states anything but `/Identity` or
`/Default`**, and it is this file's witness. `/TR2 /Default` appears 165 times and `/TR /Identity`
13. So the clause is one page rather than a population, and implementing it could not move anything
else — which the corpus and oracle gates then confirmed to the digit.

`content.rs` gained `Transfer`, three functions wide, read from Table 57 with `/TR2` in preference
to `/TR`, and applied in `fill_paint`, `stroke_paint` and to an image's samples. Three answers
rather than two — `Stated::{Unsaid, None, Set}` — because "says nothing" and "says `/Identity`" are
different instructions and this very file uses both, one state after the other.

`issue6931_reduced.pdf` page one: **20.3861 → 3.61878**, against `poppler` 3.12869, `ghostscript`
3.70614 and `hayro` 3.67272. `mupdf` stays at 20.6726.

## Why it was not simply a defect to fix

`CLAUDE.md`'s scope section says, in the owner's own words:

> **Clause 10 where it applies to a screen.** Halftones and transfer functions describe a marking
> device; those are *inapplicable*, which is not the same as excluded, and the ledger keeps them
> apart.

Implementing §10.5 crosses that sentence, so it is **a question for the project owner rather than
a change to make**. The handover's own warning is the reason this file exists rather than a commit:

> "The specification defines nothing here" is itself a claim about the specification, and it
> decays.

This is the same shape as `DeviceCMYK` → RGB, which sat as a recorded silence for thirty-two
sessions before §10.4.2.5 turned out to answer it outright.

## What it would cost, if the answer is yes

- **Reading it**: Table 57's `/TR` is a function, an array of four functions, `/Identity`, or
  `/Default`; `/TR2` is the same with `/Default` meaning the device's own. `function.rs` already
  parses and evaluates every type, so this is one entry in the `/ExtGState` walk and one field on
  the graphics state.
- **Applying it**: §10.5 puts it *after* colour conversion and *in the device's native space*,
  which for this tree is RGB — so it is a per-component map on the way to a `pdf_render::Color`,
  in `pdf-render` so that neither backend decides it alone (trap 2). Both the input and the output
  are additive, which is stated, so an RGB device needs the first three of the four.
- **Images**: the same map per sample, which is where the cost is — `Conversion`'s per-image memo
  is the place it belongs, since a transfer is a pure function of a colour.
- **What it does not need**: a halftone, a marking device, or §11.7.5.2's per-region tracking,
  which stays inapplicable until a *second* transfer function competes with a first inside a
  transparency group.
- **Unmeasured, and a round taking this owes it first**: how many of the 974 documents state a
  Table 57 `/TR` or `/TR2` at all, and how many of those state anything but `/Identity`. That
  number decides whether this is one page or a population, and `examples/field_flag_census` is the
  shape of the census that would answer it.
