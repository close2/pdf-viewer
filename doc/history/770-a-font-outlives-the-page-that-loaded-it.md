# 770 — A font outlives the page that loaded it

2026-08-28. The question 766 named and declined, assigned to this round.
Decision: [ADR 0710](../adr/0710-a-font-outlives-the-page-that-loaded-it.md).

## What was asked and what was answered

Whether a font cache can outlive an interpretation, against `Document`'s immutability and
principle 3's budgets. Answered yes, in three parts the ADR argues: it lives beside the document
in `viewer_core::Open` and is passed into `pdf_model::interpret_with_fonts` by reference (every
other caller computes through a fresh cache and is unchanged); its key is the font dictionary's
`ObjectId` *bound to the document's own bytes*, held so the allocation cannot be recycled under
the key; and it is bounded by `FONT_BUDGET` — 2 MiB of font program, least-recently-used, both
halves derived with `examples/font_cache_budget`, which prints peak resident memory beside every
row. A failed load is deliberately not kept across pages, because that would change the second
page's reports rather than its cost.

## The census that shaped it

A temporary counter (reverted, nothing of it committed): 62.4% of font loads in the corpus's
multi-page documents re-load a font an earlier page of the same document loaded; ISO 32000-2's
first forty pages load 240 times for 27 distinct fonts. The median corpus document names one
font; nothing repeats across documents, which is why the cache binds to one.

## What moved

The instrument the briefing predicted had to be built: `callgrind_interpret` repeats one page
and so *contains* the repetition a cross-page cache removes. `examples/callgrind_pages` walks
distinct pages, both arms in one binary. One sitting, command totals identical per pair:

| workload | fresh | kept | |
|---|---:|---:|---:|
| ISO 32000-2 pages 1–20 | 574 165 171 | 488 822 640 | **−14.86%** |
| pages 101–150 | 2 120 905 451 | 1 716 780 379 | −19.05% |
| page 101 × 50 (`Open::stale`'s population) | 1 208 416 582 | 829 880 750 | **−31.32%** |
| tracemonkey, 14 pages | 957 904 327 | 846 040 865 | −11.68% |
| `alphatrans.pdf`, 1 page | 6 263 848 | 6 263 863 | +0.0002% |
| `issue6127.pdf`, 51 fonts, no reuse | 105 928 196 | 105 928 417 | +0.0002% |

The keep-nothing arm — the oracle's and the gates' — pays **+0.536%** (`callgrind_interpret`
1 195 249 573 → 1 201 660 678), which is the `Send` conversion `pdf_font::LoadedFont` needed
(`Rc`→`Arc`, cells→locks, priced in isolation at +0.468%) plus the insertions. Peak resident
memory over all 1023 pages: **+2.0 MB** at the shipped budget, which is the budget and nothing
else; +6.3 MB at 4 MiB, which is why 4 MiB was declined — above 2 MiB the uncharged tables
beside the programs overtake the charge.

## The purity claim is calibrated, not asserted

`a_kept_font_changes_what_a_page_costs_and_not_what_it_says` walks one cache across five corpus
documents and compares whole interpretations against fresh-cache runs. Trap 13 both ways: a
planted any-font `get` fails the comparison on page 2; a planted never-rebind `bind` **passes**
it — object numbers happen not to collide across those files — and only the direct `rebound > 0`
assertion catches it. That asymmetry is written into the test's comment: the cross-document
binding is asserted directly because no picture comparison can be trusted to see it.

## Gates

The change is in `pdf-font`, `pdf-model` and `viewer-core`, so §2 ran whole, and this is a fifth
round so §5's binaries were rebuilt and installed. Every line exited 0; the workspace run passed
2688 of 2689 with one failure — `viewer-host`'s
`a_launch_waits_for_page_one_instead_of_polling_for_it`, the same wall-clock assertion 766's
record already named as failing under sibling load and passing alone, which it did here too
(0.016 s, verified before the full re-run). Load average 10–30 from three sibling rounds
throughout; the oracle ran against the shared warm reference cache in 57 s and its ratchets held.

The round also straddled a pause across which `main` advanced (the owner-merged GPU arc,
14eafaaf) and CI on that new `main` is red — this branch stays based on 2f6deace, whose CI was
green, and the integration is the merge round's.

## The ledger

§9.3.1, §9.6.x and §8.4.5 are the rows this touches; all were `implemented` or `partial` and
none's claim moved — the change is exact: same loads, same arguments, kept longer. One stale
phrase in §8.4.5's note ("keyed by `FontKey::Referenced(id)` as well as by name" — the name key
was retired in session 127) was corrected and now names the cache and its ADR.

## For whoever comes next

- `interpret_with`'s subtractive pair now shares one cache per call, so a `DeviceCMYK`-blended
  page loads each font once instead of twice even without a host cache.
- 47 corpus loads are keyless (direct font dictionaries, no reference) and stay uncached —
  `bug946506.pdf` is 19 of them on one page. If that page ever ranks, the key question reopens.
- The `misses` column of `font_cache_budget` is roughly 2× loads (`Interpreter::font` and
  `load_font` both ask); the example's doc says why.
- Session pause note: this round straddled the owner's 2026-08-26 agreements (device-lane
  determinism relaxed; local quorra patching allowed). Neither touches this work — the identity
  claim here is CPU-side interpretation purity, which those agreements keep as the reference.
