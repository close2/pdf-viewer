# ADR 0497 — The mechanism that explains the picture and not the verdict

Status: accepted, 2026-08-22. Session 672. Rewrites `CONTRADICTED_VISIBILITY_EXPRESSION`'s note
around measurements it never had; amends the §8.11.2.2 and §10.3.2 ledger rows and trap 9.
**No pixel moves and no list changes.**

## The sixth criterion, and why it is the escalation of the fifth

Five rounds have worked the oracle's contradicted pool and each left a sharper way of choosing than
it found: 643 wrote a page's closed form and ranked five renderers against the geometry; 651 found a
group's *name* holding while everything under it was wrong; 656 asked how many of a group's own
members its note measures; 662 asked how many clauses it cites; 668 asked whether the note names a
**mechanism** for the two voting references agreeing, and whether that mechanism is *verified*.

668's is the one to build on, because it audited a premise rather than prose. It established that a
named mechanism is real. It did not ask the next question, and the next question is the whole of
what a contradicted entry is for:

> **A contradicted entry is a standing exemption from a *specific failing bound*. 668 asked whether
> the mechanism a note names is verified. This round asks whether it is *sufficient*: does the named
> mechanism, priced from the file rather than from a picture, account for the measurement the gate
> actually fails us on — or only for the difference a person sees?**

Verified is not the same as enough. A note can name a real mechanism, check it against a binary, and
still be explaining a metric the page *passes* — which is precisely the failure trap 12 recorded on
one page in the six-hundred-and-sixty-second session ("check it against the metric that actually
fails") and which nobody had turned on the pool. The instrument is the gate's own line: it prints
four measurements and four bounds, so *which* bounds fail is on the screen already, and the question
is only whether the note's evidence is about them.

`oracle.rs` already contains the sentence this criterion inverts. `CONTRADICTED_MASK_QUANTISATION`
closes with **"A number stated correctly is not a mechanism explained"**, written by a round that
had a gate figure and no cause. The sixth criterion is its mirror: **a mechanism explained is not a
number accounted for.**

Read over all fourteen non-empty `CONTRADICTED_*` lists, the audit has three outcomes:

```text
  the note prices its mechanism against a bound the page fails                  5
    IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE   942 pixels in three columns and a row, against
                                       the worst tile it names, 5.03 of 5.00
    TIGHT_CONSENSUS                    the failing tile's closed form, and the bound
                                       beside ours to five digits on both colors.pdf pages
    GLYPH_EDGES                        says which subset fails the differing fraction and
                                       which the tightened mean, with both numbers
    ON_A_PAGE_WE_REPORT                all four bounds, and the three references' three
                                       different pictures behind them
    CALRGB_TO_SCREEN                   11.23% against the 8.82% the gate prints (ADR 0494)

  the mechanism is priced, in an aggregate the gate does not use                8
    SUBSTITUTED_FONT, DEVICE_CMYK_CONVERSION, SHARED_JBIG2_DECODER, LINK_BORDER,
    REFERENCES_DREW_NOTHING, REFERENCE_GLYPH_WIDTHS, SUBPIXEL_IMAGE,
    NEGATIVE_LINE_WIDTH
    — ink, ink ÷ length, cap rows, sampled channels, the coverage of one row: real
      measurements, and not one of them is one of the four the verdict is made of.
      Two of the eight do name which bound fails, without ever converting their cause
      into it.

  the note contains no measurement of its own page at all                       1
    VISIBILITY_EXPRESSION
```

**One entry of fourteen measures nothing.** It is four source citations and a clause, and its page
had never been put on a scale. That is the group, chosen by a rule rather than by taste, and the
criterion is spent by being applied like its five predecessors.

## What the page is

`visibility_expressions.pdf` page 1 is 340.1575 points square and its whole content stream fits on
a screen.
Five strings are set in a 36 pt embedded `FranklinGothic-Heavy` at `Td` y 260, 220, 180, 140 and
100 — once at `0 0 0 0.150 k`, then again at `0 0 0 0.890 k` inside five `BDC /OC` sections. The
five `/OCMD`s carry a `/VE` and no `/OCGs` and no `/P`; the default configuration is
`/OFF [10 0 R]`, so A and B are on and C is off:

```text
  oc1  /VE [/And 8 0 R 9 0 R]          A ∧ B              true    dark
  oc2  /VE [/Or  8 0 R 9 0 R]          A ∨ B              true    dark
  oc3  /VE [/Not 9 0 R]                ¬B                 false   pale
  oc4  /VE [/And 8 0 R [/Not 10 0 R]]  A ∧ ¬C             true    dark
  oc5  /VE [/Not [/Or 9 0 R 10 0 R]]   ¬(B ∨ C)           false   pale
```

Ours and `poppler` draw lines 3 and 5 pale. `mupdf`, `ghostscript` **and `hayro`** draw all five
dark. The side-by-side has five panels and the last of them was never described in the note.

## Which bounds fail

```text
  visibility_expressions.pdf page 1: CONTRADICTED — mupdf and ghostscript agree, we differ:
    ours at worst mean 3.89 worst tile 50.01 differing 6.38% ssim 0.9521;
    bound  mean 5.00 worst tile 40.00 differing 5.00% ssim 0.9000
```

Those four bounds are `Tolerance::TEXT_HEAVY` to the digit, so nothing was widened: the pair that
votes is 0.55 of 255 and 1.58% apart over their common 340 × 340 region, well under every floor.
**The worst tile and the differing fraction fail; the mean and the structural similarity pass.**

## The ladder, and it is the file taken apart rather than a renderer questioned

Two §7.5.6 incremental updates on the document itself, so nothing but the named objects moves.
The first replaces each `/OCMD`'s `/VE` with `/OCGs 8 0 R /P /AnyOn` — group A, which is on — so
the `/VE` question is gone and every renderer draws all five dark. The second also restates the
two `k` colours as the `rg` triples they reach in this tree and in `poppler`, so no processor's
source assumption can enter. Ours against `mupdf`, at the gate's own 72 dpi, through
`examples/compare_rasters`:

| | mean | worst tile | differing | ssim |
|---|---|---|---|---|
| the document | 3.8696 | 50.01 | 6.3456% | 0.95234 |
| `/VE` → `/OCGs` + `/P` | 0.9012 | 4.59 | 6.3086% | 0.99553 |
| … and `k` → the same `rg` | 0.5133 | 2.75 | **1.9378%** | 0.99589 |

**Removing the entire subject of this group moves the differing fraction by 0.037 of the 1.35
percentage points it is over by.** The page is still contradicted on that bound with `/VE` gone.
Split three ways, the 6.3456 points are

```text
  the /VE gap            0.037    0.6%
  the DeviceCMYK press   4.371   68.9%
  glyph edges            1.938   30.5%
```

and the worst tile is the other way round: `/VE` owns 45.42 of its 50.01. **One of the two failing
bounds is the group's and the other is 0.6% the group's.**

### Why one metric and not the others

`raster_compare`'s `JUST_NOTICEABLE` is 4. The two camps are 2 to 3 levels apart at `0 0 0 0.150 k`
and 7 to 11 apart at `0 0 0 0.890 k`, so the pale text is noise and every dark glyph pixel is not —
on a page that is nothing but dark glyphs. What those same pixels are worth to the other three
metrics is the ladder's second row: 0.90 of 255 against a mean bounded at 5.00, a worst tile of
4.59 against 40.00, and a similarity of 0.996 against 0.900 — all three comfortably inside. **A metric that counts pixels and a metric that averages them do
not see the same mechanism**, and a note that prices its cause in ink has answered the second.

## The gap, verified on the binaries that ran

The first variant is a control in both directions and it is stronger than the source citations the
note rested on, because it measures the programs installed on this machine rather than a tree
somebody read:

- `mupdf`, `ghostscript` and `hayro` render it **byte for byte** as they render the document
  (mean 0.0000, max 0). They were drawing all five dark either way.
- ours moves 3.4943 of 255 and `poppler`'s 4.6608, maximum 165 apiece.
- The other direction, so that "they ignore optional content" is excluded rather than assumed: a
  variant stating `/OCGs 10 0 R /P /AnyOn` — group C, the one `/OFF` names — is hidden by all three.
  They read §8.11 and they do not read `/VE`.

Of the note's four source citations, two were re-checked on this machine and reproduce (`poppler`'s
exported `OCGs::evalOCVisibilityExpr`, `pdf.js`'s `/VE`-first read in `src/core/evaluator_utils.js`,
which is the only one with a checkout behind it); one is inherited rather than re-checked, because
the machine carries `mupdf` 1.28.0 as a package and not as sources; and **one no longer reproduces
at all**: `strings` on `libgs.so.10` at 10.07.1 finds neither `OCMD contains VE, which is not
supported (ignoring)` nor `not supported (ignoring)`, and the invocation without `-q` prints nothing
about optional content. A citation of another project's source is a claim with no gate on it. The
behaviour is unchanged; the evidence for it had rotted, and the control replaces it.

**`hayro` is a fourth program with the same gap**, which the note never said and which matters
twice: it is the renderer that shares `skrifa` with this tree, and it makes the count three
implementations either way rather than three against two.

## Was the deciding clause the one the group cited?

**Half of it.** §8.11.2.2's `shall` — "If the VE key is present it shall be used in preference to
the OCGs and P keys" — decides the two hidden lines and therefore the worst tile, and this page is
the corpus's only witness for that sentence. The *other* failing bound is not that clause's at all:
it is §8.6.4.4 with §10.3.2's NOTE, four processors making four source assumptions with no clause
choosing among them (ADR 0484), which is `CONTRADICTED_DEVICE_CMYK_CONVERSION`'s row.

That is the fifth round running in which the deciding clause sat in a different row than the group
cites, and the first in which it sits in a different **group**.

## Consequences

- `CONTRADICTED_VISIBILITY_EXPRESSION`'s note is rewritten around the ladder above; its title names
  both mechanisms and which one the verdict is made of.
- §8.11.2.2's ledger row gains the corpus witness and what it is worth; §10.3.2's gains a second
  witness whose only colour is `k`, where the four assumptions are 7 to 11 levels apart rather than
  the 22 to 50 the shading pages show.
- Trap 9 gains a paragraph: where a page carries two of its mechanisms, price each, because the one
  a note is named for need not be the one the gate is failing.
- The verdict is unchanged, no page leaves the list, no pixel moves.

## Owed

- A criterion for the next round, and a harder problem than that: six rounds have each spent one,
  and the sixth had to be built out of a sentence already in the file. `doc/history/672-…` says what
  the pool looks like now.
- The eight entries in the audit's middle bucket are a *population*, not a finding. Each prices its
  mechanism in something the gate does not measure, and no round has asked any of them the question
  this one asked of the fourteenth.
- 0489's owed item stands and this round is another witness for it: nothing links a group's note to
  the gate figures it quotes, and a source citation in a note has no gate at all — which is how
  `ghostscript`'s warning survived its own binary.
