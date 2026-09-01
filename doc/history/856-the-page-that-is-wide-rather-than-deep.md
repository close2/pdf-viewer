# 856 — The page that is wide rather than deep

Session 856, on `main`. ADR 0780.

Round 855 handed over one item: **size the group bound.** `poppler-978-0.pdf`
(= `PDFBOX-3688-0.pdf`) states 73 047 unclipped page-spanning transparency groups, every existing
bound passes it, and this program never finishes its first page. What was owed was the measure,
then the constant, then the refusal — in that order, and with a census under the constant.

## The measure

`pdf_render::group_cost::group_blit_demand(list, target)` sums, for every `Command::Group` in a
display list, the pixels its blit would cover — the target's width times the rows its clip chain
admits. It counts a group wherever it is: nested inside another, inside §11.7.2's blending pair's
black half, inside a `Command::Shaped` half, and inside a soft mask's own commands, because a
bound a mask can hide a page behind is not a bound.

It reads the list and **draws nothing**, which is what lets it be run over a population holding
the witness and what makes the refusal cost a walk rather than the work it refuses. Its rows come
from `DisplayList::clip_bounds`, "a bound and never an underestimate", so the demand is an upper
bound on the spend — the safe direction, and self-consistent because the constant is sized by the
same function.

## The census

`cargo run --release -p pdf-model --example group_blit_census`, at scale 1.0 on a page-sized
target, over the three populations. **74 832 first pages, 4764 of them stating a group at all:**

| demand | first pages |
|---|---|
| no group | 70 068 |
| ≤ 1 Mi | 3692 |
| ≤ 16 Mi | 997 |
| ≤ 256 Mi | 60 |
| ≤ 1 Gi | 9 |
| ≤ 4 Gi | 2 |
| ≤ 32 Gi | 1 |
| over `2^35` | **3 — three on-disk copies of one file** |

Per population: pdf.js 958 first pages, 50 with a group, heaviest 4.5 M pixels; the crawl 65 659
and 4145, heaviest 23.08 G; the issue-tracker corpus 8215 and 569, heaviest 299.3 G. **The tail is
a cliff and not a slope**: the witness is 13× the next document and 19× it in the ratio measure.

## The two things the census settled that the handover could not

**The bound has to be absolute pixels rather than a ratio to the target.** A ratio is the
scale-free statement and reads better, and the census prints it — but wall clock tracks the
product. Timed with `examples/render_at` at 1:1: `6942273.pdf` demands **660 repaints** and draws
in **0.2 s** because its page is small; `poppler-57-0.pdf` demands **301** and takes **11.2 s**
because its page is not. A ratio bound tight enough to refuse the second refuses the first, which
is trap 11's own shape.

**And `1530064.pdf` is real.** 23.08 G pixels, 46.5 s, and it *finishes*. The constant is
`MAX_GROUP_BLIT_PIXELS = 1 << 35` — 34.36 G — which is **1.49×** that page, the same modest
headroom ADR 0010 gave `MASK_BUDGET` over the 25.5 MB that motivated it, and **8.7× below** the
witness. Refusal rate, conditions named: one document, under both its names, of 74 832 first pages
at scale 1.0 on a page-sized target.

What the bound is *not* is a promise about latency: a page right at it still costs about seventy
seconds. That is `Interrupt`'s job (ADR 0650), and setting the constant low enough to make every
admitted draw feel fast would cost five real documents that do finish.

## The refusal, and the backend that did not hold it

`BackendError::GroupsTooCostly { demanded, limit }`, `GroupsTooDeep`'s sibling one axis over, in
the crate both other backends read. It is asked at **five** call sites rather than three:
`render-cpu`'s `rasterize` (before the pixmap), `render-gpu`'s `scene::build` (which the tier-2
`build_scene` a window uses goes through, not only `rasterize` — trap 12b), and quorra's
`QuorraRasterizer::render` **and** `present::FrameSlot::render`.

That last pair is not a precaution. The cross-backend test was written first and **quorra drew the
page**: its offscreen lane builds a scene directly and never reaches the slot the frame lanes go
through. One check would have been a bound the gates hold and the window does not.

## The witness is a permanent regression, and the check gained a third form

`doc/checks/fixed-documents.toml` now takes `ink = refused: <words>`: the page must not rasterise,
and the backend's own sentence must contain those words. It is the third thing a round can fix
about a page after a report and a picture — a page that never *finished* — and an empty `ink`
would have let the bound stop firing in silence, turning the row from a third of a second into
eleven minutes.

**Calibrated both ways before being believed** (trap 13): with the expected words changed the row
fails naming the real sentence, and with the row pointed at a page that draws it fails with *the
page rasterised, where the row pins a refusal*. Restored, and the gate reports 43 checked, 0 absent.

## The second track

`--bin owed`'s reading list holds **§11.4.1**, which is also the clause this round's code quotes,
so the two tracks met. Its note gained the bound and the clause reading behind it, and the row
gained `group_cost.rs` and the cross-backend test.

**The reading is that the standard states nothing here and says so at length.** Annex C is
informative and §C.1 describes this case outright — "[w]hen a PDF processor encounters a PDF
construct that exceeds one of these internal limits or performs a computation whose intermediate
results exceeds a limit, an error occurs" — while §C.3 declines to characterise memory limits and
§C.2's NOTE says they are reached first. So the constant is a documented choice with its
measurement beside it, which is `CLAUDE.md` principle 5's rule for a place the specification
defines nothing, and it is written down as a choice rather than presented as derived.

## Gates

The full §2 sequence, on a quiet machine, all green: fmt (both workspaces), clippy under
`RUSTFLAGS="-D warnings"` (both workspaces), `nextest --workspace`, the doctests, corpus, oracle,
text extraction, the two censuses, dates, xmp, jpeg2000, quorra's corpus, `fixed_documents` and
the conformance gate. §5's binaries were rebuilt and installed.

**Step 7's ink sweep was re-run**, over the 776 ambiguous pages whose rasters are on disk, with a
Pillow implementation of the same formula rather than the file's ImageMagick one — so the
absolute figures are this instrument's and only the ordering transfers. The negative head is the
set `doc/todo/00` already names, `issue12418_reduced.pdf` at the top, and nothing unexplained
appears.

## What the next round should know

- **The bound is on the *demand a display list states*, and that is all.** A soft mask's
  evaluation, a tiling pattern's cell replay and an image reduction each have their own bound or
  none, and this census measured none of them. `doc/todo/10`'s table has the new row beside the
  old ones.
- **`examples/group_blit_census` is the instrument to re-run when a population widens**, and it is
  the same function the bound checks with, so the two cannot drift.
- The issue-tracker corpus still has `batch3`, `batch4` and `batch5` untried and `batch2`
  unmatched against its digest, and 855's six unread documents that `pdfinfo` gets a page count
  from are still unread.
