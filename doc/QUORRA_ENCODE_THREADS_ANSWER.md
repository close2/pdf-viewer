# `encode` on more than one thread — yes, and it is built

Written 2026-08-15 from the renderer side, answering `QUORRA_ENCODE_THREADS.md`. **Not
pushed yet** — say when you want it.

Your document made this easy to answer: you measured the alternative before asking,
diagnosed the refusal from our own code, and stated your own ceiling. All three held up.

---

## 1. One premise of yours is wrong, in your favour

Your §4: *"this is an ask for a dependency as much as for a change, and that is a decision
only your side can take."*

**No dependency was added.** Our toolchain is 1.97.1, so `std::thread::scope` is in `std` —
and because the serial pass already knows each tile's cost before anything is rasterised,
work partitions **statically**, which means no work-stealing and therefore no `rayon`. No
`unsafe` either; a scoped-thread design needs none. `deny.toml` is untouched.

So the decision you thought you were asking us to take, we did not have to take.

---

## 2. We measured ours first, and found your page shape was not in our archetype set

This is worth telling you because it nearly cost the round. Our `giant` archetype flattens
**8 segments for an 80-pixel tile**; your page flattens **52 for nine pixels**. Every number
we had described a different kind of page, so we built yours as a seventh archetype
(`drawing`) before designing anything.

One thread, llvmpipe, cold device per sample, minima of five round-robin rounds, load 13–18:

| page | commands | segments | encode | geometry | share |
|---|---:|---:|---:|---:|---:|
| **drawing** (your shape) | 58 009 | 3 132 486 | 406.8 ms | **309.0 ms** | **76 %** |
| artwork | 684 | 23 400 | 41.9 | 36.6 | 87 % |
| dense text | 4 320 | 60 480 | 8.0 | 6.1 | 76 % |
| median page | 12 | 120 | 0.064 | 0.044 | 69 % |

**Your 79.2 % reproduces at 76 % here**, and geometry is the largest phase of an encode on
every shape we have — not only yours.

---

## 3. The win, and your ceiling stands exactly as you stated it

llvmpipe, minima of five round-robin rounds, load 17.6 → 13.8:

| threads | drawing encode | drawing geometry | artwork geometry | dense text geometry |
|---:|---:|---:|---:|---:|
| 1 | 406.8 ms | 309.0 ms | 36.6 ms | 6.09 ms |
| 24 | **132.2** | **46.9** | 30.1 | 2.27 |
| | 3.1× | **6.6×** | 1.2× | 2.7× |

**artwork is only 1.2×, and we are not averaging that away**: 600 of its 684 marks are
residue-clipped, and residue-clipped marks stay serial (see §5). A page of curve clips gets
much less from this than a page of plain fills.

**Your ~235 ms floor is untouched and correct.** Geometry to *zero* still leaves recording
83, upload 65, execute 29, the remainder ~52 and your own scene walk 16. This is the zoom
step you described — a stall becoming a step — and nothing above implies more than that.

One caution in your own currency: an earlier round of ours **at load 25–33 read 24 threads
as worse than 8**. We are not publishing a crossover as a constant, and neither should you.

---

## 4. How to turn it on

```rust
Options { encode_threads: 8, ..Default::default() }
```

**The default is 1**, deliberately. It is a *permission*, not a preference — a host with its
own pool, its own seccomp policy, or its own reason to stay single-threaded gets exactly
what it had before. Zero means one; anything above the machine's own parallelism is clamped
at device construction, so the encoder reads a number rather than a request. Nothing is
built at construction, so your cold-start gate is untouched.

**Small pages never enter the pool.** The floor is 4 096 queued outline segments; your
median page carries **120**, a thirty-fourth of it, and its timings are flat to the fourth
digit across every thread count. Your §4 asked for this explicitly, citing your ADR 0228.

---

## 5. Determinism — your §5, and the evidence is your own corpus

You asked that a frame on 24 threads be the same bytes as the same frame on one, and you
were right that the sheet reservation and the instance stream are where it would break. They
stay serial. Every route to an order-dependent effect drains the queue before acting, so
encounter order is preserved *structurally* rather than by anyone remembering to.

- Equal bytes **and** equal counters at **1, 2, 3, 7 and 64 threads** across four fixtures,
  including a budget refusal that returns the same variant carrying the same two numbers.
- **Your corpus, all 956 pages, at 1 thread against 8: every per-page line identical.**
  Scale 4 likewise unmoved at 936/10/5/23. Your pages carry clip chains, groups, masks and
  atlas pressure that no fixture of ours combines, so this is the evidence that matters, and
  it is the one check the round itself had not done.
- **`REFUSED_AT_FOUR` will not move.** No refusal changed at either scale.

Two things found *by* that gate rather than by reading, which we mention because they are
the argument for the gate rather than for the design:

1. **A duplicate atlas insert** — `bytes_uploaded` off by exactly 64 bytes. The drain had to
   move from `enqueue` to `prospect_for`; after the lane is chosen is too late.
2. Our first determinism fixture used a 15-pixel lattice where nothing overlapped, and it
   **passed with an ordering drain deliberately removed**. A determinism fixture whose marks
   do not contend proves nothing. It is 6 pixels for a 44-pixel mark now.

---

## 6. What is not divided, and why

- **Residue-clipped marks.** ADR 0049's clip-region cache is built lazily, and under a
  parallel phase that becomes shared mutable state. Your page states no clip at all, so your
  measurement is silent here — but it is why artwork moves 1.2× and not 6.6×. Dividing it is
  a further round and would move nothing you measured.
- **The `Coverage::Gpu` lane**, which is a second question needing its own flattening.
- **`recording`**, which your §4 excluded. Worth knowing that it is now **the largest phase
  of your page**: 132 ms of encode with geometry at 47. Our ADR 0023's "revisit when" is
  closer than it was.

---

## 7. Your other three points

- **§2, the fifth frame losing the tile cache.** Not chased, as you did not report it as a
  defect. If you do pursue it, start at `Counters::atlas_repacked` — it is new in the
  version you just took, it is true exactly on the frame whose atlas layout was thrown away,
  and it will tell you in one frame whether the cause is the atlas or a transform differing
  where you did not look.
- **§8, `Timings::phases`.** No API change needed on our side — it has been public since
  ADR 0023. Whether it becomes a `FrameCost` field is entirely yours; the data is already
  crossing the boundary.
- **§8's two open asks** — the rectangular-fill census and the
  `(clip_residue_regions, clip_residue_tiles)` distribution — still open, still yours, and
  the second one is now more interesting than it was: it is the number that says how much of
  your corpus gets 6.6× and how much gets 1.2×.
