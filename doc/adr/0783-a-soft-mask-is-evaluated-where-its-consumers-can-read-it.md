# 0783 — A soft mask is evaluated where its consumers can read it

Date: 2026-09-01 (session 859). Status: accepted.

## Context

Round 857 handed `doc/todo/40` — the clip chain as one crop and one intersect — what it called
that item's best witness ever: `corpus-cache/tika-issue-tracker/batch3/MOZILLA/MOZILLA-831621-14.pdf`,
3166 commands referencing 3149 distinct clips, 414 ms to interpret, **41.5 s** to rasterise onto
1280 × 800, nothing reported. Very nearly one clip per command looked like exactly the chain
arithmetic that file prices.

## The attribution: the witness was not the item's

`clip_chain_census` on the page said otherwise before any profile did. The chains are 3134 of
3158 **depth one** — 63 082 scanned mask rows in total, about 80 M pixel-operations, one or two
per cent of the page — so the chain-sharing question this file prices is nearly worthless here
(`exact` −1.9%, `full` −4.4%). What the census could not see was where the cost was, because it
had never walked the **soft-mask command lists**: the page states **3059 soft masks**, and the
census, extended this round to walk them, printed the whole story in three lines:

```text
soft-mask lists: 3057 commands, 3057 shading fills (3057 clipped), 1 clip leaves of their own
soft-mask shading fills: path pixels 3130368000, within their clips 3130368000 (100.0%)
soft-mask value-pass rows: by path bounds 2445602, clip-intersected 2445602 (+0.0%)
```

Each mask is one page-sized shading fill under one shared clip leaf that covers the target — so
ADR 0236's crop does not apply *inside* the mask (a rectangle covering the surface is recorded as
no bound, deliberately), and each of the 3059 masks was rasterised and luminosity-derived over
the whole 1280 × 800 surface: about **3.1 G shaded pixels plus 3.1 G `SoftMask::value`
derivations**, for masks whose *consumers* — the main list's fills — read them through clips
admitting **0.1%** of the page (main-list census line: path pixels 3 220 480 000, within their
clips 4 112 990). Callgrind on the same binary confirms the ranking (this session's history file
holds the table); `MaskCache::get`'s chain arithmetic is single-digit percent.

ADR 0271 and ADR 0328 had already taken the *value pass* and the *storage* to the rows the
group's own marks could reach (`marked_rows`) — but `marked_rows` answers from the **path**
bounds, and this page's mask fills are page-sized paths, so the reach was the whole surface and
honestly so: the buffer really was marked everywhere. What nobody had asked is who can *read* it.

## Decision

**Evaluate a soft mask's group only over the rows the mask's consumers can read.**

Every reader of a stored soft mask goes through `MaskCache::effective`, and each of its arms
reads the mask over the band of the consuming command's clip — or over the whole surface where
the command has none. So `render_cpu::mask_consumer_reach` walks the display list once per
rasterisation — main commands, groups, shaped pairs, and every soft-mask list, since a mask's
group can itself consume masks — and unions, per `SoftMaskId`, the y-extent of each consumer's
clip-chain bounds: the same per-clip path bounds `MaskCache::build` measures, intersected along
the chain, `Everywhere` for an unclipped consumer or an unmeasurable clip, with a walk that
cannot see a subtree (nesting past `MAX_GROUP_DEPTH`) restricting nothing at all.

`build_soft_mask` then draws the group into a buffer covering `marked_rows ∩ reach` instead of
the surface, using the same `Surface`/`ToDevice` machinery a strip uses — the band's first row
composed into the translation once and last — and runs the value pass over exactly those rows.
The stored entry's band shrinks with it; every row outside is §11.6.5.1's one value for the area
the group's marks never reached, `SoftMask::outside`, which `combine` and `expand_soft_mask`
already substitute. A mask with any unclipped consumer — including everything
`expand_soft_mask` serves — reaches the build with no restriction and takes the old path
unchanged, byte for byte. The narrowed build uses a `MaskCache` of its own, because a clip mask
is banded against the surface it was built for; what sharing would have reused is bounded by the
very rows the narrowing keeps.

## What it is worth

`open_one`, open + interpret + rasterise, both arms one sitting, same machine:

| document | before | after | |
|---|---|---|---|
| `MOZILLA-831621-14.pdf` (3059 masks, 1280×800) | 41.99 s | **1.51 s** | 27.8× |
| `MOZILLA-892314-0.pdf` (36 masks, 8646×3544) | 32.32 s | **5.32 s** | 6.1× |
| `6081357.pdf` (912 masks) | 1.72 s | 1.07 s | −38% |
| `0423548.pdf` (89 masks) | 2.54 s | 2.27 s | −11% |
| `bug1721218_reduced.pdf` (corpus's worst page) | 118 ms | 122 ms | unchanged |

`MOZILLA-892314-0.pdf` is the second document round 857 called slow, recorded there as "a size
rather than a structure" — the census now says it is the same structure at a larger size: 36
masks, each one page-sized shading fill, 1.1 G mask pixels.

## The departure, priced

Narrowing the evaluation surface changes the band a mask-group command draws into wherever that
command's clip band would have started above the narrowed rows: `Band::covering` clamps the
band's top to the surface, `ToDevice` composes that top into the translation, and `tiny-skia`
maps a point as `y·sy + ty` — a different whole-row `ty` rounds in another binade. That is ADR
0219's residue exactly, the one this backend declined for the chain item, and the reason it is
acceptable here where it was not there is its measured size on the population the oracle
actually judges:

- **the 974-document pdf.js corpus (`raster_digest`, both arms): one page moves** —
  `bug1703683_page2_reduced.pdf`, one pixel, one level of 255, two channels. The page is held
  `ambiguous` by name (`AMBIGUOUS_PAGE_DRAWN_IN_INK`), its note quotes no figure at a precision
  a 10⁻⁸ mean shift reaches, and no verdict anywhere moves.
- the three crawl documents above move 1, 5 and 25 bytes respectively, every one by a single
  level.
- the witness itself, and the corpus's worst page, are **byte-identical**.

The same class of movement already exists in this tree between strip divisions — `plan_strips`'
own comment: machines with different core counts "draw the same bytes but for a handful in a
million" — so this admits no picture the tree's configurations could not already differ by.

## What was deliberately not done

- **The chain-as-one-crop-and-intersect stays untaken**, at ADR 0656's pricing: on the one page
  where it is the whole cost the exact arm is −1.9% and the full arm −4.4% of 63 082 scanned
  rows, which is not a round's work even before its own departure is argued. The witness that
  looked like its justification turned out to be this ADR's subject instead.
- **An exact variant was constructed and rejected**: skipping mask-group commands outside the
  reach while drawing the rest at their old bands is byte-identical, and worth nothing on the
  witness — its mask fills' bands are the whole surface, so nothing would be skipped. The win
  and the residue are the same rows.
- `doc/todo/40`'s "cheapest thing left" (deciding droppability against the surface) is untouched
  and still open.

## Traps

Trap 1 (looked at the witness's page: the Mozilla bees artwork, drawn correctly, before and
after); trap 13's shape in reverse — the census was extended to see the population it was blind
on, and its new lines named the defect on the first run; trap 10b (`touch` before every arm;
both arms rebuilt in one sitting).
