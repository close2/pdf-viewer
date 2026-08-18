# ADR 0413 — Two stages of refusal, and two lanes that place a mark differently

Status: accepted, 2026-08-18. Session 578. Splits `REFUSED`/`REFUSED_AT_FOUR` in
`crates/render-quorra/tests/corpus.rs` into `REFUSED_BEFORE_THE_SCENE`, `REFUSED_BY_THE_DEVICE`
and `REFUSED_BY_THE_DEVICE_AT_FOUR`, held together by `refused_pages`. Adds
`crates/render-quorra/examples/lane_diff.rs` and `doc/QUORRA_FEEDBACK.md` §31. Closes items 1 and
4 of `doc/todo/54-what-quorras-answer-asked.md`.

No page moves and no pixel moves: the gate's refusal set and both differing lists are what they
were.

## 1. The refusal lists flattened two stages, and a ratchet could not say which had moved

`REFUSED_AT_FOUR` held three names for reasons that are not the same *kind* of reason.
`issue1905.pdf` is refused **at render time**, inside quorra, because the frame's rasterised
coverage sheet exceeds this adapter's 16 384 × 16 384 texture — a device capability, quorra's to
move, and it has moved twice on this list already. `bug1721218_reduced.pdf` and `issue18032.pdf`
are refused **before the scene is built**, by this tree's own translation, for §11.6.6/§11.7.2's
four-component blending space and §11.4.6's non-isolated knockout group (ADR 0327) — refusals no
upstream release can move, at any scale.

One array for both means a name leaving it is two unrelated events wearing one shape, and the
ratchet's failure message could not distinguish them. Worse, it meant this tree's own two names
were written down **twice**, once per scale — and that second copy went stale exactly the way a
second copy does: `issue18032.pdf` was added to the scale-1 list in the four-hundred-and-ninety-
second session and did not arrive in the 4× list until the five-hundred-and-twelfth, not because
anything changed but because no round in between ran the 4× lane.

**Decision.** Split along the stage, not along the scale:

- `REFUSED_BEFORE_THE_SCENE` — this tree's translation, scale-free, checked at both scales from
  one array. A departure is a construction the oracle states and the translation cannot, or one
  quorra's vocabulary grew a way to express. It is never a statement about the adapter.
- `REFUSED_BY_THE_DEVICE` — the adapter, at the page's own scale, and it is **empty**. An empty
  array is a statement rather than an omission: at the resolution a document opens at, nothing in
  this corpus asks this adapter for more than it has. A name arriving is a page a person could
  open at 100% and not see.
- `REFUSED_BY_THE_DEVICE_AT_FOUR` — the adapter under magnification, one name. A departure is a
  hole that only opens when somebody zooms.

`refused_pages(&device_half)` chains the scale-free half with whichever device half the run is the
measurement for and sorts, the way `differing_pages()` already did, so a run is still compared
against one list of names. **The split is about what a departure means, not about running two
comparisons.**

`REFUSED`'s own shape did not need arguing separately, which the todo asked to check: its two
names were both this tree's, so it was already one stage — it was flat rather than flattened, and
the split gives it the missing half as an empty array with the statement written on it.

## 2. The two coverage lanes' differing sets, diagnosed

The five-hundred-and-seventy-sixth session left a residue: both lanes differ from the CPU oracle on
23 of the 957 comparable first pages at page scale, and the two sets of 23 are not the same set.
`bug1863910.pdf` and `issue16500.pdf` are the gpu lane's alone; `bug1743245.pdf` and
`issue21068.pdf` are the default lane's alone. A count that agrees while its membership does not is
`doc/todo/02` §7's second habit in one sentence.

The gate could not answer it, and the reason is structural rather than an oversight: it renders one
lane per invocation and writes an artefact only where *that* lane differs, so for each of the four
pages there was no picture of the other lane at all. `examples/lane_diff.rs` is the instrument —
one display list, three rasters, all three comparisons printed, all three PNGs written. **The list
is built once and handed to both lanes**, so nothing it shows can be this tree's reading of the
document.

**All four pages are one population**: axis-aligned rules about one device pixel wide. On all four
the *amount* of ink is right — page ink agrees to under 1% — and the *placement* is not.

**The default lane carries a per-command sub-pixel offset the gpu lane does not have.**
`bug1743245.pdf` is graph paper, one `q … cm … S … Q` per rule. The centroid of each rule's
coverage along a raster row: oracle and gpu lane both give 33.000, 49.500, 66.000, 82.500, 99.000,
115.500 — a pitch of 16.500 against the document's own `52.0277778 × 0.317180616 = 16.5013` — and
the default lane gives 33.122, 49.602, 66.083, 82.567, 99.047, 115.531, a pitch of 16.482. The
offset is constant *within* a drawing command and different *between* commands, which is what makes
a regular grid of one-rule commands look like a scale error: `bug1863910.pdf`'s two identical
widget borders take +0.103 and +0.078. The range over everything measured is about ±⅛ of a device
pixel.

**The gpu lane quantises some marks' y coverage where the default lane does not.**
`bug1863910.pdf`'s box rule splits 0.247/0.753 across two raster rows in the oracle, 0.370/0.626 on
the default lane and **0.500/0.500** on the gpu lane; `issue16500.pdf`'s table rule is 0.439/0.439
in the oracle, 0.612/0.235 on the default lane and **0.753/0.000** on the gpu lane — the same mark
14% lighter and entirely inside one row. On the *other* axis of the same corpus the gpu lane is
exact, reproducing `bug1743245.pdf`'s rules to three decimals, so this is not "the sampled lane is
approximate": whatever chooses between the exact and the sampled tile per command decides
differently on those pages.

**So: which lane is right?** On `bug1743245.pdf` and `issue21068.pdf` the gpu lane is, and
demonstrably — it reproduces the oracle's geometry to three decimals on the marks in question, and
that geometry is the document's own arithmetic rather than a renderer's opinion. On
`bug1863910.pdf` and `issue16500.pdf` the default lane is *closer* and neither is right. **Every
one of the four differences is quorra's**, because the display list is the same one, and none is
this tree's to fix. `doc/QUORRA_FEEDBACK.md` §31 carries it upstream with the numbers and two
questions; it asks for nothing that costs a release, because none of the four is a wrong picture.

**Why this was worth a round rather than a note.** Two rasterisers that disagree by an eighth of a
pixel disagree about something, and the *default* lane is the one every page this viewer draws goes
through below ten times magnification. The convergence of the two counts is what made it look like
nothing; the four names are what it actually is.

## 3. Why `lane_diff.rs` is committed rather than described

`doc/todo/01`'s standing lesson: a sweep that lives as a paragraph is a sweep that gets rebuilt
from its own description twenty-four rounds later. This is the same shape one instrument over — the
next round that meets a page differing on one lane needs exactly this comparison, and reconstructing
it costs more than keeping it. It takes document stems as arguments and defaults to the four pages
this round diagnosed, so a later round can point it anywhere without editing it.

## 4. What was rejected

- **Holding the two lanes to each other in the gate.** They deliberately do not draw identical
  pixels (quorra's ADR 0016 states the sampled lane's bound against the exact one), so an equality
  ratchet between them would fail on a stated property. What this round found is *inside* that
  bound; the instrument is a diagnosis, not a gate.
- **Moving the gate's bounds so the four pages agree.** Curve-fitting a tolerance until a corpus
  matches, which `CLAUDE.md` principle 5 forbids outright — and the bounds are `real_pages.rs`'s,
  derived elsewhere.
- **Renaming the constants without splitting them.** The todo asked for the split; a rename would
  have kept one array carrying two meanings and cost the same edits to the documents that name it.

## 5. Consequences

- `crates/render-quorra/tests/corpus.rs` holds three refusal arrays and `refused_pages`; the
  failure message names which half a departure is in.
- `doc/performance.md`'s two `corpus.rs::REFUSED_AT_FOUR` pointers and its `REFUSED` sentence are
  updated in the same commit — found by `doc/todo/01`'s eighth sweep, which is the round that
  breaks a pointer being the cheapest round to fix it.
- `crates/pdf-model/src/image.rs`'s `RASTER_BUDGET` cited a `doc/todo/47` that the same commit
  which wrote the sentence had deleted (ADR 0374's round). It cites ADR 0374 now. Also the eighth
  sweep's.
- `doc/todo/54` is two residues rather than four.
