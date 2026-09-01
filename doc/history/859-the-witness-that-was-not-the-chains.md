# 859 — The witness that was not the chain's

2026-09-01. Argued in
[ADR 0783](../adr/0783-a-soft-mask-is-evaluated-where-its-consumers-can-read-it.md).

**The finding**: `MOZILLA-831621-14.pdf`'s 41.5 s was never the clip chain `doc/todo/40` prices —
its chains are 3134 of 3158 depth one. It was 3059 soft masks, each one page-sized shading fill
whose consumers read it through clips admitting ~0.1% of the page, every one rasterised and
luminosity-derived over the whole surface. `build_soft_mask` now evaluates a mask's group only
over the rows its consumers can read (`mask_consumer_reach`), and the page draws in **1.51 s**
(was 41.99 in the same sitting), byte-identically on itself and on the corpus's worst page.

Touched: `crates/render-cpu/src/lib.rs`, `crates/pdf-model/examples/clip_chain_census.rs`,
`doc/conformance/ledger.toml` (§11.6.5.1, §12.10.1), `doc/todo/40-mask-chain-crop.md`,
`doc/performance.md`,
`doc/adr/0783-a-soft-mask-is-evaluated-where-its-consumers-can-read-it.md`.

## The attribution, and the instrument gap it went through

The census's main walk said the chain arithmetic is one or two per cent of the page (63 082
scanned mask rows). What it could not see was the soft-mask command lists — a mask's group is a
display list of its own — so the census was extended to walk them, and its three new lines
(`soft-mask lists`, `soft-mask shading fills`, `soft-mask value-pass rows`) named the cost on
their first run: 3057 mask-list shading fills, 3.13 G path pixels, 100% inside their own
(page-covering) clips, 2 445 602 value-pass rows — the whole surface once per mask. Attribution
was confirmed by removal — the fix took the page from 41.99 s to 1.51 s — and by callgrind on the
before arm, read off an intermediate dump at 665.7 G Ir (the run was stopped once it had answered,
so this is a ranking rather than totals; the pool was not pinned and the machine not quiet, which
instruction ranking tolerates):

```text
34.09%  <render_cpu::CpuRasterizer>::build_soft_mask     (self: the value pass)
14.96%  tiny_skia::pipeline::highp::gradient
11.49%  fmaf      7.66%  roundf                          (the gradient's own evaluation)
 6.23%  highp::mask_u8   5.61%  load_dst   4.50%  store   3.83%  fma
 2.69%  render_cpu::scan::is_a_set
```

`MaskCache::get` and the chain building sit below the listing's 99% cumulative threshold — the
clip chain was never the page.

## What was built

- `render_cpu::mask_consumer_reach` — one walk per rasterisation over main commands, groups,
  shaped pairs and every soft-mask list, unioning per `SoftMaskId` the y-extent of each
  consumer's clip-chain bounds; `MaskCache::soft_mask_rows` narrows `marked_rows`' answer with
  it; `build_soft_mask` draws into a buffer covering only those rows, through the same
  `Surface`/`ToDevice` machinery a strip uses. A mask with any unclipped consumer takes the old
  path unchanged.
- The test `a_mask_narrowed_to_its_consumers_rows_draws_the_unrestricted_page`, which asserts
  the narrowing is in force *and* changes no byte on integer geometry; calibrated by planting the
  `Rows` arm as `one_row` and watching it fail.
- The census extension above, permanent, so the next page like this is diagnosable in one run.

Same-sitting A/B (`open_one`, open + interpret + rasterise): `MOZILLA-892314-0.pdf` 32.32 →
5.32 s (round 857's "a size rather than a structure" is the same structure larger — 36 page-sized
mask shadings on 8646×3544); `6081357.pdf` 1.72 → 1.07 s; `0423548.pdf` 2.54 → 2.27 s;
`bug1721218_reduced.pdf` unchanged (118 → 122 ms, noise).

## The departure, measured

Narrowing the evaluation surface lets a mask-group command's band clamp to a different first row,
which is ADR 0219's `y·sy + ty` binade residue. `raster_digest` over all 974 pdf.js first pages:
**one page moves** — `bug1703683_page2_reduced.pdf`, one pixel, one level of 255, two channels; the
page is held `ambiguous` by name and no verdict or quoted figure moves. The three crawl documents
above move 1, 5 and 25 bytes, all single-level. The same movement class already separates strip
divisions (`plan_strips`: "a handful in a million").

## The chain item itself

Unmoved, and its pricing survives this round intact: the witness that looked like its best case
was mis-attributed, so ADR 0656's/0219's numbers still say the exact arm is worth −1.9% and the
full arm −4.4% on the one page where the chain is the whole cost. `doc/todo/40` now says so, and
its "cheapest thing left" (surface-level droppability) is still open and still the cheapest.

## The second track

`--bin owed`'s oldest row: **§12.10.1**, `partial` since 2026-08-13. Re-read against
`measurement.rs`: `registration`, `matrix_has_priority` and `projected_position` are present and
tested, §12.10.3–.5 are `implemented`, and the one leg owed is §12.10.2's projected-to-geographic
conversion, which waits on the EPSG registry and ISO 19162 rather than on anything in this tree.
The row's note now says it was re-read and holds. §11.6.5.1's note is brought up to the new
construction.

## Gates

Full §2 (pixels round): fmt (both workspaces), clippy with `-D warnings` (both), nextest
workspace, doctests, corpus, oracle, text_extraction, selection_census, accessibility_census,
dates, xmp, jpeg2000, quorra corpus, fixed_documents, conformance — all green after the final
edit; the log tails are in the round's scratch. `doc/todo/00` step 7's ink sweep is answered by
the digest: one ambiguous page's ink moved by one level on one pixel, everything else
byte-identical.

## What the next round should know

- One lost-work near miss worth naming: calibrating the new test with a planted defect and
  `git checkout -- <file>` reverted the whole file, not the plant; the patch habit
  (`git diff > x.patch` *before* any experiment) is what made it a two-minute recovery. Plant
  with a patch too, and un-plant with `git apply -R` of the plant alone.
- `doc/todo/40`'s remaining chain half is now genuinely a refusal-shaped item: the best witness
  it ever had turned out to be another mechanism's, and nothing in the corpus or the crawls
  currently argues for taking the departure the chain wants.
- The quorra backend evaluates the same soft masks its own way; whether its encoder pays the
  same page-sized cost on this witness is unmeasured and belongs to `doc/QUORRA_FEEDBACK.md` if
  someone runs it.
