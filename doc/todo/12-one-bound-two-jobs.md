# One bound doing two jobs: the differing fraction on text pages

Status: **answered, and nothing here is owed.** The two jobs stay one number, and the reason is
measured rather than cautious — from both ends since the eight-hundred-and-forty-ninth session,
which found that they are not two jobs a threshold can separate in *either* direction. The file
is kept rather than deleted because twenty-one comments in `crates/` and `tools/` point a reader
at it, and every decision in it is in the ADRs below.
Opened in the four-hundred-and-seventh session, answered in the eight-hundred-and-forty-fourth
and closed in the eight-hundred-and-forty-ninth.
Priority: 12 — demand-driven, and it is about the instrument rather than about a page
Code: `tools/pdfref/src/lib.rs` (`Tolerance`, `Judgement`, `widened_to`),
`tools/pdfref/src/reference.rs` (`substituted_cmyk_profile`),
`crates/pdf-model/tests/oracle.rs` (`the_fixed_bounds_against_the_references_own_spread`,
`substitutions_of`, `print_the_substitutions`, `the_excluded_reference_under_the_same_bound`,
`name_the_pages_the_excluded_reference_survives`, `ConsensusIdentity`,
`the_consensus_that_decided_it`, `what_the_consensus_was_made_of`, `RaisedFormation`,
`a_raised_formation_bound`, `what_the_new_convictions_are_made_of`,
`the_pages_a_raised_formation_bound_would_move`), `tools/pdfref/src/lib.rs`
(`Triangulation::rejudged`, and `decide`'s two tolerances)
Derivation and numbers: **ADRs 0243, 0717, 0771, 0772, 0773, 0774 and 0776**. Read 0771 first; it
supersedes the reason the other two give for leaving the bound alone without changing what they
measured, 0772 corrects two populations it stated in prose, 0773 reads the vector row it handed on,
0774 answers the question 0773 handed on by widening its denominator — which is 0771's own
general shape arriving one round later — and **0776 does the same for the consensus half**: the
278 are composed rather than counted, and the formation bound turns out to move our own floor
through `widened_to`.

## What was asked

`Tolerance::TEXT_HEAVY::max_differing_fraction` is 0.05 and does two jobs: `Tolerance::accepts`
decides whether two references form a **consensus**, and the same number **floors** the per-page
bound `widened_to` derives. ADR 0243 measured that it sits below its own references' spread and
left it, because separating the two needed a floor derived from a pair including a non-hinting
renderer with neither member ours, and the only candidate shares `skrifa` with this tree.

## What was found

**The population that requirement asked for existed all along, behind an `ldd`.** `poppler` and
`mupdf` load one `libfreetype.so.6`; `ghostscript` names none and carries a statically linked
copy. So `ghostscript` against either is two separate FreeType copies, neither member ours — a
weak independence, being one algorithm twice, and enough to derive a floor from. Split that way,
on text pages, the differing fraction runs median 0.86% within the sharing pair and 2.50% across
the boundary, while the class's other three measures do not move across it at all.

**The floor was derived at ADR 0243's own rule — the 99th percentile — implemented, and priced.**
Floored at 12.04% for our judgement alone with consensus formation untouched, the corpus gate
reports 1017 agrees / 24 contradicted / 835 ambiguous against 980 / 60 / 836: 36 pages leave
`contradicted` and none arrives.

**It was not taken, and six named pages are why.** Five are `CONTRADICTED_CALRGB_TO_SCREEN` and
one is `CONTRADICTED_SUBPIXEL_IMAGE` — a §8.6.5.3 colour reading and a §10.7.4 departure, each
measured in its own note. A differing fraction is a threshold count, so 5–12% of it is reached
either by a sub-pixel phase on every glyph edge or by a small colour error over a large area, and
**a bound cannot separate what a mechanism separates**. That is the answer: not that no number
could be derived, but that the measure the number bounds conflates two mechanisms.

**And the *different verdict* branch is closed by a base rate.** The candidate rule was trap 12's
control — where the consensus would contradict the voting reference it excludes, the bound is not
one an independent implementation meets. The gate counts it every run now, and it holds on **52 of
the 60** contradicted pages, across the JBIG2 pages, the colour pages and the link border alike.
ADR 0717's *32 of 32* is the pool's base rate rather than that population's signature; a rule
resting on it would acquit us wherever two references agree for any reason at all. (0717's own
figure is **31 of 32** when the gate counts it rather than a document quoting it, and the
exception is `freeculture.pdf` page 313 — ADR 0772.)

**What replaced `widened_to`'s standing request.** It asked for "a measurement of how far a
*fourth* independent rasteriser sits from the three", and `pdfium` is still not packaged. The
question a verdict asks needs no fourth renderer: `decide` takes the *closest* pair in the room,
so the bound is a selected minimum and what a third implementation owes is to sit as near that
pair as the excluded one of three manages. `substitutions_of` measures exactly that, by running
the gate's own judgement with a reference standing where our render stands — and the answer is
that the same 5% floor convicts a known-good reference on 0.6% of text pages under the one
consensus whose members do not share the FreeType object and on 9.1% under the one whose members
do. **The bound is not what varies; the consensus is.**

Re-run either derivation at any time — one command, both tables:

```sh
PDFVIEWER_ORACLE_SPREAD=1 cargo test --profile gates -p pdf-model --test oracle -- \
    --ignored --nocapture the_fixed_bounds_against_the_references_own_spread
```

And re-run the shared-profile removal beside it — the same command with one variable, because the
cache keys on the invocation and will re-render `ghostscript` rather than answer from the
baseline's renders. Its control is the *third* line, which must reproduce the baseline byte for
byte:

```sh
PDFREF_GS_CMYK_PROFILE=/usr/share/ghostscript/iccprofiles/ps_cmyk.icc        # a different press
PDFREF_GS_CMYK_PROFILE=<a CGATS copy>                                        # the same press: a null
PDFREF_GS_CMYK_PROFILE=/usr/share/ghostscript/iccprofiles/default_cmyk.icc   # the control
```

## What was left, and is not any more

Three things were listed here, and all three are closed; none of them was the number this item was
named for.

1. ~~**The consensus half was never moved and its 278 pages are still a programme.**~~ **Read in
   the eight-hundred-and-forty-ninth session, and the threshold does not move** (ADR 0776). The
   gate runs the raise as a counterfactual every run — `pdfref::Triangulation::rejudged` over the
   comparisons it already holds, calibrated by an assertion that re-judging at a page's own bounds
   reproduces its own verdict — so the composition is printed rather than described.

   **The population is one book.** Of the 276 pages the raise newly convicts, **272 are
   `freeculture.pdf`** and the other four are one page each; 275 of 276 are text pages, and the
   same derivation applied to the vector class moves one page in the corpus. This item's guess
   that "several hundred are `doc/todo/00`'s dense-text population" was right and is now counted:
   it is that population and almost nothing else, which is `doc/todo/00`'s work rather than this
   bound's.

   **The convictions are on a measure the raise does not touch.** 274 of the 276 are convicted on
   structural similarity — the bound `Tolerance::TEXT_HEAVY` set at 0.90 precisely to put font
   substitution in `ambiguous` — 2 on the worst tile, and **none on the differing fraction**. On
   **263 of the 276** a reference agrees with *us* more closely, on that same deciding measure,
   than the convicting set agrees with itself; on `freeculture.pdf` page 100 ours against `mupdf`
   is ssim 0.9558 where the convicting pair is 0.9315 and 11.19% of channels apart. And 205 of the
   276 form only past 10% differing, so a smaller raise buys a seventh of them at the same price.

   **And the price is ADR 0771's, which is the finding that closes this item.** Raising formation
   *alone*, with our own floor left at the class bound, still acquits **27 of the 60** contradicted
   pages — including all five `CONTRADICTED_CALRGB_TO_SCREEN` pages and `CONTRADICTED_SUBPIXEL_IMAGE`,
   every one of the six ADR 0771 refused the floor raise for. `widened_to` derives our bound from
   the spread of whatever set formed, so admitting a wider set widens what we are held to. **The
   two knobs this file is named after are one knob**, and ADR 0243's narrow move and its mirror are
   the same change reached from two directions.
2. ~~**The three pages the control does not excuse.**~~ **Read in the
   eight-hundred-and-forty-fifth session, and closed** (ADR 0772). Two things came out of it. The
   population is not what this item said — the gate names it now rather than a document naming it,
   and it is `bug847420.pdf` page 1, `issue7891_bc1.pdf` page 1 and `freeculture.pdf` page **313**,
   where `issue19633.pdf` is convicted by the control like the other 52. And on each of the three
   the reason is in its group's note: three references drawing **one substituted face** on the
   first, a reference inside the bound by 0.06 of a level while 25.6× further from the page's own
   closed form on the second, and a bound that falls *inside* the continuous spread of the pairs
   that do not define it on the third. **None of the three is an accusation, and the shape they
   share is the finding**: the control asks where a renderer sits on the deciding measure, so it
   fires whenever we are the extreme of an ordering — which is what being on the clause looks like
   when the pair that sets the bound departs in one direction. Nothing is owed. What is left of
   this item is items 1 and 3.
3. ~~**The vector row of the substitution table, which nobody went looking for.**~~ **Read in the
   eight-hundred-and-forty-sixth session, and the hypothesis was wrong** (ADR 0773). Trap 9's
   shared-data bullet was the guess; the removal that prices it —
   `gs -sDefaultCMYKProfile=`, now `PDFREF_GS_CMYK_PROFILE` in the harness so that the cache sees a
   changed invocation — moves **five** of the 119, and they are
   `CONTRADICTED_DEVICE_CMYK_CONVERSION` exactly, the group ADR 0048 gave that mechanism eight
   hundred sessions ago. **The shared profile owns 4.2% of the row and nothing outside its own
   group.** It owns five of *our* thirteen, which is the honest direction.

   **A null removal needed a control of its own, and that is the transferable half.** Substituting
   hayro's CGATS profile first moved *nothing* — same 226 pages, same 119, page lists `diff`-clean
   — because Artifex's `desc` says SWOP and CGATS TR 001 is the data SWOP publishes: 0.1257 of 255
   between the two renders, against 12.9954 for `ghostscript`'s own `ps_cmyk.icc`. A shared-data
   removal that substitutes another copy of the same press has taken the file and left the
   mechanism.

   **What the row is actually made of is a stronger mechanism than any on trap 9's list**, and it
   is now a bullet there: on **97 of the 117** of those pages that can be compared, `mupdf` and
   `ghostscript` are **pixel-identical** (`max 0`), all 76 of the `bitmap-*` family among them. So
   `widened_to` widens nothing — twice zero is zero — and `poppler` is held to the bare
   `Tolerance::VECTOR` floor, failing on structural similarity at a median 0.97549 against 0.9900.
   The convictions are 116 on the worst tile and 109 on the similarity against 45 on the mean,
   which is why a colour profile could never have owned them.

   ~~**What is left of this item is one question, and it needs a population rather than a page.**~~
   **Asked and answered in the eight-hundred-and-forty-seventh session, and the answer is no rule**
   (ADR 0774). *Should a consensus whose two rasters are identical be a consensus at all?* The gate
   counts identity over the whole population now — `what_the_consensus_was_made_of`, every run, off
   comparisons it already had — and it is **176 of the 1044 pages a consensus decides**, with the
   bound at the bare class floor on **629** of them, which is the wider fact nobody had: on three
   pages in five that a consensus decides, `widened_to` widened nothing at all.

   **Identity is a property of the page rather than of the pair, and three readings say so.** It
   runs at **0.4% on text pages against 68.9% on vector ones**, where a mechanism of dependence
   would show at one rate on both — two Artifex programs share their code on every page they draw.
   It is **depleted** in the pool where a manufactured consensus would cost something, 6.7% of the
   contradicted pages against 17.6% of the agreeing ones, so 172 of the 176 are agreements and a
   rule would move `ambiguous` by that many in exchange for four convictions. And **68 of the 176
   are a three-way identity including `poppler`** — the reference item 3's own row excludes — where
   a rule refusing them would turn the strongest agreement the instrument can record into no
   evidence, inverting ADR 0005. `Tolerance::widened_to`'s doc comment said it first: *a spread of
   zero — two references producing identical pixels, which happens on simple pages*. The floor
   exists for this population.

   **And the four convictions are the group already named for the mechanism**, which is why nothing
   moved: three are `CONTRADICTED_SHARED_JBIG2_DECODER` — `jbig2dec` twice, whose *right* answer is
   ADR 0381's out of the documents themselves — and the fourth is
   `CONTRADICTED_ON_A_PAGE_WE_REPORT`. The gate names them rather than this file.

## Two neighbouring questions, both asked and answered elsewhere

- **Is a consensus of two the same evidence as a consensus of three?** ADR 0575: same kind, one
  factor less, and nothing moved. The six pages it printed are about why the third reference could
  not read the document, which is `doc/oracle-and-corpus.md` §3g.
- **Which pair forms the consensus, where two maximal sets exist?** ADRs 0616 and 0617: a verdict
  is one every maximal consensus reaches, and a page whose sets divide about us is `ambiguous`.
  Two residues, both recorded in trap 12: a *width* division and a *camp* division are treated
  alike, and 36 pages carry sets that concur in agreeing with us.
