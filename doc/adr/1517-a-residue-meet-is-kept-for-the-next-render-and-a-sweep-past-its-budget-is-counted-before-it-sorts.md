# 1517 — A residue meet is kept for the next render, and a sweep past its budget is counted before it sorts

Status: accepted. Session 1341. Answers ADR 1503 section 5, which named two levers: the clip's side
of a pixel kept for the frame, and the 14 060 small region fills. The measurement below moves the
first lever to where the repeats are. The second turned out to be mostly eight large fills. Amends
ADR 1467 in one respect: a chain's kept edges are built charged row by row (`RowEdges::of_charged`).
Supersedes nothing.
Context: ISO 32000-2 §8.5.3.3, §10.7.4, §11.4.5; ADRs 1389, 1467, 1471, 1479, 1491, 1503, 1513;
traps 62, 73, 94; `doc/habits/measuring.md` 52, 53, 59, 63; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/encode/meet/kept.rs` (new: `KeptMeets`), `encode/meet.rs`
(`Encoder::meet_residue`, `meet_words`, `chain_number`), `encode/clips.rs` (`residue_content`,
`ChainEdges`, `ResolvedClip::leaf`), `encode.rs`, `device.rs`, `device/render.rs`;
`raster/fill/topology/count.rs` (new: `surely_past`), `raster/fill/topology.rs` (`meeting_pairs`);
`raster/meet.rs` (`RowEdges::bytes` counts its charges).
Tests: `tests/residue_meet_kept_for_the_next_render.rs` (new, two cases on three arms; the first
was watched failing with chains named by outline id); `topology/count.rs` (two cases; watched
failing with half the apart pairs counted); `meet/kept.rs` (three); `tests/residue_regions.rs`
puts one render between its two counted renders.

**`crates/render-cpu/` was not opened.**

## 1. Where the time went (callgrind, one `zoom_frame` pair of `bug1721218_reduced.pdf`, one thread)

The division ADR 1503 left holds on the merged tree, ADR 1513's job-built mark edges included.
The pair is 7.84 G instructions and `encode_scene` 4.17 G. Of that, `meet_residue` is 3.53 G:
the exact meet 2.05 G, the residue tiles 1.10 G (`intersect_links` 0.79 G) and the chains'
row index 0.37 G.

**The repeats are across renders, not inside one.** A zoom frame draws the page's four-component
group as two frames of the same elements, chromatic and black (ADR 1471). Each is a render of its
own, with 6 769 meets asking 10 879 pixels. Of the pair's 45 734 asks, 25 983 repeat a chain and
a pixel. Inside one render only 6 232 do, 1 558 a render, or 14%. All the rest are the black
render asking exactly what the chromatic render asked. A cache kept for one render reaches the
14%. The black render's outlines are uploaded again under new ids: the big clip is outline 17 in
one render and 14 208 in the other.

**The 14 060 fills are two populations.** 14 052 small chains average 22 k instructions each,
0.31 G in all: deposits 13.6 k for about 37 edges, and the plain-topology question 5.4 k. Each is
a first use. The chain is asked once or twice and its region admitted at the first ask, so ADR
1479's row index, built on second use, never exists (trap 73). Eight large fills (two of 111 677
points, two of 9 512, a frame) cost 0.48 G. Of that, 0.29 G was `Topology::of` sorting 111 k
boxes for a sweep that then ran past its budget, so the sort bought nothing.

## 2. Kept: a meet is kept for the next render (`KeptMeets`)

What `meet_residue` writes is a function of the mark's tile (place, extent, bytes), its rule and
polylines, and each residue link's segments under its device transform and rule. The frame's
visible rectangle is added too. The device keeps the met tiles of this render and the one before,
keyed by exactly those words, compared whole. A chain's content is named once a render by a
number. The same content, compared whole, keeps the number it had. New content gets a number
never handed out before. So a meet compares the mark's words and one number, never a hundred
thousand points. Its budget is a sixteenth of the frame budget per render. Past it, a meet is
computed again.

**Byte-identical, and why.** A hit hands back what the miss wrote, for the same input. Two
dependencies of the bytes on frame state were checked. (a) The residue tile is the same byte
whether a region or the tile asks (ADR 1491), so skipping a meet's fill changes only later
admissions. (b) The exact meet's bound is a budget dependence that ADR 1467 states as absent. A
chain whose edges the frame's edge budget declined is remade per meet and held to the meet's own
bound. Past it, the meet keeps `min`. A kept chain is never held to that bound and meets exactly.
So a meet whose chain passes that bound is not kept: kept edges are charged row by row, and
`ChainEdges::unbounded` asks the bound a build over the meet's rows would ask (ADR 1513's
`charge`). The queue drains before the lookup, as `residue_intersection` drains it, so the walk's
order is the same on a hit. The memo is read and written only on the walk's thread, in encounter
order. The value under a key does not depend on that order, so thread count changes nothing.
Measured: every one of the black render's 6 769 meets hits, in both frames.

**A key longer than 1 024 words is not built (`KEY_WORDS`).** Keys are built and hashed on every
meet, but they are read again only when a render repeats. The stroked Type 3 page's renders never
repeat, and its 126 meets a pair have keys of thousands of words. Building them cost 10.8 M
instructions a pair (`meet_residue` 16.8 → 27.6 M). That put its step at 14.14 ms, past
`turn_path`'s 14.10 band (watched). With the cap the cost is 17.6 M and the step is 11.74, inside
the band. On `bug1721218_reduced.pdf`, 99% of keys are under 90 words.

## 3. Kept: a sweep past its budget is counted before it sorts (`surely_past`)

`meeting_pairs` sorts boxes by start and compares each box with the earlier ones that have not
ended. Its count is all `n (n − 1) / 2` pairs, less the pairs where one box ends before the other
starts. Cut the axis into `n` buckets. A box that ends before another starts ends in that box's
starting bucket or an earlier one, so the count of such boxes per start overcounts only by pairs
sharing a bucket. Where all pairs less that count already pass the budget, the sweep would pass
it too. Its answer, `false`, is then given without the sort. That is exact by construction. It is
asked only of 4 096 boxes and more (`COUNT_FIRST`); fewer are swept as before.

## 4. Measured, and not built

| step | `encode_scene` (pair) | `meet_residue` | `Topology::of` |
|---|---:|---:|---:|
| HEAD | 4.17 G | 3.53 G | 0.29 G |
| kept meets | 2.80 G | 1.89 G | 0.29 G |
| and the counted sweep, keys capped | 2.61 G | 1.79 G | 0.09 G |

On the 1.25× step, HEAD reads **193.1 ms** and this change **147.6 ms** on the GPU lane. Both are
pinned minima of 3 × 5 interleaved, at load 2.3–2.5, on exported HEAD and HEAD-plus-change trees.
Each tree had its own target directory, and the binaries are `md5sum`-distinct. Earlier sittings
read 203 → 155 on the CPU lane and 199 → 157 on compute.

- **The clip's side of a pixel kept for one render: not built.** It reaches the 1 558 asks a
  render that repeat. At most the clip's share of their `area_in_pixel`, which is under 4% of the
  encode after section 2.
- **A small region filled by the meet's band machinery: not built.** A region's bytes are ADR
  1491's `f64` accumulator. The band sum is another `f64` sum of the same area, which can differ
  from it at a rounding boundary (habit 62), so it is not byte-identical by construction. Section
  2 already halves the small fills.

## 5. Corpus

`render-raster --test corpus` ran on the CPU, GPU and compute lanes at 1× and 4×. Each run used
the exported HEAD tree and the change tree, each with its own target directory and sandbox
worker. Per-page digests (`PDFVIEWER_RASTER_TIMES`) were compared by name: **0 pages moved on any
of the six arms**. The key cap came later. Its tree was walked again at 1× on all three lanes and
at 4× on the GPU lane, and again 0 moved. Verdicts equal HEAD's. At 1×: CPU and compute
967 / 0 / 0, GPU 966 / 1 / 0. At 4×: CPU and compute 963 / 0 / 3, GPU 962 / 0 / 4. One-versus-many
is 0 on every run.

## 6. What is left

147.6 ms against the CPU backend's 84 ms: within 2× (168 ms). That figure is not re-taken here,
because `crates/render-cpu/` is not this round's to open. In the pair that remains, `meet_residue`
is 1.79 G of 2.61 G of encode: the chromatic render's exact meet 1.03 G and residue tiles 0.63 G.
The rest is outside the meet. The black render still fills every mark the chromatic render filled
(`parallel::rasterise` 0.41 G a pair, half of it the black's). It also rebuilds the group
residue of the big clip (`plan_group_residue` 0.18 G). Neither is a meet. **The next lever is
render-raster's:** draw ADR 1471's two halves as one encode, or name the black half's outlines
by the chromatic half's. After that, the per-render clip side (section 4) and the small fills'
deposits (13.6 k instructions for 37 edges).
