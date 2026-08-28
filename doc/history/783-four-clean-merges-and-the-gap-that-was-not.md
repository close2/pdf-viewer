# 783 — Four clean merges, and the gap that was not

Merge round: rounds 779–782, all based on `28ed2239`, merged into `main` in round order.
Date: 2026-08-28. **Zero textual conflicts**: every merge was automatic, and every contact
point was then read rather than trusted.

## The merges

`round-779`, `round-780`, `round-781` and `round-782` each merged clean with `--no-ff`. The
only file touched by more than one branch was `doc/conformance/ledger.toml` (779, 781 and
782 — disjoint rows, as each round had predicted). Verified two ways: every line each branch
added to the ledger is present verbatim in the merged file, and the ledger binary — run as
the formatter it is — changed nothing, so the merged ledger was already canonical (222
partial, 17 reported, 69 inapplicable, 8 writer-side, 113 out-of-scope, 0 unreviewed among
cited rows). 781's code diff turned out to live entirely in
`crates/viewer-ui/src/bin/pdf-viewer-confined.rs` — the `viewer_host::password` policy it
shares already existed and was consumed, not moved — so 781 and 782 shared no code file
at all.

**ADR numbers, verified against the briefing and won by the tree**: the briefing said 780
wrote no ADR and asked this round to record the 0717 gap. The tree says otherwise — 780
wrote `doc/adr/0717-the-bound-one-rasteriser-agrees-with-itself-under.md`, its history file
cites it, and the merged `doc/adr/` holds **0716, 0717, 0718, 0719, one per round, no
duplicates anywhere, nothing at 0720 or above**. There is no gap. (780's agent delivered no
final report; its history file `doc/history/780-…md` is complete and was read as the record.)

## The sequence, whole, on a quiet machine (load ~1.1), on merged `main`

| | |
|---|---|
| `fmt --check` | clean |
| `clippy --workspace --all-targets`, `-D warnings` | silent, exit 0 |
| `nextest --workspace` | **2726 run: 2726 passed, 18 skipped** — the batch added nine tests; no failure at all |
| doctests | ok |
| fuzz `check`, `-D warnings` | clean |
| corpus | 974 documents in 2.8 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow |
| oracle | 1945 pages in 52.5 s: **983 agree, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render** — identical to the pre-batch census; ratchets held; 6707 reference renders from cache, 100.0% hit rate |
| text extraction | **98.26%** (10969/11163 words), 486 of 508 documents; all three gates green |
| selection census | clean, 0 panics |
| accessibility census | ratchet holds — 102853 elements reached, 0 disagreeing lines |
| dates, XMP, JPEG 2000 | clean |
| `render-quorra` corpus | 957 pages in 26.8 s: **932 agree, 22 differ, 3 refused, 17 not comparable** |
| `fixed_documents` | **40 checked, 0 absent, 40 rows** |
| `cargo test -p conformance` | 11629 citations, 1092 quotations verbatim, 0 unreviewed |

Two lines carry news. **The launch test passed inside the full parallel run again** — second
merged full-sequence run since 776's structural rewrite, second clean pass; the flake is
staying retired. And **780's new ranking line prints on the merged tree**: under the
differing-fraction bound the oracle now says "32 of those are convicted by `poppler` and
`mupdf` alone, the one voting pair that shares a glyph rasteriser (trap 9; ADR 0717 …)" —
the count the round calibrated by hand, printing unchanged after the merge. The
accessibility census figure did not move under 782's fifth route, which is consistent rather
than suspicious: 782's own note records that no corpus or crawl document exercises an
element whose only content lives in an appearance stream (`mcid_stream_census`), which is
why the route came from the coverage track in the first place.

## §5, into the main tree's `target/`

Rebuilt in one invocation and installed, build directory taken from `cargo metadata` (it
answered `/home/AI/cargo-target/pdf-viewer/release` — the main tree's, this round running in
the main checkout): `pdf-viewer`, `pdf-sandbox-worker`, `pdf-view-worker`, `pdf-viewer-gtk`,
`pdf-viewer-qt`, `pdf-viewer-confined`, `pdf-retrieve`, and `libviewer_ffi.so` as its own
`cargo build -p viewer-ffi`. All eight present under `target/`, dated this round.

## Sweeps, with the deltas accounted

Baseline taken on pre-merge `main` at `28ed2239` before anything merged; the after-run on
the merged tree, after the sequence, never beside it. Every moved figure attributes to the
batch:

- **`entries`**: 176 → 175 reported entries over 50 → 49 rows, named-nowhere 51 → 50 — 782
  retiring its own subject, §14.7.5.2's `/StmOwn`, from the sweep's read-first shape.
- **`owed`**: 222 `partial` rows unchanged, terms 3928 → 3934, named-by-no-source unchanged
  at 182 over 112 rows — batch prose in extended notes, no new debt.
- **`inapplicable`**: 292 → 295 stated terms, all three new ones named with
  non-inapplicable cousins — the documented noise shape, from 779's amended notes.
- **`tables`**: 6805 → 6865 sentences, attributed key citations 2514 → 2539, **all 25 new
  ones agreeing**, absent unchanged at 101 — 779's ledger rows and 782's Table 357 work.
- **`blockers`**: 40 → 41 — the one new blocker is 781's own, holding, at
  `pdf-viewer-confined.rs` ("[§7.6.4.1 is partial] A key press while §7.6.4.1's card has
  the keyboard"), which is the round's deliberate remainder and not decay.
- **`overstated`**: corroborated 61 → 62, contradicted unchanged at 8 — a child of 782's
  now corroborating its parent's term.
- **`overtaken`**: 45 → 47 against 608 → 612 decision records — exactly 780's own
  accounting: ADR 0717 adds passing-mention hits on the pool's standing-witness notes
  (`CONTRADICTED_GLYPH_EDGES` left the list, having cited 0717; `CONTRADICTED_UNEXPLAINED`
  gained it as a prose-page mention), and the one `names a member` hit is the documented
  `pdfbox/unencrypted.pdf` stem collision.
- **`pointers`**: 8712 → 8740 paths, absent unchanged at 98, undefined symbols unchanged
  at 13 — growth all live or unrooted prose from the batch's ADRs and histories.
- **`quotations`**: verbatim 2765 → 2772, **diverging unchanged at 38 and 2**.
- **`counts`**: +48 sentences, every attribution bucket unchanged (435/151/58/226).
- **`unread`, `parts`, `callers`, `capabilities`**: headline-identical — in particular
  `parts` confirms 781's window verb and 782's route added no counted part.
- **`quoted` / `unpriced`** over this run's oracle log: 237 figures read, 123 confirmed
  (the 101 contradicted are the documented correction-narration and finer-written noise);
  **93 failing bounds over 61 pages, 93 named by the note that holds the page, 0 not** —
  the contradicted pool remains fully priced after 780's rewrites, `issue6069.pdf` page 1
  still the one page whose printed line rounds inside every bound.
- **`spec-errata check`** after 779's tenth use: 0 quotations of struck text.

## What 779–782 delivered, as one batch

- **779** — the errata selection rule's tenth use (ADR 0716): ledger rows §9.8.1,
  §12.8.2.2.1, §14.8.5.4.4 and Annex L read against Errata Collection 3; a new DocMDP `/P`
  integer test — the fixture-too-small shape found on a live family rather than the settled
  head; the tenth-use sections in `doc/errata-read.md` and `doc/todo/01`.
- **780** — the contradicted pool's differing-fraction tail measured as one mechanism
  (ADR 0717): 32 pages convicted by the one voting pair that hints glyphs through a shared
  libfreetype, with ghostscript failing the same bound against both members on 32 of 32 as
  the population-wide control. No pixel, no verdict, no bound moved — the gate prints the
  count, trap 9 gains its tenth-bullet paragraph, `doc/todo/12` the per-verdict control.
- **781** — the confined window's first verb (ADR 0718): §7.6.4.1's password prompt in
  `pdf-viewer-confined` as a `PasswordCard` overlay consuming the shared
  `viewer_host::password` policy; the ledger's §7.6.4.1 cardinal three hosts → four.
- **782** — appearance streams as structure content (ADR 0719): Table 357's `/Stm` and
  `/StmOwn` through `pdf-model` and `viewer-core`, and the finding it did not go looking
  for — `Tree::appearance_owners`, the fifth population route, fixing elements pruned when
  their only content lived in an appearance stream.

The batch shape held: the round-780 ADR the briefing believed absent, the ledger's
three-way contact merging clean, and 781's code footprint being one file were each only
checkable from the merged tree.

## Closed

Worktrees r779–r782 and their build directories removed as one act
(`tools/worktree.sh close 779 780 781 782`, bare numbers). `list` shows `main` and `r784` —
a next round already open, **branched from `28ed2239`, the pre-merge base**, so its merge
will land on this round's result and its briefing should say so.

## Owed

- **CI's verdict on this merged `main`** — the failing run is pre-existing on the owner-arc
  commit `48bb1167`; local `main` is ahead and unpushed, the token is read-only, so CI's
  verdict still awaits the owner's next push.
- **The upstream answer to `doc/QUORRA_FEEDBACK.md` §40** (carried since 773) — the
  device-refused `issue1905.pdf` stays drawn by the CPU backend, loudly, until it comes.
- **`doc/todo/15`'s remainder**, unchanged by 781: the warn-before-abort input for the three
  established windows, and the quorra surface behind the confined window; plus ADR 0718 §2's
  priced-and-refused retry-crossing-only-the-password.
- The owner's `git stash drop` of the known-dead entry (`doc/environment.md`).
