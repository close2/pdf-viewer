# One bound doing two jobs: the differing fraction on text pages

Status: **the question this item was opened for is answered** — the two jobs stay one number, and
the reason is measured rather than cautious. What is left is named at the bottom and is smaller
than the item was.
Opened in the four-hundred-and-seventh session, answered in the eight-hundred-and-forty-fourth.
Priority: 12 — demand-driven, and it is about the instrument rather than about a page
Code: `tools/pdfref/src/lib.rs` (`Tolerance`, `Judgement`, `widened_to`),
`tools/pdfref/src/reference.rs` (`substituted_cmyk_profile`),
`crates/pdf-model/tests/oracle.rs` (`the_fixed_bounds_against_the_references_own_spread`,
`substitutions_of`, `print_the_substitutions`, `the_excluded_reference_under_the_same_bound`,
`name_the_pages_the_excluded_reference_survives`)
Derivation and numbers: **ADRs 0243, 0717, 0771, 0772 and 0773**. Read 0771 first; it supersedes
the reason the other two give for leaving the bound alone without changing what they measured,
0772 corrects two populations it stated in prose, and 0773 reads the vector row it handed on.

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

## What is left

Three things were listed here; two are closed and none of the three is the number this item was
named for.

1. **The consensus half was never moved and its 278 pages are still a programme.** Raising
   `max_differing_fraction` for consensus formation makes 457 `ambiguous` pages judgeable and 278
   of them contradicted — ADR 0243 measured it. Several hundred are `doc/todo/00`'s dense-text
   population, which is the only mechanism anybody has named for why that bucket is the size it
   is. Nothing here argues for or against it; what ADR 0771 removes is the *floor* as a reason to
   go near it.
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

   **What is left of this item is one question, and it needs a population rather than a page.**
   Should a consensus whose two rasters are identical be a consensus at all? Acting on it needs a
   rule, a rule needs the base-rate control item 2's answer was built out of, and it would move
   `ambiguous` in the direction ADR 0243's 278 arrivals already point. Nothing here argues for it;
   what is measured is that on those pages the relative bound — the whole reason this gate judges
   the way it does — is not acting.

## Two neighbouring questions, both asked and answered elsewhere

- **Is a consensus of two the same evidence as a consensus of three?** ADR 0575: same kind, one
  factor less, and nothing moved. The six pages it printed are about why the third reference could
  not read the document, which is `doc/oracle-and-corpus.md` §3g.
- **Which pair forms the consensus, where two maximal sets exist?** ADRs 0616 and 0617: a verdict
  is one every maximal consensus reaches, and a page whose sets divide about us is `ambiguous`.
  Two residues, both recorded in trap 12: a *width* division and a *camp* division are treated
  alike, and 36 pages carry sets that concur in agreeing with us.
