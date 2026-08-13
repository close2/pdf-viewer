# 462 — A reduced raster recomputed on every redraw

**Finding.** `doc/todo/45` item 2 left one question behind and said what it needed: *a redraw-heavy
session measured with `--trace=frames`, which nothing in this project has taken*. The reason nobody
had taken it is that this file's own instrument is 38 **page turns**, and a page turn draws each
page once — so the reduction that ADR 0228 made 7.7× faster was still being paid once per *frame*
and looked free. Twenty `Down` presses on the owner's scanned document redraw one page twenty times,
and each step cost 12.7 to 16.8 ms of which **8.5 to 9.8 was `Image::area_averaged`** on a display
list of one command. The work is per *source* sample, so it does not shrink with the window, and the
twenty steps recompute the identical 1350×1725 raster from the identical `Arc`. It is kept now:
keyed by the source image's `Arc` identity and the two reduction factors, in the cache
`render-quorra` already had. **Median frame 15.0 → 4.8 ms, uploads 23 → 2**, three runs an arm,
every gate unmoved.

**Date.** 2026-08-13.
**ADR.** [0297](../adr/0297-a-reduced-raster-recomputed-on-every-redraw.md).
**Touched.** `crates/pdf-render/src/paint.rs` (`Reduction`, `Image::reduction`, the shared
`smoothed`, two tests), `crates/pdf-render/src/lib.rs` (the export),
`crates/render-quorra/src/cache.rs` (the key, `drop_unreachable`, two tests),
`crates/render-quorra/src/scene.rs` (`reduced_image`), `doc/conformance/ledger.toml` (§8.9.5.3,
§10.7.4), `doc/performance.md`, `doc/todo/45-where-a-frame-goes.md`, `doc/todo/README.md`,
`doc/adr/0297-*`, this file.

## Which item was taken, and why it rather than the other three

The brief named four priced performance items. Three of them could not be *decided* by a
measurement this round:

- **47 and 41 are the same lever from two ends**, and both are open on an argument rather than on a
  number: what 47's cold search still owes is a page-tree index whose design has to satisfy
  `CLAUDE.md`'s launch rule, and what 41 owes is a byte budget, an eviction policy and a liveness
  invariant. Re-measuring either would have produced a number nobody is waiting for.
- **42's fifth item is explicitly not ours** — quorra's ADR 0031 took 2.4 ms of it, and what is left
  is ~6 ms inside `run_frame` that scales with the target and that a warm-up thread cannot allocate
  before the viewport exists. That is an API question to ask, not a measurement to take.
- **45's leftover named its own witness and named what the witness would settle.** It is a
  redraw-latency item, which `CLAUDE.md` principle 2 ranks first, and one run of `xdotool` answered
  it.

## The three things the attribution said, and the third is why a cache was possible

A scratch build — two `Instant`s round the call in `render-quorra`'s `image()`, not committed —
printed one line per drawn image:

```text
SCRATCH source 2700x3450 ptr 0x7f571e6f0020 averaged Some((1350, 1725)) in 8.969 ms
SCRATCH upload 0.002 ms
trace: frame p1 1cmd presented 15.6 | host 0.0 scene 9.8 device 6.4 settle 0.0 | 1 up, 0 culled
```

1. **The averaging is the whole of `scene`** — 8.5–9.8 ms of an 8.5–9.8 ms translation.
2. **The upload is not the cost.** 0.002 ms to hand it over and a 0.8 ms `transfer` median. Had the
   ratio been the other way the answer would have been quorra's rather than ours.
3. **The `Arc` pointer is stable across redraws and changes on a page turn.** Printing it cost one
   rebuild and it decided the design: a display list is rebuilt per page, not per frame, so there is
   an identity to key on and it is exactly a page's. A cache built without that check would have
   missed on every frame and looked like a cache that does not help.

## What had to be paid for it

**A backend has to be able to name the reduction before paying for it.**
`pdf_render::Image::reduction` answers the factors, the reduced dimensions and the filter in two
vector lengths and two divisions; `area_averaged` is written in terms of it, so every refusal is one
function's. The filter is a *field* rather than something derived from the factors, and that is a
real edge: an axis whose factor clamps to one may be magnified while the other is reduced, so
§8.9.5.3's rule is now one function asked about two grids, one of which no `Image` exists for.

**And the pin had to acquire an exit.** The key is the *source's* address, so a reduced entry pins a
whole scanned page's 37 MB against a device budget that counts only the 9 MB uploaded. What settles
it is a proof rather than a policy, and it applies to all three of the caches: **an entry whose pin
is the only reference left to that allocation can never be looked up again**, because no display
list can hold the address the pin holds. `drop_unreachable` releases those after every frame. So the
change needs no new memory argument at all — the entry's own bytes were already inside a stated
budget and its pin outlives the display list by one frame.

That the `#[expect(dead_code)]` on `Entry::pin` went unfulfilled the moment its count was read is
trap 7 working as advertised: the lint said the field's documentation had stopped being true.

## The gates

`cargo fmt --all --check`; `cargo clippy --workspace --all-targets` (silent — the only output is
`viewer-qt`'s cold-build gcc warnings, which §2 names); `cargo nextest run --workspace` (1647 run,
1647 passed, 11 skipped); `cargo test --workspace --doc`; and under `--profile gates`: `corpus`,
`oracle` (**agrees 905, contradicted 68, ambiguous 786** — unchanged), `text_extraction` (99.2%),
`dates`, `xmp`, `jpeg2000`, `render-quorra`'s `corpus` (**919 agree, 37 differ, 1 refused, 17 not
comparable**), and `cargo test -p conformance`. §5's binaries rebuilt and installed.

**And the 4× coverage lane was run on both arms**, because this change is in the resource cache and
that lane is the instrument that found the eviction problem the cache exists inside (ADR 0156, 533
pages refused at 4×). Before and after: `952 pages compared: 929 agree, 16 differ, 7 refused, 22 not
comparable`, and the same seven refusals by name, character for character.

**`doc/todo/00` step 7's ink sweep is not owed and was not run.** Nothing this round changes what is
drawn: a cache hit serves the raster a miss produced, and the corpus gate at both scales is
identical.

## What the next round should know

- **The other two backends still recompute.** `render-cpu` and `render-gpu` call `area_averaged` per
  draw, and `Image::reduction` is available to both. It is not academic: `viewer-confined`'s
  `pdf-view-worker` rasterises with `render-cpu` and returns pixels, so a **confined** host
  redrawing a scanned page still pays the 9 ms the window no longer does. Neither has a per-frame
  resource cache to hang an entry on, so each needs its own bound and its own liveness rule — and
  the honest next step is a measurement in that host rather than the same change twice.
- **`doc/performance.md` section 3b's "zero resource refusals at 4×" has decayed**, and this round
  measured it on *both* arms so that it is not mistakable for a regression: there is one today,
  `22060_A1_01_Plans.pdf`, `uploading would hold 548104348 resource bytes … over the stated budget of
  536870912`. It predates this change. A count in a document with nothing ratcheting it is exactly
  ADR 0281's argument, and this one has been wrong for some number of rounds nobody can name.
- **The redraw witness is a recipe now and it is cheap** — two `+` and twenty `Down` under `Xvfb`
  with `--trace=frames`, in `doc/todo/45`. A round that changes anything on the frame path should
  run it beside the page-turn one, because they measure different halves and this round is the
  evidence: the item that was invisible to 38 page turns was 57% of a scroll step.
- **A scratch build of two `Instant`s is three minutes and it decided this round twice** — once by
  saying the averaging rather than the upload was the cost, and once by printing a pointer that said
  a cache could hit at all. `doc/habits.md`'s *attribute by removing the suspect* has a cheaper
  sibling: print the thing the design depends on before designing round it.
