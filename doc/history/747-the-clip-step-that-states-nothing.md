# 747 — `doc/todo/40`'s exactness question had a price, and it was nearly the whole item

Date: 2026-08-25. General-improvement round; subject chosen rather than assigned.
ADR: [0656](../adr/0656-the-clip-step-that-states-nothing.md). ADR numbers 0657 and 0658 were
allocated to this round and are unused.
Ledger: §10.7.4's row moves — four tests added and the note extended.
Touched: `crates/render-cpu/src/scan.rs`, `crates/render-cpu/src/lib.rs`,
`crates/pdf-model/examples/clip_chain_census.rs`, `doc/conformance/ledger.toml`,
`doc/todo/40-mask-chain-crop.md`, `doc/todo/README.md`, `doc/verify.md`.

## Why this item

The briefing's own evidence was that the highest-yield general rounds are the ones that check a
claim this tree makes about itself, and named the shape: a number this project wrote down and has
not re-run. `doc/todo/40` carried four — a base of 13.83 G instructions from session 493, a
`MaskCache::get` share of 24.3% from session 399, a census of 3551 leaves through 7066 nodes, and a
memory peak of 12.31 MB — on an item that had been open for about three hundred rounds while ADRs
0271, 0328, 0363, 0476, 0482, 0590 and the transfer-function work all moved the code underneath it.

Two of the four had moved. The census's counts had not, and could not: they are a property of the
display list.

## What the round actually turned on

Not the re-derivation. The item's third bullet says the reuse it proposes is "not obviously
pixel-exact" and offers three roads without a number on any of them, so the exactness question had
been an argument for three hundred rounds. Extending `clip_chain_census` to simulate two arms —
reuse restricted to prefixes that share their child's band, which is byte-for-byte safe, against the
whole proposal — priced it: the safe arm is a twentieth of what the departure buys on the page the
item is about, and a fifteenth of it corpus-wide. Half the page's nodes *do* share their parent's
band, so the arm is not short of candidates; the sharing is simply one-to-one, and an intermediate
built to serve one leaf moves work rather than removing it.

That is a negative result and it is the round's main finding: **there is no cheap exact version of
`doc/todo/40`.**

## And then the thing the census found by accident

Asked one further question — how many chain steps state a rectangle that admits *every pixel* of the
band they are converted into — the same page answered **three in four**. Such a step contributes
§10.7.4's whole set, so composing it is the identity, and declining it carries nothing between bands
and therefore has none of the reuse road's problem.

Taken, it is −11.1% of a rasterisation of the corpus's worst page and −31.8% of `MaskCache::get`,
with `raster_digest` byte-identical on all 957 corpus first pages it rasterises.

**The instructive part is the second cut.** The first version asked the condition `mask_fill`
branches on — anti-aliasing kept and the rectangle inside `SUPERSAMPLED_LIMIT` — and found two
thirds of what was there. A probe attributed the missing third exactly: one step per chain, a
page-covering clip stated as a rectangle thousands of pages across, which `tiny-skia`'s fixed point
cannot express. Answering from the rectangle's own containment instead of from what the converter
would have done recovered it, and is the more defensible reading besides — §10.7.4 defines the
region by the set a fill *would* include.

## Two things worth carrying

- **A census that answers the question you asked can still be measuring something else.** This one
  predicted three steps in four and the implementation dropped two, and the gap was a *condition in
  the code* the census had no reason to model. Ten minutes of `eprintln` closed it; an afternoon of
  reasoning would not have.
- **A byte-identical optimisation cannot be calibrated against a document** (trap 13). The
  calibration has to be a planted defect, and the one that discriminates here is a containment test
  written for one axis and copied — which fails two of the three new tests and is exactly the
  mistake the code invites.

## What did not move, and is recorded rather than hidden

Two text pages went +0.23% and +0.12%. `MaskCache::get` on one of them grew by 47 522 instructions
of a 12.5 M delta, so the rest is the binary's layout, which any change in this crate moves. It is
in the ADR because `CLAUDE.md` asks for an optimisation's cost in writing, and the honest cost is
"not measurable in the code that changed".
