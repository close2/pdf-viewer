# ADR 0026 — Transparency groups, and the page group nobody had noticed

Status: accepted, 2026-07-29.

## Context

A transparency group (ISO 32000-2 §11.4) is the one construction in clause 11 that this tree
had never looked at. A form XObject carrying a `/Group` was run as an ordinary form: its
elements were painted straight onto the page, each carrying the constant alpha and blend mode
that were in force at the `Do`. The ledger recorded that three times over, from three
different clauses, because a reader of any of them should have found it — §11.4.6 (knockout),
§11.6.6 (`/Group` read nowhere) and §11.3.7.3 (a group's result shape) were three of the five
`silent` rows, the status this project hunts.

It is also the largest *rendering* gap the corpus sizes: 29 documents report a soft mask in an
`/ExtGState`, and a soft mask is derived from a transparency group (§11.5.2, §11.5.3), so
nothing on that list can be built before groups exist.

The demand-track evidence was five contradicted pages named in `oracle.rs`: `knockout_*.pdf`,
where `mutool` and `ghostscript` show no blend in an overlap and we and `poppler` show one.

## What a group is, and what this implements

§11.4.1:

> A transparency group is a sequence of consecutive objects in a transparency stack that shall
> be collected together and composited to produce a single colour, shape, and opacity at each
> point. The result shall then be treated as if it were a single object for subsequent
> compositing operations.

So the display list gains one command, `Command::Group`, holding its elements and the alpha
and blend mode the group as a whole is painted with. Both backends already have the
construction natively: `tiny-skia` composites the elements into a pixmap of its own and draws
that pixmap once, and a Vello layer *is* a transparency group. The interpreter builds it by
marking the command list, running the form, and taking back what was drawn — a group is
discovered from the outside, and pulling the elements out afterwards is cheaper and clearer
than threading a builder through the whole interpreter.

**§11.6.6's initialisation is the half that is easy to leave out.** Before a group's content
runs, the blend mode goes to Normal and both alpha constants to 1.0, because those parameters
belong to the group and applying them to each element as well applies them twice. That single
rule is the whole visible difference on an ordinary page: two overlapping opaque objects
inside a group under `/ca 0.5` are one translucent shape, and without it the band they share
is painted twice.

## Three departures, each reported where it can change a pixel

Compositing onto a transparent buffer is §11.4.5's *isolated*, non-knockout group. Three of
Table 145's answers ask for something else, and the argument for each is the clause's:

**A non-isolated group (`/I false`) is drawn as an isolated one where that is provably the
same computation.** §11.4.4 composites a non-isolated group's elements onto the group's
backdrop and then removes the backdrop's contribution again (NOTE 3). With every element
blending Normal the removal is exact and the backdrop cancels — and the standard says so
itself, of the same formulas applied to a pattern cell (§11.6.7 NOTE 1):

> in the common case in which the pattern consists entirely of objects painted with the Normal
> blend mode, this behaviour can be optimised by treating the pattern cell as if it were an
> isolated group. Since in this case the results depend only on the colour, shape, and opacity
> of the pattern cell and not on those of the backdrop, the pattern cell can be evaluated once
> and then replicated, just as in opaque painting.

What makes the two differ is a blend mode *inside* the group, which §11.4.4's NOTE 2 gives as
the whole reason the two kinds exist. So the report fires on that and not on the flag: 9 corpus
documents.

**A knockout group (`/K true`) is reported, not implemented.** §11.4.6 composites each element
with the group's initial backdrop rather than with the elements below it, so the topmost object
at a point wins. The report asks the two conditions under which that can differ from what we
do — an element that composites, overlapping one painted before it — which is 6 corpus
documents rather than the 8 that write `/K true`. The reading is written down in the ledger
because it is the load-bearing part of the implementation nobody has written: for an *isolated*
knockout group the initial backdrop is transparent, so §11.3.6 leaves each element its own
colour and its blend mode has no effect at all, and the clause's two-stage average by source
shape is then a Porter-Duff Source composite modulated by coverage. A *non-isolated* knockout
group needs the group's backdrop, which is the same thing §11.4.4 lacks.

**A group blending colour space (`/CS`) that is not the device's is reported.** Compositing
here happens on the three components of the raster; a group asking for `/DeviceCMYK` blends
four, and `Lab` blends three that are not a linear map of these. Honouring it means holding the
group's raster in its own components and converting once at the end, which is a second raster
format rather than a colour conversion. 4 corpus documents, all `/DeviceCMYK`. A `/CS` that is
`/DeviceRGB`, `CalRGB` or an ICC profile of three components is *not* reported: those are the
colorimetric difference this renderer already takes page-wide and records as a choice.

## Two defects the family review found, and neither is about groups

**§8.10.1 step c): a form XObject's `/BBox` clips its content, and only an annotation's
appearance was clipped.** The clause lists what `Do` performs, and step c) is "Clips according
to the form dictionary's BBox entry"; Table 93 says the same of the entry. This tree had it in
`draw_appearance`, where §12.5.5's placement algorithm made it unavoidable, and not in
`draw_xobject`. `issue11279.pdf` was contradicted by all three references for it. The cost is
better seen on `tracemonkey.pdf` page 6, which is not in the comparison because its fonts are
substituted: a form paints a white background beyond its own box and covered the figure above
it, so a page that four renderers draw with two figures had one.

**§11.4.7: the page group is isolated, and both backends were painting onto the medium.**

> Ordinarily, the page shall be imposed directly on an output medium, such as paper or a
> display screen. The page group shall be treated as an isolated group, whose results shall
> then be composited with a backdrop colour appropriate for the medium.

Isolated — so a page's own initial backdrop is *transparent*, and the medium's white is applied
to the finished page. Filling the raster with white and drawing over it is the natural
implementation, and it is a different picture for every blend mode: §11.3.6 makes an object
blending against a backdrop of zero alpha keep its own colour, and a white backdrop is not zero
alpha. `transparency_group.pdf` announces the difference on its face — an ellipse under
`/BM /Difference` that four reference renderers draw crimson over white, and that this tree
drew as its inverse. `impose_on_medium` in `pdf-render` is the composite, called by both
backends at their boundary so neither can make the choice differently.

This is the session's largest finding and it was **invisible until groups made it visible**:
nothing in the tree had a reason to render onto transparency before, so nothing could tell the
two backdrops apart.

## What it cost, and what it exposed

`callgrind_rasterise` over page 101 of ISO 32000-2 — a dense text page with no transparency at
all — goes from 14.07 to 14.31 billion instructions, **+1.7%**: one page-sized pass to impose
the medium, less the page-sized fill it replaces. That is the price of the clause on every page
whether or not it uses transparency, and there is no cheaper form of it: the composite is what
the clause asks for.

**And it exposed a defect in the GPU backend that had been there since Vello landed.**
`read_back` converted Vello's output from premultiplied to straight alpha. Vello hands back
straight alpha already. Fifteen sessions of tests could not see it because the page was rendered
onto an opaque background and every pixel came back with an alpha of 255, where the conversion
is the identity. The first render onto transparency showed it in one pixel: half-covered by a
50% grey, `tiny-skia` gives `[128, 0, 0, 128]` and the GPU gave `[255, 0, 0, 128]` — the colour
divided by its own coverage. `vello_hands_back_straight_alpha` pins it, deliberately with a
transparent medium, because an opaque one takes every alpha back to 255 before the raster leaves
the backend.

## Consequences

- 93 contradicted pages against 100, of which one is a fix (`issue11279.pdf`, the `/BBox`
  clip) and four left by being reported rather than drawn (`knockout_*.pdf`).
- 237 documents report something, against 231: six knockout groups and one non-isolated group
  that blends began saying so, and `issue15372.pdf` stopped reporting §9.3.8 because the
  constant alpha its glyphs carried is now applied to their group instead.
- Three `silent` ledger rows closed — §11.4.6 to `reported`, §11.6.6 and §11.3.7.3 to
  `partial` — leaving two: §8.11.4.4 and §10.7.5.
- The soft masks of §11.5 are now buildable. A soft mask is a group evaluated for its alpha or
  its luminosity, and the group machinery is what they were waiting for. That is 28 documents
  and the largest reported rendering gap left.
- A group costs a page-sized buffer in the CPU backend, allocated per group and not pooled.
  The corpus's heaviest first pages are unchanged in the gate's timing, so it is not paid for
  yet; a page with hundreds of groups would pay it, and the fix — bounding the buffer to the
  group's own band — is a coordinate-system change rather than a new idea.
