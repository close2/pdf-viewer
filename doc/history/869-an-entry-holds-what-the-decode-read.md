# 869 — An entry holds what the decode read: two merges, and the raster cache's key is the `/ColorSpace` entry

Date: 2026-09-01.
ADR: [0791](../adr/0791-an-entry-holds-what-the-decode-read.md).
Touched: `crates/pdf-model/src/image.rs`, `crates/pdf-model/tests/image_reuse.rs`,
`doc/conformance/ledger.toml`, `doc/todo/17-a-mebibyte-per-image-xobject.md` (deleted),
`doc/todo/README.md`, `doc/todo/03-more-corpora.md`; and the two merge commits before this
round's own.

## The merges

`round-866` (`795224f9`, the memory investigation: `tools/bounded.sh`, ADR 0798, `doc/todo/17`,
the walk agreement) and `round-867` (`a33e0d0f`, RFC 0002's transform seam: `crates/pdf-transform`,
ADR 0800, `doc/todo/57`; round 868 continues on that branch in its own worktree, which was not
touched) were merged into `main` with `--no-ff`, each with a body saying what it brings. **Neither
needed a conflict resolved**: 866 added a `doc/todo/README.md` row at `17` and 867 one at `57`,
and git placed both. The whole `doc/todo/02` §2 sequence then ran on the merged `main`, alone on
the machine, and every line was green — the numbers each gate printed are in this round's log
and not here; the oracle, text, census, quorra and fixed-document lines all held their counts.

## The fix

ADR 0791. `RasterCache` held a clone of the page's resource dictionary in every entry and charged
`RASTER_BUDGET` the samples alone. The decode reads exactly one of §7.8.3 Table 34's entries —
`/ColorSpace`, under which §8.6.5.1's named spaces and §8.6.5.6's `/Default*` both live — so the
decode is now *handed* that entry alone, the entry holds it, and the budget charges its walked
footprint beside the samples.

On the witness, `render_at` page one at scale 1.0, exact by `getrusage`: **10.59 GiB → 0.15 GiB**
resident, 5.4 s → 0.7 s. Four pages byte-identical between the arms (the witness,
`22060_A1_01_Plans.pdf`, `issue16263.pdf`, `issue12213.pdf`); `display_list_digest` over the 975
pdf.js first pages identical. The two new tests fail against the old code. One instrument
mistake on the way, recorded in the ADR: the *before* binary copied away from the
`pdf-sandbox-worker` refused a JPEG 2000 image and read as a difference in the change.

## The re-walk

Documents 341–680 of `batch2/GHOSTSCRIPT` in C-locale sorted order, one at a time under
`tools/bounded.sh --data 2 -- render_at …`: 340 documents, none stopped by the bound — 333 clean
at a worst sampled peak of 0.03 GiB, 7 aborts that are `render_at`'s own `expect`s on 6 locked
and 1 pageless document. Then the same 340 as one survey shard through the wrapper at 24 threads:
**1.55 GiB peak**, where 866 measured 12.58 GiB for the same half of the same slice, with the
survey's own line `0 unopenable, 6 locked, 0 encrypted beyond us, 1 pageless, 24 incomplete,
0 slow` — recorded in `doc/todo/03` §38 beside 866's. The witness's verdict is still `complete`.
The shard was a symlink farm in the scratchpad, deleted afterwards (§30's rule).

## Gates

The full §2 sequence, twice: once on the merged `main` before this round's change, once after
it, each alone on the machine. Every line
green both times, and every count the second run printed equal to the first's, line for line
(the two logs' summary lines were diffed; only durations moved): the oracle's agrees, contradicted,
ambiguous, not-comparable and no-render counts, the text gate's in-bounds fraction, both censuses'
counts, quorra's agree/differ/refused/not-comparable line, the fixed-document rows and the
conformance gate. The two new tests were run against the old code before being believed (trap 13)
and failed there. Round 868 was building in its own directory beside the first run; neither gate
that spawns a reference showed a jump in "not comparable", so no line was re-run.

## For the next round

- The spec half this round did not reach: `CalGray` and ICC-gray blending groups — a 1-D
  `blending::resolve` with the curve out — named by 865 as the next piece of §11.3.4, still
  named in `doc/todo/01`.
- `doc/todo/03` §37's `MAX_FORM_DEPTH` sixteen, and `batch4` once its pieces land.
- `doc/todo/57`: the transform suite's next steps, blocked on the owner's answers to RFC 0002 §13.
