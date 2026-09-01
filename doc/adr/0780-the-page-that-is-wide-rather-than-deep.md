# 0780 — The page that is wide rather than deep

Session 856. Status: **accepted**.

## Context

`poppler-978-0.pdf`, in the issue-tracker corpus, is byte-identical to `PDFBOX-3688-0.pdf` — one
file filed against two readers by two people. It **opens in 1.6 ms** and **interprets in 2.5 s
into 298 379 commands with nothing reported**. What it does not do is finish: its first page
states **73 047 transparency groups**, every one isolated, unclipped and spanning a 1701 × 2409
sheet, so compositing them is some **300 billion blitted pixels** — about **640 s**, measured at
115 groups a second, with nine of ten sampled stacks inside `draw_group` → `PixmapMut::draw_pixmap`.

Every bound this program has passes it, and each for a reason worth naming rather than
apologising for:

| bound | what it counts | why it is silent here |
|---|---|---|
| `MAX_OPERATIONS`, `MAX_TILES`, `MAX_FORM_DEPTH`, the decode deadline | interpretation | 298 379 commands is not many, and interpretation *finishes* |
| `TargetSpec::for_page`'s `max_pixels`, `MAX_EXTENT` | the target, once | the target is an ordinary page |
| `MAX_GROUP_DEPTH` | how deeply groups **nest** | this page is 73 047 groups **wide** and one deep |
| `MASK_BUDGET` | bytes the mask cache holds | the groups' buffers are not masks |

So the shape of the hole is precise: **nothing in this tree bounds the *cost* of compositing, as
against the depth of it.** Session 855 found the page, built and measured the obvious fix —
narrowing a group's blit to `marked_rows` — and reverted it, because every one of the 73 047
groups holds nine images spanning the page's full height, so `marked` comes back as all 2409 rows
4497 times in 4546 sampled. It left the measure and the constant owed, in that order.

## Decision

### 1. The measure is cumulative group-blit pixels, and it is a *demand* read off the list

`pdf_render::group_cost::group_blit_demand(list, target)` walks a display list and sums, for
every `Command::Group` in it, the pixels its blit would cover: the target's width times the rows
its clip chain admits. It counts a group wherever it is — nested inside another, inside §11.7.2's
blending pair's black half, and inside a soft mask's own command list, because a bound a mask can
hide a page behind is not a bound.

A group **count** would have been the wrong measure and the file says why: 73 047 groups a few
rows tall is cheap. Rows come from `DisplayList::clip_bounds`, which is "a bound and never an
underestimate", so the demand is an upper bound on what a backend would actually spend — the safe
direction, and self-consistent because the *constant* is sized by the same function.

**Reading the list rather than watching the spend is the second decision, and it is what makes the
refusal cheap.** A budget counted down inside `draw_group` returns the thread only after it has
been spent; this walk is O(commands), so the witness is refused in the 2.4 s its interpretation
costs and not in eleven minutes.

### 2. The bound is absolute pixels, not a ratio — decided by measurement

A ratio ("a page may repaint its target *N* times") is the scale-free statement and reads better,
and the census prints it. It is not a statement about the resource. Timed at 1:1 with
`examples/render_at`:

| first page | demand | ratio to its own target | drawn in |
|---|---|---|---|
| `6942273.pdf` | 0.33 G | **660 ×** | **0.2 s** |
| `poppler-57-0.pdf` | 2.42 G | **301 ×** | **11.2 s** |
| `4236297.pdf` | 0.39 G | 980 × | 2.1 s |
| `poppler-LINK-250-0.pdf` | 2.33 G | 378 × | 5.1 s |
| `7311598.pdf` | 0.82 G | 395 × | 4.4 s |
| `1530064.pdf` | 23.08 G | 3861 × | 46.5 s |

Wall clock tracks the pixels and not the ratio, and a ratio bound tight enough to refuse the
eleven-second page would refuse the two-tenths-of-a-second one. That is trap 11's own shape — a
refusal firing on a condition that is not the resource's — so the bound holds the product.

The consequence is written down rather than left to be found: a target is a page, a window, or one
tile of one, so this bounds the work of **one draw** rather than of a whole page at every zoom.
For an interactive viewer that is the right unit, because a frame is what a reader waits for.

### 3. The constant is `2^35` pixels, sized the way `MASK_BUDGET` was

`cargo run --release -p pdf-model --example group_blit_census` is the instrument, and it calls the
same function the bound is checked with — a census that measured something adjacent could not size
it. Over **74 832 first pages** (958 curated pdf.js documents with a first page, the crawl's
65 659, the issue-tracker corpus's 8215), at scale 1.0 on a page-sized target, **4764 state a
group at all**:

| demand | first pages |
|---|---|
| no group | 70 068 |
| ≤ 1 Mi | 3692 |
| ≤ 16 Mi | 997 |
| ≤ 256 Mi | 60 |
| ≤ 1 Gi | 9 |
| ≤ 4 Gi | 2 |
| ≤ 32 Gi | 1 |
| over `2^35` | **3** — three on-disk copies of one file |

`MAX_GROUP_BLIT_PIXELS = 1 << 35` (34.36 G) is **1.49 ×** the heaviest first page that is not the
witness, which is the same modest headroom ADR 0010 gave `MASK_BUDGET` — 32 MiB over the 25.5 MB
of banded masks that motivated it. The witness sits **8.7 ×** *above* the bound, so nothing here is
finely balanced. **Refusal rate, conditions named**: one document, under both its names, of 74 832
first pages at scale 1.0 on a page-sized target.

**What the bound is not for.** It is principle 3's explicit resource bound and nothing more: it
stops a page that *cannot finish* from being started. A page right at it still costs about seventy
seconds of drawing — `render-quorra/tests/group_cost.rs` watched four minutes for one under the
test profile before that arm was made cheap — and that is deliberate. Interactivity is
`Interrupt`'s job (ADR 0650), which returns a host's thread in milliseconds per command. Setting
this constant low enough to make every admitted draw feel fast would refuse `1530064.pdf` and four
other real documents that do finish: a page nobody can read, to save a page somebody can already
cancel.

### 4. The refusal is `BackendError::GroupsTooCostly`, and every backend asks

`GroupsTooDeep`'s sibling one axis over, in the crate both other backends read, carrying `demanded`
and `limit` so a host prints the bound by number as well as by name. It is asked at **five** call
sites and not three, because two backends have more than one entry that takes a display list:

| backend | where | why there |
|---|---|---|
| `render-cpu` | `Rasterizer::rasterize`, before the pixmap | one entry, and the allocation must not be paid for a page that is refused |
| `render-gpu` | `scene::build` | reached by `Rasterizer::rasterize` *and* by the public `build_scene`, which is the tier-2 path a window uses — trap 12b's own lesson |
| `render-quorra` | `QuorraRasterizer::render` and `present::FrameSlot::render` | the offscreen lane builds its scene directly and the frame lanes go through the slot; a check in one is a bound the gates hold and the window does not |

The quorra arm is not a hypothetical: the cross-backend test was written first and **quorra drew
the page**, because `rasterize` never reaches `FrameSlot::render`. A bound one backend holds and
another does not is worse than no bound — the same document then draws in one window and hangs in
the next, and no gate that rasterises with the CPU oracle can see it.

### 5. The witness is a permanent regression, and `fixed-documents.toml` gained a third form

`ink = refused: <words>` pins that a page must **not** rasterise and that the backend's own
sentence contains those words. It is the third thing a round can fix about a page, after a report
and a picture: a page that never *finished*. A band would be meaningless (there is no raster) and
an empty `ink` would let the bound stop firing in silence, which is the direction that matters —
the row would go from a third of a second to eleven minutes and read as a slow machine.

The words are carried in the `ink` value rather than in `reports`, which is about what the
*interpretation* said; and they are mandatory, because a row pinning only "it did not draw" would
pass for a target that failed to size, which is trap 27 exactly.

## What the standard says, which is nothing and says so

Annex C is **informative** and is the clause that describes this situation outright (§C.1):

> In general, this PDF standard does not restrict the size or quantity of things described in the
> PDF file format, such as numbers, arrays, images, and so on. However, a particular PDF processor
> running on a particular device and in a particular operating environment will always have
> practical limits. When a PDF processor encounters a PDF construct that exceeds one of these
> internal limits or performs a computation whose intermediate results exceeds a limit, an error
> occurs.

That is what this is — a computation whose intermediate results exceed a limit, and an error.
§C.3 puts memory limits in the same class and declines to characterise them, and §C.2's NOTE adds
that memory limits are often exceeded first. So the standard neither states a number here nor
expects one to be stated, and the constant is a documented choice with its measurement beside it
rather than a reading. §11.4.1's ledger row carries the same paragraph.

## Consequences

- One document of 74 832 first pages stops rendering, and it is the one that never rendered.
- Every other page in all three populations is unaffected: the check is a walk over the commands
  and the gates' verdicts, ratchets and rasters are unchanged.
- `examples/group_blit_census` is the instrument a later round re-runs when a population widens,
  and it is the same function the bound uses, so the two cannot drift.
- What is still owed is the *other* backends' own accounting. This bounds the demand a display
  list states; it does not bound a soft mask's evaluation, a tiling pattern's cell replay, or an
  image reduction — each of which has its own bound or none, and none of which this census
  measured.
