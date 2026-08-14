# ADR 0354 — The bound a `Vec` doubled past, and the copy a newline bought

Status: accepted, 2026-08-14. Session 519. Carries out a defect found while re-pricing
`doc/todo/10` §5's four roads against today's code. **Decides none of the four roads**: the
choice is still the project owner's, and §5 now carries each road's price in this round's
numbers instead of ADR 0306's. Amends §7.4, §7.4.4.1 and §7.7.3.3's ledger rows.

## The finding, in one sentence

**A bound that caps an allocation is a claim about `Vec`'s growth policy, and `Vec::reserve` is
amortised.** `filter::inflate` computed exactly the right step — "grow by the capacity, or by the
room left under the ceiling, whichever is smaller" — and then handed it to a method documented to
take `max(2 × capacity, len + additional)`. The step was a *floor*, not a ceiling. Everywhere
below the bound the two agree and the buffer doubles as intended; at the bound they disagree by a
factor of two, and the buffer doubled straight past the number it was there to honour.

The comment above the loop said the opposite, and had said it since the loop was written:

> The buffer never grows past it, so a bomb costs the bound rather than whatever it claims to
> inflate to.

## What it cost, measured

`doc/todo/10` §2's two bombs were rebuilt from that file's description for the third time and came
out **389 317 and 1 847 467 bytes, both 1029:1** — the sizes §2 records, to the byte. The witness is
the project owner's `tmp/Entwurf.pdf`, which is not in this repository and is named in no test.

Peak resident is `ru_maxrss` for the child, which is the counter `/usr/bin/time -v` prints and which
this machine has no `/usr/bin/time` to print; `VmPeak` is polled from `/proc` and is the quantity
`RLIMIT_AS` is compared against. Three runs each, on a quiet machine, both binaries built
`--profile gates`.

| `pdf-retrieve page … 0` | before | after |
|---|---|---|
| **Bomb A** 0.39 MB → 400 MB, 200 M `n` | 0.83–0.84 s, **1145 MB**, VmPeak 1158 MB | 0.77–0.79 s, **768 MB**, VmPeak 777 MB |
| **Bomb B** 1.85 MB → 1.9 GB | 2.08–2.18 s, **1811 MB**, VmPeak 1821 MB | 1.16–1.22 s, **1031 MB**, VmPeak 1041 MB |
| **`Entwurf.pdf`** 49.6 MB, one page, 3.19 M operators | 1.04–1.21 s, **429 MB**, VmPeak 531 MB | 0.94–0.99 s, **381 MB**, VmPeak 390 MB |
| the same through `render_at … 1 1.0` | 1.29–2.13 s, **429 MB**, VmPeak 1730–1791 MB | 1.53–2.96 s, **381 MB**, VmPeak 1730–1791 MB |

**The two rasters are byte-identical**, which is the check that matters more than the megabytes:
`render_at` writes the same PNG before and after, so the witness draws exactly the page it drew.
Its wall clock is `rayon`'s and says nothing either way — the peak is the reading, and `render_at`'s
own VmPeak is the raster's rather than the decode's, which is why that row's address space does not
move.

**Bomb B's 1811 MB is arithmetic rather than an accident**, which is what makes it a defect rather
than a surprise: the buffer starts at four times the compressed input, doubles to 945 666 560 bytes,
and the last step — computed as the 128 075 265 bytes of room left under a ceiling of 1 073 741 825
— was granted as `max(2 × 945 666 560, …)` = **1 891 333 120 bytes, 1804 MiB**. The measurement and
the multiplication agree to within the program's own eight megabytes.

**And it cost time in the same proportion**, because `flate2::Decompress::decompress_vec` fills all
the spare capacity it is given before returning: the oversized buffer was an instruction to inflate
1.76 GiB before the loop could notice that 1 GiB had passed. Callgrind, `RAYON_NUM_THREADS=1`:
**22 228 599 946 → 12 620 142 664 instructions, −43.2%**, matching the wall clock.

## Three changes, and the second one is where the memory actually was

### 1. `reserve_exact`, in `filter::inflate_buffer`

One word. `len == capacity` at that line, so the step *is* the capacity and the growth below the
bound is unchanged — the doubling is preserved, not traded away. At the bound the buffer now stops
at the bound.

**`inflate` was split into `inflate` and `inflate_buffer` so that a test can read
`Vec::capacity`**, and that is not a shape preference. Nothing observable outside the function
changes when the buffer doubles past the ceiling: the refusal is the same `FilterRefusal::TooLarge`,
the bytes are the same absent bytes. `tests/stream_length_bound.rs` checks the *report*, which was
right the whole time. **The allocation is the thing the bound is about, and `capacity()` is the only
instrument that sees it** — which is why the defect survived the round that wrote the loop (508) and
the round that measured its output (471). `an_inflate_never_buys_a_buffer_past_the_bound` was
confirmed to fail with `reserve` put back: *a bound of 65536 bought a buffer of 130784*.

### 2. The buffer's slack is resident, and `finish` allocates the whole decode beside it

`massif` on Bomb A named the peak exactly — **796 379 136 bytes in `filter::flate` plus
400 000 016 bytes in `Arc<[u8]>::copy_from_slice`, 1 197 169 248 total**, against a measured
`ru_maxrss` of 1145 MiB. A decode of *L* bytes ends in a buffer of up to 2*L*, because a loop that
cannot know where the stream ends must double, and `Arc<[u8]>` is a copy rather than a hand-over.
So the peak was capacity + length, up to **3*L***.

`out.shrink_to_fit()` before the hand-over releases the slack first, turning 2*L* + *L* into
*L* + *L*. It is **not** a second copy for the ordinary stream, which is the measurement that had to
be taken rather than assumed: callgrind over `pdf-syntax`'s `callgrind_open` (ten opens of ISO
32000-2) and `pdf-model`'s `callgrind_interpret` (one page of it) reads **−0.145%** and **−0.116%**,
both slightly *cheaper*, because the allocator shrinks a large mapping in place and the copy that
follows touches fewer pages.

### 3. A newline that bought a copy of the page

`Page::content_with_report` concatenates `/Contents`. `extend_from_slice` asks for exactly the
part's length, so on the single-part page every real document has, the `push(b'\n')` that follows
found the buffer full and doubled it: one reallocation and one copy of the entire content stream,
for one byte. `reserve(data.len() + 1)` before the extend is one allocation of exactly the right
size, and keeps amortised doubling for an array of parts. It moves address space rather than
resident memory — `Entwurf.pdf`'s VmPeak falls 531 → 439 MB with this change alone — and address
space is what the confined worker's `RLIMIT_AS` is compared against.

## What did *not* change, and why that is the point

**No constant moved.** ADR 0306 derived `max_stream_len` = 1 GiB from two sides, and its upper
side was an arithmetic on this very cost:

> Decoding costs about *twice* the decoded length before the bytes are handed over […] A bound of L
> costs about 2L, and 2L has to fit in the 3 GiB the raster leaves: L ≤ 1.5 GiB.

That arithmetic was right and the code did not obey it: a whole decode cost up to 3*L* and a bomb
cost up to 2 × the bound. **This round makes the code cost what the ADR assumed** — 2*L* for a whole
decode, exactly the bound for a bomb — so the gibibyte is unchanged and is now derived from a cost
the tree actually pays. `doc/todo/10` §6's rule that nothing arbitrary may be replaced by something
equally arbitrary is satisfied by moving no number at all.

**Nothing drawn moved**, and the artefact is the proof rather than a summary: `display_list_digest`
over all 974 pdf.js documents is **byte-identical** before and after, and the corpus gate's
incomplete count is 61 both ways, measured both ways.

## Why this is not road D, and what it does to D's price

`doc/todo/10` §5 D — stream the decompression — is the only road that removes the allocation rather
than surviving it, and the choice among the four is the project owner's. This round leaves it
undecided and re-prices it, because **session 508 built half of D without setting out to**:
`flate2::read::ZlibDecoder` and `read_to_end` are gone, and what replaced them is a pump — a
decoder held across iterations, an explicit input cursor, and three named termination conditions.
A window-fed decoder is that loop with a fixed buffer in place of a growing one and a consumer
between the two. §5 carries the four prices in today's numbers.

What this round changes about D's *prize* is worth stating plainly, because it shrank: a bomb now
costs the bound exactly rather than twice it, and the honest witness costs 381 MB rather than
429 MB. D would take the bomb from 1024 MiB to a window and the witness from 381 MB to about
315 MB. That is still the only entry that changes the kind of the quantity — and it is a smaller
number than it was this morning.
