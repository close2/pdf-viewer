# 0483 — Three clauses that were said to meet at one operator

Status: accepted.
Session: 655. Follows ADR 0479, which reported that a shading pattern's colours are resolved at the
`scn` rather than at the mark, wrote that "a round that moves it should move all three" — §10.5's
transfer function, §8.6.5.9's black point and §11.4.7's compositing target — and priced the fix as
one change. **The three were then read verbatim and they name three different moments**, one of
which is *earlier* than the `scn`. This ADR is that reading and the two halves it made
implementable.

## The decision

**A shading pattern's definition is evaluated under the graphics state the content stream holding
its `scn` began with, augmented by Table 75's `/ExtGState`** — for §8.6.5.9's black point
compensation, §8.6.5.8's rendering intent and §10.7.3's smoothness. `content::PatternInitial`
carries them and `Interpreter::run` scopes them, exactly as `Interpreter::base` is scoped.

**A group that a shading pattern is carried into does not composite in a press of its own.**
`group_press` gains the condition, beside the two it already had for the same reason.

**§10.5's transfer function stays where it is and stays reported.** It is not one of §11.6.7's
parameters, and the sentence that says so is in §11.7.5.3.

## What each clause actually says about *when*

### §11.6.7 — the pattern's definition, and it is not the `scn`

> The definition shall not inherit the current values of the graphics state parameters at the time
> it is evaluated; those parameters shall take effect only when the resulting pattern is later used
> to paint an object.

> Any parameters that are not so specified shall be inherited from the graphics state that was in
> effect at the beginning of the content stream in which the shading pattern is set to be the
> current colour in the graphics state or in which the sh operator is used.

> In the case of a shading pattern, the parameter values may be augmented by the contents of the
> ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only those parameters
> that affect the sh operator, such as the current transformation matrix, black point compensation
> and rendering intent, shall be used. Parameters that affect path-painting operators shall not be
> used, since the execution of sh does not entail painting a path.

Table 75 says the same thing from the other end, in its own words: parameters the pattern's
`/ExtGState` does not state are "inherited from the graphics state that was in effect at the
beginning of the pattern's parent content stream, and as modified by clause 11.6.7".

**Both of the obvious answers are therefore wrong.** Resolving at the `scn` is what this tree did;
resolving at the mark is what ADR 0479 priced; the clause names a third moment, before both.

**And this tree had been obeying that sentence for one of the three parameters since the
fifty-second session without anyone noticing it was the same sentence.** §8.7.2 maps a pattern's
matrix to "the default coordinate system of the pattern's parent content stream", which is
`Interpreter::base`, swapped at each of the four ways of becoming a parent — the transformation
matrix is the first parameter §11.6.7's third bullet names. That is trap 5's shape stated over a
*clause* rather than over an entry: **where one rule governs a set of parameters, implementing it
for one of them is the failure mode that reports nothing.**

### §8.6.5.9 — and why the clause's own silence is not the issue here

> If the value is not given or set to Default , then the behaviour is left to the PDF processor to
> determine.

That silence is real and is trap 6's standing example, and it is about *whether* to compensate, not
about *when the parameter is read*. The when is §11.6.7's above, which names black point
compensation in as many words. The two questions had been conflated.

### §11.4.7 and §11.7.2 — the compositing target is the mark's group

§11.6.7's first sentence about the implicit group is the load-bearing one:

> In both cases, the pattern definition shall be treated as if it were implicitly enclosed in a
> non-isolated transparency group: a non-knockout group for tiling patterns, a knockout group for
> shading patterns.

and §11.7.2 then says where a non-isolated group's space comes from:

> Non-isolated groups shall inherit their colour space from the nearest ancestor isolated parent
> group (subject to special treatment for the page group, as described in 11.4.7, "Page group").

The nearest ancestor of a pattern *painted inside* a group is that group. So the colours belong in
the space the mark composites in, and this tree resolves them in the space the `scn` was in.

### §11.7.5.2 and §11.7.5.3 — the transfer function, and only it, belongs to the mark

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

and the NOTE in §11.7.5.3, which is what takes it out of the pattern's evaluation altogether:

> This differs from the current halftone and transfer function, whose values are used only when all
> colour compositing has been completed and rasterization is being performed.

A `/TR` in a pattern's `/ExtGState` therefore says nothing about the pattern's own colours, and is
deliberately not read.

### The `sh` operator, which the same sentence names and which does not move

§11.6.7's second bullet says "or in which the sh operator is used", which taken literally would make
a `ri` earlier in the same content stream unreachable by the `sh` after it. §11.7.5.3 answers it for
an elementary object painted where it stands:

> When painting an elementary object with a CIE-based colour into a transparency group having a
> different colour space, the rendering intent used shall be the current rendering intent in effect
> in the graphics state at the time of the painting operation.

and Table 76 points the same way for the third parameter — an `sh`'s coordinates "are interpreted
relative to the current user space". So `paint_shading` keeps reading the state it stands in, and
`a_shadings_ramp_honours_the_rendering_intent` is the test that would fail if that changed.

## What the errata say

`cargo run --release -p spec-errata -- emit doc/*.pdf` over all fourteen documents, clauses 8 and 11
first as the round was told: **no annotation falls in §8.6.5.9, §8.7.4.1, §11.4.7, §11.6.7, §10.5,
§11.7.5.2 or §11.7.5.3.** The nearest are §8.6.5.8's one strikeout, whose text — "does not have to
support all PDF rendering intents and" — is found in §8.6.5.8's own NOTE at line 5222 of `doc/md/`
and so does not stray into the next subclause; §8.7.3.1's two, which §8.7.3.1's ledger row already
carries; §11.4.8's two `a`→`α` corrections inside its own formulas; and §11.6.6's three. Confirmed
rather than assumed, which is what a round filing a caret by the page its heading opens on owes.

## The population, measured before the code and over both corpora

`examples/pattern_state_census` is new and states each condition as the clause states it.

- **`doc/pdf.js`**: 964 open, 38 hold a `/PatternType 2` object (601 of them), **0 state Table 75's
  `/ExtGState`**, **0** can see the black point move, 2 hold a four-component group `/CS`.
- **The SafeDocs crawl**: 65 703 open, 1504 hold one (36 527 patterns), **42 state an `/ExtGState`**,
  **0** can see the black point move, 211 hold a four-component group `/CS`.

Two things follow and both are the point of measuring first.

**The ledger's §8.7.4.1 row was wrong about the world and right about its corpus.** It claimed since
the six-hundred-and-sixteenth session that no corpus document writes an `/ExtGState`, re-derived
then over `doc/pdf.js` and `doc/corpora/`. Both figures still hold. The crawl was not in either
population, and it holds 42 — whose keys are `/SM`, `/RI`, `/HT`, `/OP`, `/op`, `/OPM`, `/SA`,
`/BG2`, `/UCR2` and one dictionary's `/BM /CA /ca /SMask /AIS /TK`. **A negative claim about a
population decays when the population grows**, which is `doc/todo/01`'s sixteenth sweep stated one
turn further round.

**The black point's population is zero and that is said plainly.** Compensation moves only an ICC
conversion (`ColourSpace::to_rgb_at`'s `Icc` arm and nowhere else), so a pattern shading in a device
space cannot see the parameter at all; no document in either corpus both shades a Type 2 pattern in
a CIE-based space and states `/UseBlackPtComp` or an absolute intent. The fixtures are hand-built
(trap 8) and each was run against the defect it guards.

## What moves, and it is one page

`examples/raster_digest` on both arms:

- **`doc/pdf.js`'s 974 first pages: byte-identical.**
- **The 251 crawled documents the census named: one differs.** `6696799.pdf` states `/SM 0.002` in
  a shading pattern's `/ExtGState` over a type 0 sampled function of 256 samples, so §10.7.3's
  tolerance asks for a finer sampling of the ramp than this device's default and now gets it.
  **6688 pixels of 3 601 584 change, by at most one level of 255**, and the page's ink is 16.4368
  either way. The page is a newspaper spread and looks the same; it was opened rather than assumed
  (trap 1).

So the argument for the change is the clause, and the corpus is evidence that nothing else moved
rather than evidence that anything was broken.

## Why the compositing target was answered by a refusal rather than by a rebuild

`group_press` already declines to build a subtractive pair in two situations whose reason is
identical to this one, and one of them is written down in its own doc comment: an uncoloured tiling
cell's marks "carry a colour resolved for the *parent's* compositing, and reinterpreting them in ink
would convert a colour that was never stated here". A shading pattern selected before the `Do` is
that sentence one colour over. The pair's two runs would otherwise carry one device-resolved colour
into both the chromatic and the black half, which is an ink neither the file nor this tree ever
stated.

Refusing is exact, it is three lines, and it puts the page back on the construction that was right
before the pair existed — with §11.6.6's standing report naming the space rather than silence. The
alternative is the rebuild §10.5's row still owes, which would resolve the pattern's colours at the
mark and get this for nothing; when that lands, this condition comes off. It is written here so
that the next round does not have to re-derive why it was ever on.

The condition is an over-approximation in the safe direction — the test is the pattern being the
current colour at the `Do`, and the group may never fill with it — and it costs nothing measurable:
no page on this disk changes.

## What was not done

- **The rebuild at the mark**, which is §10.5's remaining half and is `doc/todo/13`'s. Its price is
  unchanged in substance and smaller in scope: one clause rather than three.
- **`/BG2` and `/UCR2` in a pattern's `/ExtGState`** are §11.7.5.3's conversion parameters, which
  this device performs nowhere. `Interpreter::note_black_generation` records them so that the
  *second* route to a statement `gs` already makes reaches the same page-wide flag, which is trap
  5's rule applied literally; what happens then is the standing decision in §11.4.7's row.
- **`/HT`, `/OP`, `/op`, `/OPM`, `/SA`** keep the answers §10.6, §8.6.7 and §11.6.7's own
  path-painting exclusion give them. `content/pattern.rs` carries the bucket each of Table 57's
  entries falls into and the sentence that puts it there, because a reader will otherwise ask why
  three entries are read and twenty are not.
