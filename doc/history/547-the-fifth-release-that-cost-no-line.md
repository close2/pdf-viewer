# 547 — The fifth release that cost no line, and the three open items that are one item

**The release.** quorra `619ef3b4` → `a4380e2c`, thirty-three commits, all from one day. Fifth bump
in a row costing this tree no source — and the first where that is a fact about upstream rather than
about our forward-compatibility: every `pub` line of both crates was diffed across the range and
**not one public item was added, removed, renamed or resignatured**, with `quorra-scene` untouched
entirely. Their encode walk split into eleven private submodules and their retained-frame tests into
five files, and every path `render-quorra` names still resolves.

**Finding 1: one pixel-moving change in thirty-three commits, and the clause was read before it was
taken.** Their ADR 0055 gives a colour ramp's coincident stop offset to the interval that *starts*
there — §7.10.4's half-open subdomains, with two exceptions that point in opposite directions. On
this corpus it moves exactly one judged line, on both magnified lanes, to the digit, toward the
oracle. A change that moves a mark cannot be accepted here because a corpus improved; the derivation
is what made it acceptable.

**Finding 2: ADR 0377's two magnified rows were one page stale, and the page is ours.**
`22060_A1_01_Plans.pdf` left `REFUSED_AT_FOUR` in session 543 when the raster cache took it under
`max_resource_bytes` (ADR 0374). The ratchet moved in that commit; the summary numbers beside it did
not, and travelled through two of our documents into upstream's `doc/PLAN.md`. Upstream caught it
before we did and was right. **A ratchet held to equality and a count written in prose decay at
different rates, and only one of them can go stale silently.**

**Finding 3, and it is the round's: three open items are one item, and it is a *target*.** Upstream's
encode cache needs the page's `Scene` stable across frames; session 548 needs an already-rendered
frame re-presented under a transform without a readback; session 543's `capture_presented` costs
19–36 ms because this crate never holds the swapchain texture. All three are answered by rendering
the page into a texture the presenter owns instead of straight to the surface — two `render` calls
into one target work against `Target::Texture` and only against it, a texture created
`RENDER_ATTACHMENT | TEXTURE_BINDING` passes quorra's `contains` check and may be sampled
afterwards, and `Device::wgpu()` plus `pub use wgpu` remove the seam. **Nothing was built: 548 owns
it.**

**Date.** 2026-08-16.
**ADR.** [0382](../adr/0382-the-fifth-release-that-cost-no-line-and-the-three-open-items-that-are-one-item.md).

## The draft answered, point by point

`/home/cl/projects/render-lib/doc/feedback-answers-draft.md` is upstream answering our §15, §19 and
§22.5 and raising the encode cache. `QUORRA_FEEDBACK.md` §28 is the reply.

| their answer | ours |
|---|---|
| §15 — the coverage lane already bounds a clipped fill by its clip | **confirmed, closed as already handled.** Checked at the revision now pinned rather than the one they wrote against. The correction we owe with it: §15 said we could not see this from here, and `visible_tile`'s doc comment states the claim verbatim |
| §15's cross-reference — the tile is bounded but the bound can still be a page | **taken as ours too.** It is a sharper statement of §15 than §15 was, and it is the same seam as their tiling measurements |
| §19 — what a `Rect` costs against a `Fill` of the same outline | **closed, and their own four lines closed it first.** ADR 0047 wired the analytic lane to the *solid* fill arm at `a85cc47`, inside a release we took two bumps ago — verified in `encode/fill.rs` at `a4380e2c`. Recognition at upload beats recognition per command in our translation |
| §19 — "do nothing on your side yet" | **accepted, with a stronger reason than the recommendation.** The rectangular-fill census is demoted rather than closed |
| §22.5 — document, do not rename | **agreed; we decline the rename they offered.** Their three rules accepted; the third is the one that would have reached us, because the ADRs that moved `layer_textures` are documents we read and neither said the word |
| the encode cache — their pricing, and obstacle (b) corrected | **accepted in full.** We wrote that reuse survives a transform change; it does not, at any price, and their ADR 0045 said so before we asked |
| the encode cache — obstacle (a), "two `render` calls?" | **yes, and no fragment composition is needed.** Our frame is a background rect, the page, then overlays — no group spanning them, no overlay clipped by page geometry, no blend that must see beneath. But it is `Target::Texture`-only, and that is the finding |

Two direct questions of theirs also answered:

- **Does `issue1905.pdf` refuse in the product or only in the gate?** Only in the gate, and the same
  for `bug1703683_page2_reduced.pdf`. Measured with `examples/zoom_ladder.rs` — a 900 × 1100 window,
  the magnification in the transform, which is what this viewer does — and both draw on every rung
  from 100 % to 6400 %, up and back down, on both backends. At 6400 % `issue1905.pdf`'s page would
  be 79 808 × 126 976 whole; the window is 900 × 1100. Honest caveat recorded: above 800 % the
  window holds a blank part of the second page, so that agreement is trivial.
- **Their count correction.** Right, and the cause is finding 2 above.

## The four lanes, and the two magnified ones A/B'd in one working copy

Flipping only `Cargo.lock`, so what moved is attributed rather than inferred.

| | `619ef3b4` | `a4380e2c` |
|---|---|---|
| scale 1, `cpu` | — | 931 / 23 / 2 / 18 |
| scale 1, `gpu` | — | 929 / 25 / 2 / 18 |
| scale 4, `cpu` | 936 / 11 / 4 / 23 | 936 / 11 / 4 / 23 |
| scale 4, `gpu` | 937 / 10 / 4 / 23 | 937 / 10 / 4 / 23 |

One judged line moves, on both magnified lanes, and it is the same line:

```text
- differs: issue10572.pdf: mean 0.1332 worst tile 7.97 at (256, 1792) differing 0.0005 ssim 0.99497
+ differs: issue10572.pdf: mean 0.1036 worst tile 7.97 at (256, 1792) differing 0.0004 ssim 0.99602
```

Every other judged line of both lanes is character-identical across the pins. **Nothing moves at
scale 1, and that is proved rather than counted**: the scale-1 default lane holds both its refusal
list and its differing list to equality and it passed, so no page changed category there.
`issue10572.pdf` is in neither scale-1 lane's judged output at either pin.

## Gates, verbatim

```text
cargo fmt --all --check                                   clean
cargo clippy --workspace --all-targets                    silent of lints
cargo nextest run --workspace                             2025 tests run: 2025 passed, 15 skipped
cargo test --workspace --doc                              ok, 0 failed
corpus    974 documents in 2.8s: 0 unopenable, 8 locked, 2 encrypted beyond us,
          6 pageless, 64 incomplete, 0 slow
oracle    1794 pages in 48.2s (1691 complete, 103 incomplete)
          agrees 906/862   contradicted 67/66   ambiguous 786/753
          our geometry 1/0   reference geometry 2/2   not comparable 13/8   no render 19/0
text      974 documents in 30.9s: 25 skipped, 61 incomplete and not gated;
          overall 99.2% (22836/23015 words), 22 below 90%
          40 documents against PDFBox: 99.8% (14257/14281) both orders, 4 below 90%
          10969/11163 matched words in bounds (98.26%), 486 of 508 documents fully in bounds
dates     1545 strings in 974 documents, 1514 conform to §7.9.4 (97.99%)
xmp       319 documents carry §14.3.2's stream: 318 read, 1 refused, 3191 properties
quorra    scale 1, cpu lane:  956 pages in 31.8s: 931 agree, 23 differ, 2 refused,
                              18 not comparable  (both ratchets held)
quorra    scale 1, gpu lane:  956 pages in 28.7s: 929 agree, 25 differ, 2 refused,
                              18 not comparable  (ratchets off, and the run says so)
quorra    scale 4, cpu lane:  951 pages in 258.3s: 936 agree, 11 differ, 4 refused,
                              23 not comparable  (REFUSED_AT_FOUR held)
quorra    scale 4, gpu lane:  951 pages in 245.4s: 937 agree, 10 differ, 4 refused,
                              23 not comparable  (ratchets off, and the run says so)
conformance  5 tests pass; 8404 citations, 800 quotations all verbatim
```

`jpeg2000` runs inside `nextest` (`pdf-model::jpeg2000`, 14 codestreams) and is not repeated.

**The A/B arms.** Both magnified lanes were run a second time at `619ef3b4` in the same working
copy: 951 pages in 261.8s and 249.5s, on the counts in the table above.

**§5's binaries were rebuilt.** No source changed, but the dependency a person's binary links did,
so `target/`'s copies would have been a measurement of the previous release.

## Not done, and why

- **Nothing was built for session 548.** The reprojecting presenter is that round's, and a round
  that took a release should not also rewrite the present path. What this round owed it is the
  finding — `QUORRA_FEEDBACK.md` §28.6 — and the answer that a readback is not needed at all.
- **The two censuses are still ours**, fifth session standing. The residue one is the one that
  matters; the rectangular-fill one is demoted by upstream's ADR 0047 rather than closed.
- **No ledger row moved.** No clause's implementation on this side changed: the round's only pixel
  movement is inside the dependency, derived from §7.10.4 by upstream and checked here against the
  clause rather than re-implemented.
- **`doc/todo/00` step 7 is not owed.** Nothing in `crates/*/src` changed, and the oracle's own
  per-page output is what the gate above prints.

## Touched

`Cargo.lock` (two hashes), `doc/QUORRA_UPGRADE.md` (the release's section), `doc/QUORRA_FEEDBACK.md`
(§28), `doc/todo/44-a-draft-that-takes-ten-seconds.md` (§3.3, their question back answered),
`doc/adr/0382-*` (new), this file.
