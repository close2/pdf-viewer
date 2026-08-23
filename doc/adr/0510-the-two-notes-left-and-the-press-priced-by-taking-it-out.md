# ADR 0510 — The two notes left, and the mechanism priced by taking it out

Status: accepted, 2026-08-23. Session 680. Finishes what ADR 0499 left owed: the last two of the
eight middle-bucket `CONTRADICTED_*` groups, `SUBSTITUTED_FONT` (8 pages) and
`DEVICE_CMYK_CONVERSION` (5), against ADR 0497's sixth criterion — *a mechanism explained is not a
number accounted for*. Rewrites both notes, amends three ledger rows and adds one paragraph to
trap 9. **No pixel moves and no list changes.**

## What the criterion asks, and what it took

A contradicted entry is a standing exemption from a **specific failing bound**. So the question is
not whether the group's named mechanism is real — both of these are, and both were long ago measured
to a closed form with no renderer in it — but whether it accounts, *quantitatively*, for the number
the gate fails us on. `DEVICE_CMYK_CONVERSION` named the failing bound for none of its five pages
and `SUBSTITUTED_FONT` for one of its eight, and neither converted a single one of its figures into
the gate's units.

Both were answerable by **ablation** rather than by arithmetic: 675's two-group middle case, and it
was the right guess. The instrument in each case is trap 9's cheap half — *edit the document so
that one mechanism cannot act, and re-measure* — but the two edits are not the same kind of edit and
they need different controls.

- The font ablation **embeds the face the references already substitute**, so it disarms the
  mechanism for everybody. Its control is that the references must not move, and on seven of the
  eight documents they do not move by a single byte.
- The colour ablation **names a press in the document**, which reaches only our own code: §8.6.5.6 is
  a knob this tree honours, and the counterfactual is "if our source assumption had been theirs". Our
  ablated render is therefore compared against the references' renders of the **original** file, and
  there is no control to take because no reference was asked anything new.

**The gate's printed line is the worst-ratio member of the *agreeing consensus*, not of every
reference.** That is worth writing down because it is what makes either ablation checkable at all:
the figures the gate prints for `bug847420.pdf` are `mupdf`'s, not `ghostscript`'s, although
`ghostscript` is further from us on every one of the four. Reproduced on all thirteen pages by
recomputing `Distance::of`'s ratio per reference and taking the maximum inside the pair the verdict
line names.

## `DEVICE_CMYK_CONVERSION`: the press owns every failing bound on all five pages

### Which bound, per page — a thing the note had never said

| page | fails | bound |
|---|---|---|
| `function_based_shading_cmyk.pdf` p1 | mean, worst tile, differing | 1.00 / 5.00 / 1.00% / 0.9900 |
| `function_based_shading_cmyk.pdf` p2 | mean, worst tile, differing | 1.00 / 5.00 / 1.00% / 0.9900 |
| `postscript_type4_many_outputs.pdf` p1 | mean, worst tile, differing | 1.00 / 5.00 / 1.56% / 0.9900 |
| `transparent.pdf` p1 | **the differing fraction alone** | 1.00 / 5.00 / 1.38% / 0.9900 |
| `type4psfunc.pdf` p1 | worst tile, differing | 1.00 / 5.00 / 1.00% / 0.9900 |

### The arithmetic, on the note's own sampled colours

`transparent.pdf` is the page where the conversion is written out in one line, so it is the page
where the conversion into the gate's units is exact. The note's own table samples the bottle at
ours (28, 32, 40) against `ghostscript` (25, 35, 46) — differences of **3, 3 and 6 levels**.
`raster_compare`'s `JUST_NOTICEABLE` is **4**, so exactly one of the four channels crosses it, and
the differing fraction is the mark's own area divided by four:

```text
  ours' ink                                      11.4175% of the page
  pixels with any RGB channel over four levels   11.5175%
  blue alone, as a share of all four channels     2.8750%   = 11.50% ÷ 4
  red and green, at the silhouette's edge only    0.4413%
  printed by the gate                             3.3163%   against a bound of 1.38%
```

**The whole failing measurement is two levels of blue.** Had the press difference on this ink been
four levels rather than six in every channel, the page would report a differing fraction of about
0.44% and agree. That is the sixth criterion answered in the units the gate uses, and it is also
the sharpest instance this tree has of §10.3.2's licence costing a verdict.

### The ablation: name the press in the document and every bound goes inside

§8.6.5.6 gives a document a way to say what its `DeviceCMYK` *is*, and this tree honours it. So the
counterfactual "if our source assumption had been theirs" is a §7.5.6 incremental update adding a
`/DefaultCMYK` naming an `ICCBased` stream, and nothing in this tree's code moves. Our ablated
render is compared against **the references' renders of the original file**, which is what the
counterfactual means and is why no control on the references is needed here.

Worst-ratio member of the consensus pair (`mupdf` and `ghostscript` on all five), against each
page's own bound:

| page | ours | with `hayro`'s CGATS press | with Artifex's SWOP press |
|---|---|---|---|
| `function_based_shading_cmyk.pdf` p1 | 2.70 / 10.68 / 17.25% / 0.9959 | 0.48 / 1.74 / 0.19% / 0.9988 | 0.44 / 1.57 / 0.72% / 0.9992 |
| `function_based_shading_cmyk.pdf` p2 | 5.15 / 19.47 / 29.19% / 0.9956 | 0.41 / 3.15 / 0.46% / 0.9988 | 0.91 / 4.54 / 2.19% / 0.9987 |
| `postscript_type4_many_outputs.pdf` p1 | 7.30 / 18.04 / 37.25% / 0.9942 | 0.39 / 0.74 / 0.85% / 0.9976 | 0.60 / 1.20 / 2.00% / 0.9978 |
| `transparent.pdf` p1 | 0.65 / 3.18 / 3.32% / 0.9952 | 0.41 / 1.77 / 0.66% / 0.9953 | 1.07 / 5.97 / 8.64% / 0.9924 |
| `type4psfunc.pdf` p1 | 0.31 / 6.70 / 1.29% / 0.9998 | 0.12 / 1.72 / 0.07% / 0.9954 | 0.13 / 1.77 / 0.44% / 0.9954 |

With the CGATS press assumed, **every one of the five is inside every bound**, the largest ratio
anywhere in that column being 0.63 of what the page allows. The group's named mechanism owns
**100% of every failing measurement on all five pages**, which is the strongest of the three
outcomes ADR 0499 found and the first group to reach it by ablation.

This prices the mechanism; it does not license the change. §10.4.2.1 ranks §10.3's ICC route above
§10.4.2.5's formula, §10.3.2's NOTE licenses a *source* assumption, and `CMYK_CORNERS` is one —
adopting somebody's press because it moves five pages is exactly the curve-fitting principle 5
forbids, and the arithmetic above is what that refusal costs, written down.

### And the two presses are not interchangeable through *our* evaluator, at the shadow end

The Artifex column is the finding. On the three shading pages either profile lands us in the
reference camp; on `transparent.pdf` the same file that `mupdf` and `ghostscript` are both reading —
187 484 bytes, one digest, ADR 0048 — evaluated by `pdf_model::icc` puts the bottle at
**(36, 44, 53)** where all three renderers are within a level of (25, 34, 45), while `hayro`'s
8 464-byte CGATS profile through the same evaluator gives **(25, 34, 44)**. Eleven levels on one
side, one on the other, on one flat ink.

It is not the rendering intent — Artifex's `A2B0` and `A2B1` point at the *same* 41 478 bytes — and
`icc.rs` had already predicted it in prose:

> An alternative construction estimates a *perceptual* [black point] by round-tripping through the
> profile's `B2A` table; readers built on Little CMS take that route, and the two agree everywhere
> except in the darkest few percent.

`0.82 0.7 0.54 0.67 k` is the darkest few percent, and the CGATS profile carries no `B2A` at all, so
the two constructions have nothing to disagree about there. That paragraph now has its number.

**The consequence for trap 9's sixth bullet is a correction of scope**: "this tree's own evaluator
on **either** of the two predicts all three renderers to within eight levels" is true of the
sampled ramp, where it was measured, and false by eleven levels of the one deep-shadow ink in the
same group. A claim about a profile is a claim about the region of the space it was sampled in.

## `SUBSTITUTED_FONT`: the face owns the failing bound on seven of eight

### Which bound, per page — five of the eight are the shape the note itself moved five pages out for

| page | consensus | fails |
|---|---|---|
| `bug847420.pdf` p1 | `poppler` + `mupdf` | mean 8.45/5.00, differing 9.16%/8.09%, ssim 0.8581/0.9000 |
| `issue15716.pdf` p1 | `mupdf` + `ghostscript` | mean 14.03/5.00, differing 9.22%/6.06%, ssim 0.6886/0.9000 |
| `issue9243.pdf` p1 | all three | **structural similarity alone**, 0.8907/0.9000 |
| `bug850854.pdf` p1 | `poppler` + `mupdf` | **the differing fraction alone**, 5.38%/5.00% |
| `issue11403_reduced.pdf` p1 | `poppler` + `mupdf` | differing alone, 6.25%/5.00% |
| `issue6069.pdf` p1 | `poppler` + `mupdf` | differing alone, 6.62%/6.55% |
| `issue6108.pdf` p1 | `poppler` + `mupdf` | differing alone, 6.55%/5.75% |
| `issue7580.pdf` p1 | `poppler` + `mupdf` | differing alone, 6.92%/5.00% |

The note's own discriminator, written in the four-hundred-and-thirty-first session, moved five
`/Times-Roman` pages out of this group because "[e]ach of the five fails exactly one of the four
bounds and it is the differing fraction … **That is `CONTRADICTED_GLYPH_EDGES`' diagnosis and not
this one's**". Five of the eight that stayed are that same shape. Applying the discriminator as
written would empty most of the group; the question is whether the cap height owns those five
differing fractions or whether the glyph edges do, and no arithmetic in the note answers it.

### The ablation: embed the face the references substitute

`gs -sDEVICE=pdfwrite -dEmbedAllFonts=true -dSubsetFonts=false -c "<</NeverEmbed[]>>
setdistillerparams"` writes the page back with `ghostscript`'s own fontconfig resolution embedded as
a `/FontFile3`. Every renderer then draws one program and §9.5 NOTE 5's mechanism cannot act at all.

**The control is as clean as this instrument gets**: on seven of the eight files `poppler`, `mupdf`
and `ghostscript` render the rewritten document **byte-identically** to the original — mean 0.0000
on every channel — so the rewrite changed the font program and nothing else, and the page's bound,
being derived from the references' distance from each other, is unchanged too. The exceptions are
`mupdf` moving 0.09 on `bug847420.pdf` and 0.08 on `issue11403_reduced.pdf`, and `poppler` moving
0.72 on `issue15716.pdf` — where `poppler` is not in the consensus pair and both members of it are
byte-identical.

Worst-ratio member of the named consensus, before and after:

| page | ours | with the face embedded | |
|---|---|---|---|
| `bug847420.pdf` p1 | 8.45 / 16.01 / 9.16% / 0.8581 | 1.62 / 3.77 / 6.51% / 0.9928 | inside |
| `bug850854.pdf` p1 | 2.76 / 10.39 / 5.38% / 0.9758 | 1.01 / 4.26 / 4.22% / 0.9964 | inside |
| `issue11403_reduced.pdf` p1 | 2.55 / 5.32 / 6.25% / 0.9795 | 0.80 / 1.69 / 4.78% / 0.9975 | inside |
| `issue15716.pdf` p1 | 14.03 / 31.31 / 9.22% / 0.6886 | 3.28 / 10.55 / 4.94% / 0.9461 | inside |
| `issue6069.pdf` p1 | 2.41 / 5.33 / 6.62% / 0.9836 | 1.46 / 4.19 / 5.97% / 0.9936 | inside |
| `issue6108.pdf` p1 | 2.35 / 4.94 / 6.55% / 0.9791 | 1.48 / 3.58 / **5.89%** / 0.9926 | **still out, by 0.14 points of 5.75%** |
| `issue7580.pdf` p1 | 2.92 / 6.93 / 6.92% / 0.9753 | 0.80 / 1.66 / **4.99%** / 0.9978 | inside by 0.0125 points |
| `issue9243.pdf` p1 | 3.13 / 16.29 / 3.05% / 0.8907 | 0.92 / 4.49 / 1.85% / 0.9793 | inside |

**Seven of eight go inside every bound they were failing.** The substituted face owns the whole of
the mean on `bug847420.pdf` (8.45 → 1.62 against 5.00), the whole of the structural similarity on
`issue9243.pdf` (0.8907 → 0.9793 against 0.9000) — the one page in this file whose only failing
measure is `ssim`, and the one where the note's *symbolic* mechanism has nothing to do with it —
and the whole of `issue15716.pdf`'s three, whose 25% ink deficit the note had already derived from
the two font programs' charstrings.

**`issue6108.pdf` is the eighth and it carries two mechanisms.** The face owns 0.66 of the 0.80
points by which it misses the differing bound — 82% — and the residue, 5.89% against 5.75%, is a
sub-pixel glyph-edge population that would keep the page contradicted with the substitution
entirely removed. That is trap 9's *a page can carry two of the eight* in a group that is not
`visibility_expressions.pdf`, and it is recorded rather than moved: a mechanism owning 82% of a
bound is the group's, and the second one is named beside it.

The margin on `issue7580.pdf` — inside by 0.0125 of a percentage point — is stated at the precision
it was measured and is not evidence of anything beyond "the face owns essentially all of it". Five
of these pages sit within a point of a bound in either direction, which is what a 200 × 50 line of
text does to a metric that counts channels.

### And two figures in the note do not reproduce

The note's `bug847420.pdf` paragraph opens with the head of a **second** ranking, in levels of 255,
which `rank_the_contradicted`'s own doc comment describes as a hand-built one taken off the
artefacts. The unit is named and is a real one — this is *not* ADR 0499's misread-unit shape — but
both figures are wrong about their operand, on rasters that have not changed since:

- **8.65** is our distance from **`hayro`** (8.6520), which is the *furthest* of the four references
  and the one that does not vote. From the nearest of the four we are **7.44** (`poppler`).
- **4.64** for the four references' spread among themselves reproduces in no pairing; their six
  pairwise means run **1.38 to 3.48**.
- "twice as far as any page on the list that is not a link border" is false: `issue15716.pdf` sits
  13.96 from its nearest reference in the same unit.

Our render has not moved — the note's own ink ladder, ours 12.955 and `poppler` 13.224, reproduces
to 12.952 and 13.224 — so these are not figures that decayed. They were the wrong end of the range
when they were written.

## What this leaves

The eight groups ADR 0497 named are done. Three account for their bound by arithmetic, two by
ablation, one accounts for none of it, and these two account for theirs by ablation as completely as
the instrument can show — five pages at 100%, seven of eight at 100%, one at 82% with its second
mechanism named. **No group in that bucket turned out to be an unearned exemption.**

What has no instrument is unchanged from 675: nothing links a group's note to *which* bound the gate
fails its pages on. Every one of the thirteen diagnoses here started by reading that off a log by
hand, and a note can go on explaining a mean while the page fails on a differing fraction for as
long as nobody looks. `--bin quoted` checks a figure a note quotes; it cannot ask for one that is
missing.
