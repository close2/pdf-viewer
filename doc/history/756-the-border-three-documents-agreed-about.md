# 756 — The border three documents agreed about

Sent to settle one pixel: whether §12.5.4 puts a one-unit annotation border on `/Rect`'s edge or
inset by half its width, on `bug766086.pdf`, which 751 found at the head of the *we are alone* list
and did not act on. Parallel round, worktree `r756`, branch `round-756`. **The answer is that we
were right and `poppler` is one pixel outside the clause** — and rendering the neighbouring page of
the same clause found a defect of ours that three of this project's own documents said was not
there. ADRs 0674 and 0675.

## What §12.5.4 requires

Two sentences decide it, and neither has a width-1 case in it:

> If present, the border shall be drawn completely inside the annotation rectangle.

> W — number — (Optional) The border width in points. If this value is 0, no border shall be
> drawn. Default value: 1.

The only place the subclause names 1 is Table 168's default `/W`, which says how *wide* a border is
and nothing about where it goes. A stroke straddles its path, so a border whose ink is entirely
inside the rectangle has its path inset by half its width — at every width. This tree does that.

## Who is right, measured rather than argued

751's pixel reading of `bug766086.pdf` reproduces exactly: at 72 dpi on a 200 × 50 page, ours
strokes device columns 5 and 189 and rows 10 and 39, `poppler` strokes 5 and **190** and 10 and
**40**, and `/Rect [5 10 190 40]` makes the second of each pair outside the rectangle. Both renders
were magnified and looked at (trap 1).

**The mechanism is unreadable at one pixel and plain at ten.** A synthetic link, `/Border [0 0 10]`
on `/Rect [20 20 120 80]`: ours covers device 20…119 by 20…79 — the frame ten units wide measured
inward, and not one pixel outside — and `poppler` covers 15…124 by 15…84, five units beyond `/Rect`
on all four sides, which is half the width exactly. At width 1 `poppler` snaps a thin line to the
pixel grid, so which sides show it depends on where the rectangle's edges fall: two of four on
`bug766086.pdf`, whose edges are integers, and **none** on `issue12750.pdf`, whose `/Rect` is
`[178.019 654.247 265.051 668.194]` and whose border lands on the same columns as ours.

## The population, which is what a finding about a reference needs

`crates/pdf-model/examples/border_overhang_census.rs` is new: both renders of every page whose
annotation states a border this tree strokes and no `/AP`, `/Rect` mapped into each raster through
the crop box and that raster's own size, asking how far outside the rectangle ink of the border's
*stated* colour reaches. Its counts are its own to print; the shape is that over `doc/pdf.js` and
`doc/corpora` together, on the comparisons whose border is in a colour of the page's own, `poppler`
reaches further outside `/Rect` than this tree on three quarters of them and **this tree reaches
further on none**.

Three properties of the instrument were paid for rather than designed:

- **Equality, not nearness.** The first version asked whether a pixel was *nearer* the border's
  colour than the paper. On `issue17056.pdf` — 31 links whose `/C` is `[0 0 0.5]` over a page of
  black text — it called every black glyph near a rectangle that annotation's border and reported
  **this tree** two pixels outside `/Rect` on all 31 (trap 11).
- **The difference discriminates and the level does not.** Content in the border's own colour raises
  both renders' figures, so only a band one renderer reaches is the border. §8.4.1's black is where
  that bites and the summary splits on it.
- **Its population depends on the machine's load**, uniquely among the censuses here: a reference
  render is a subprocess under a time budget, and a run beside three neighbours reported 72
  unmeasurable where the same tree quiet reported 2.

## The defect the round found by looking at the page next door

`bug1552113.pdf` states `/Border [0 0 112]` on a `/Rect [5 25 155 45]`. `AMBIGUOUS_OVERSIZED_BORDER`
said "[a] border that wide, drawn inside that rectangle, *is* the rectangle. Ours fills it";
`Border::inset`'s comment said the clamp "fills the rectangle solid"; §12.5.4's ledger row said it a
third time. **Ours drew a 38 × 20 block in the middle of a 150 × 20 rectangle**, ink 29.65 where the
region the clause states is 67.21, over a content stream that reads "this text should be visible".

The three synthetic regimes on a 100 × 60 rectangle, before and after:

| `/Border` | the region the clause states | drew | now draws |
|---|---|---|---|
| 10 | the 2800-pixel frame inside `/Rect` | 2800 | 2800 |
| 80 (past the height) | the rectangle, 6000 | 1200 | 6000 |
| 300 (past both) | the rectangle, 6000 | **0** | 6000 |

A border of width *w* is the part of the rectangle within *w* of its boundary; stroking the
rectangle inset by *w*/2 states that exactly while the inset path is a rectangle, and at
`w >= rect_width || w >= rect_height` the path degenerates in one axis and the stroke loses the two
sides that vanished. `Border::fills` states the condition and `Border::draw` fills the rectangle
where it holds; §12.5.6.8's square and circle take it inline, because Table 180's `/BS` is the width
of the annotation's own line and its inscribed ellipse's band closes at the same width. Table 168's
`U` underline needs neither — it spans the full width with butt caps already.

**Obeying the clause moved us away from the references, and the note says so.** Our nearest on that
page goes 1.90 → 5.98 bounds, which puts it second in `rank_the_pages_we_are_alone_on` where it was
on no printed list: `mupdf` and `ghostscript` are nearest by drawing **no link border at all**, so
every unit the clause requires is a unit further from them. Trap 9's shared gap acting on a
numerator, and the verdict does not move.

## Measured

§2's sequence whole and green, with `PDFREF_CACHE` on the shared warm cache at a **100% hit rate —
6707 reference renders from disk, 0 produced** — so no gate figure here measures another program's
speed and the round takes no timing measurement at all. Load ran between 20 and 30 across the round,
which is what three parallel neighbours cost.

**The census is identical to 751's** — 983 agrees, 61 contradicted, 836 ambiguous, 3 our geometry,
2 reference geometry, 42 not comparable, 18 no render — and no page changes verdict. The only per-page
line that moves is `bug1552113.pdf`'s own, which is the page the fix is about.

The two new tests fail against the tree before the change and pass after it, run rather than
assumed (trap 13). The `border_overhang_census` was likewise run against the two known pages and the
three synthetic ones before it was pointed at a corpus.

**`doc/todo/00` step 7's ink sweep, re-run whole over all 835 ambiguous pages** because the round
moves pixels. The negative tail — content this tree is *not* drawing — is unchanged in names and
order: 19 at or past −1, most of them documents the corpus gate already calls incomplete, with
`issue16038.pdf` −5.64, `issue12295.pdf` −2.36 and `issue14297.pdf` −1.14 at the diagnosed end.
**The one page that moves is `bug1552113.pdf`, from about +12.8 to +50.37**, which puts it at the
head of the *positive* tail: ours 67.21 against the lightest reference's 16.85, because it is the
only renderer drawing the region §12.5.4 states in the place §12.5.4 states it.

Sweeps: `--bin unpriced` 93 failing bounds over 61 pages, 93 named, 0 not — unchanged, still
`issue6069.pdf`. `--bin overtaken` **48, unchanged**, after the round's own hit was cleared by
citing ADR 0675 in the note it is about. `--bin pointers` 131 absent and `--bin quotations` 38
diverging, both unchanged. `--bin quoted` 190 figures read and 101 confirmed, unchanged, with no
new hit from either rewritten note.

## Changed

- `crates/pdf-model/src/appearance.rs` — `Border::fills`, `Border::draw`, the three call sites, and
  §12.5.6.8's inline case; `Border::inset`'s comment corrected to what the clamp does.
- `crates/pdf-model/tests/annotations.rs` — two tests.
- `crates/pdf-model/examples/border_overhang_census.rs` — new; `border_precedence_census.rs`'s
  module comment now names it as the census that answers its own placement question.
- `crates/pdf-model/tests/oracle.rs` — `AMBIGUOUS_OVERSIZED_BORDER` and `AMBIGUOUS_LINK_BORDER`.
- `doc/conformance/ledger.toml` — §12.5.4, which stays `partial` on Table 168's `B` and `I`.
- `doc/traps/pixels-and-rasterisers.md` — trap 1 gains the paragraph about a comment that states a
  picture.
- `doc/todo/00-ambiguous-bucket.md` — `bug766086.pdf`'s row taken to its clause.
- ADRs 0674 and 0675.

## Owed

- **`poppler`'s placement is not reported to `poppler`.** This tree records the departure and there
  is no channel here for telling the reference about it; whether that is worth one is a question for
  the project owner rather than for a round.
- **The census cannot see a sub-pixel border at all**, on either side, because its test is a pixel
  covered whole. A magnified run would separate placement from scan conversion at the cost of
  measuring two scan converters, and was not taken.
- **The crawl is unmeasured.** `border_overhang_census` has no `--crawl` scope; a reference render
  per candidate over 65 944 documents is a different kind of run from the two here.
- Unchanged from 751: the 22 dropped pages of the ranking, `AMBIGUOUS_ONE_LADDER`'s hold on
  `issue12337.pdf`, `doc/todo/12`'s 278 pages.
