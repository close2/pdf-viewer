# A transfer function that changes what a screen shows

Status: **found in the three-hundred-and-fifty-seventh session and implemented in the
three-hundred-and-fifty-eighth**, after the project owner amended `CLAUDE.md`'s scope line on the
evidence below. §10.5's ledger row was `silent` for one round — the first since the thirty-fifth
session — and is `implemented` now. Kept because the *argument* is what took the round, and because
`doc/todo/01`'s sweeps look here.
**And re-opened in the six-hundred-and-thirty-second**, which found §11.7.5.2 `inapplicable` on an
argument about the clause that the clause does not make. **The six-hundred-and-thirty-seventh
closed the `silent` half**: the report is built and that row is `reported`, and the reading it took
found a *second* gap one clause up — a shading's colours never passed through §10.5's map at all.
**The six-hundred-and-fiftieth closed that one** (ADR 0479): the function is applied where a
shading's colours are *made*, so it reaches an axial or radial ramp's samples, a parametric mesh's
ramp, a mesh's corners and a function-based shading's grid. Two things are still owed and both are
below — the per-region model, and the pattern whose colours were resolved a graphics state before
the mark.
**The six-hundred-and-fifty-fifth read the three clauses that were said to meet at the `scn`** and
found they name three different moments (ADR 0483): two of them are implemented and the third is
what the last section then owed. Its price was smaller than this file used to say and its shape was
different, which is the whole of that session's contribution here.
**The six-hundred-and-sixtieth built it** (ADR 0487): a shading pattern's colours are rebuilt at the
mark and §11.6.7's three parameters are kept off the live state by a signature that cannot reach it.
**So one thing is owed here now** — §11.7.5.2's per-region model, below — and the file stays for it.
Priority: 13 — a defect: a wrong picture with nothing said about it
Corpus: `cargo run --release -p pdf-model --example transfer_function_census --
doc/pdf.js/test/pdfs/*.pdf` counts how many state a `/TR` or `/TR2`, how many state a real one, and
how many paint a shading on a page that states one. It takes the SafeDocs crawl too — `find
corpus-cache/safedocs -name '*.pdf' -print0 | xargs -0 -n 2000 <the binary>` — in under a minute
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
> and is actually used for rendering, or is just assumed …

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
- **What it does not need**: a halftone or a marking device. **This bullet also said it did not
  need "§11.7.5.2's per-region tracking, which stays inapplicable until a *second* transfer
  function competes with a first inside a transparency group", and that is wrong about the
  clause** — see the section below, which is the debt this file now carries.
- **What it turned out to miss, found in the six-hundred-and-thirty-seventh session and
  implemented in the six-hundred-and-fiftieth**: a **shading**. §10.5's subject is the component
  value — "[t]he output shall be the transformed component value to be transmitted to the device"
  — and a shading's are values. **The price 637 wrote here was wrong, and the reason is worth
  keeping.** It said `Shading::with_alpha`'s walk done again with a closure, in `pdf-render`. That
  walk maps a *finished* ramp, and a finished ramp has been through ADR 0068's simplifier: a
  `/FunctionType 2` interpolation with `/N 1`, which is most of the shadings in the world, reaches
  the display list as **two stops**. Mapping two stops and letting a rasteriser interpolate draws
  the chord between the transferred ends where the clause asks for the transfer's own curve — for
  a transfer that squares its input, 0.5 instead of 0.25 at the midpoint, which is 64 levels of
  255. So the function is applied *inside the sampling* instead, in `shading::kind_of` and
  `mesh::read`, where the ramp becomes a sampling of the composition and the simplifier then
  measures what will be drawn. It is a `pdf-model` change and `pdf-render` needed no line of it.
  Two things fall out and are in the code with their reasons: a shading built under a transfer is
  not cached, and a type 1 shading's device program is withdrawn.
- **Measured in the three-hundred-and-fifty-eighth session, by the census this bullet asked for**:
  `examples/transfer_function_census` over the corpus. Run it rather than reading a number here.

## What is still owed: §11.7.5.2, the parameter that belongs to a *region*

**Found in the six-hundred-and-thirty-second session, reading the ledger's unread rows.** §11.7.5.2
was `inapplicable` on the argument quoted above and its own row said the same. The clause does not
say it. Its rule is not about two functions competing; it is about *opacity*:

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

and then, at the end of the same paragraph's list of conditions:

> For portions of the page whose topmost object is not fully opaque or that are never painted at
> all, the default halftone and transfer function for the page shall be used

So one stated function is enough to make the clause bite. Where the topmost object at a point is
painted under a constant alpha below 1.0, a blend mode other than Normal, a soft mask, or is an
image XObject with an `/SMask` — and the clause extends each condition to the groups the object is
inside and to a tiling pattern's cell — the transfer function at that point is the page's
**default**, and this tree applies the object's own. It applies it per object, before compositing;
the clause applies the topmost object's, after. The two agree exactly on the fully opaque case,
which is where every corpus witness is, and nowhere else.

**The population, measured rather than assumed**, and it is why this is a `doc/todo` entry rather
than a round's work: `cargo run --release -p pdf-model --example transfer_function_census --
doc/pdf.js/test/pdfs/*.pdf` finds one document stating a transfer function that is not `/Identity`
or `/Default`, and `mutool draw -F trace` on that document's page shows its one image drawn at
`alpha="1"`, Normal, no soft mask and no `/SMask`. **Zero corpus pages are drawn wrong by this
today.** A round that changes it therefore has no oracle witness and owes a fixture (trap 8).

Two pieces of work, in this order. **The first is done as of the six-hundred-and-thirty-seventh
session; the second is what this file still owes.**

1. **The report — built.** `Interpreter::note_transfer` raises `Unsupported::TransferFunction`, and
   the condition is derived from the clause rather than approximated from the code. At a point the
   clause's answer is the topmost object's function where that object is fully opaque and the
   page's default otherwise; this tree's is each contributor's own applied before compositing. So
   the two differ **exactly where some object covering the point carried a function and the topmost
   object covering it is not fully opaque** — one statement covering both the object that carries
   the transfer and the translucent object painted over it, which two separate conditions would
   have got wrong in the second case. Nothing here knows which objects overlap, so what fires is
   the geometric over-approximation: a mark §11.7.5.2 does not call fully opaque, made while some
   mark on the page has carried a function. It cannot under-report, since a point drawn wrong has
   both halves on the page in that order.

   The *ancestry* was the trap and it is carried rather than read: §11.6.6 resets the blend mode,
   both alpha constants and the soft mask before a group's content runs, and §11.6.7 starts a
   tiling cell from the initial state, so a flag reading the mark's own alpha would have reported
   **nothing at all** for the nested case. `Interpreter::opaque_ancestry` is narrowed in
   `group_commands` and in `tile`, and scoped away inside a soft mask's group, whose marks are
   never painted at a point on the page. `tests/transfer_functions.rs` has the three fixtures,
   each with its mutation.
2. **The per-region model, which is a rasteriser change and is not small.** The clause's quantity
   is a property of a *point*, decided by the topmost object covering it, and applied to the
   composited colour rather than to each contributor's. Nothing in the display list carries a
   per-point parameter today, and inventing one for a population of zero would be speculative
   work of exactly the kind `CLAUDE.md` forbids. What it would take, so that the next round does
   not have to re-derive it: a per-pixel *transfer identity* rasterised beside the colour — each
   fully opaque mark writing its own function's index, each non-opaque one writing the page's
   default — and one pass over the finished raster mapping each pixel through the function its
   index names. That is a second channel in `pdf-render`'s target and a matching pass in all three
   backends, which is why it is priced as a rasteriser change and not as a report. What makes it
   worth writing down is that the *reason* to defer is now a measurement rather than a claim about
   the clause, so a document that turns up stating a transfer function under a soft mask moves it
   straight to the top — and since the six-hundred-and-thirty-seventh such a document says so out
   loud instead of being drawn wrong in silence.

## Closed in the six-hundred-and-sixtieth: the pattern whose colours were resolved before the mark

**Kept as a heading rather than deleted with the section**, because two other rows and an ADR point
here and because the *shape* of the answer is what the next reader wants. The argument in full is
ADR 0487; what it settles is below in four lines.

§8.7.2 makes a pattern a colour and `scn` is where a colour is set, so a shading pattern's colours
were built at the selection and the mark could be several graphics states later. §11.6.7 says which
half of that is right. The definition — §8.6.5.9's black point, §8.6.5.8's intent, §10.7.3's
smoothness, §8.7.2's matrix — belongs to the beginning of the content stream and to nothing later:

> The definition shall not inherit the current values of the graphics state parameters at the time
> it is evaluated; those parameters shall take effect only when the resulting pattern is later used
> to paint an object.

The *painting* is the other half, and the paragraph after the bullets says so outright:

> This painting operation is subject to the values of the graphics state parameters in effect at
> the time, just as in painting an object with a constant colour.

So `PatternPaint::Shading` carries a `ShadingDefinition` — the `/Shading` object unresolved, its
resources, the matrix and §11.6.7's `PatternInitial` — and `Interpreter::shading_paint` rebuilds
through `shading::Cache` where the mark's `MarkColouring` differs from the one the `scn` built
under. **The warning this file carried is answered by a type**: `mark_colouring` and `build_shading`
take a `&ShadingDefinition` and no `&GraphicsState`, so §11.6.7's parameters have no route in from
the mark — the trade of one departure for another is not something a later round can write by
accident. §11.7.2's compositing target falls out for nothing, so `group_press`'s fifth condition
came off with it, and `Painted::Shading`'s `stale` flag went because there is no stale state left.

