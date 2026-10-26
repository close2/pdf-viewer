# 2364 — A run is tested eight pixels at a time, and the memo holds the converted word

Status: accepted. Session 1765, 2026-10-26, the measure slot of batch one hundred and sixteen.
Context: ISO 32000-2 §8.6.5.5 and §8.9.5.2 (the conversion and the map it reads, neither read
differently); ADR 1469 (the memo, and the run answered before it), ADR 2348 section 1 (which left
the memo at 455 M on `22060_A1_01_Plans.pdf`), ADR 1969 (when a band moves); habits 122, 164 and
188; trap 13; principle 2's rule that an optimisation carries its benchmark and its comment. Moves
no ledger row and no band.

## 1. What the 17 instructions were

Callgrind with `--dump-instr=yes` over `pdf-model`'s `examples/callgrind_pages -- 1 <file> 1
kept-revisit` on HEAD's export, the counts of `convert_three`'s closure read against its
disassembly. The plan's four `ICCBased` photographs and its small images are 26 127 469 pixels and
455.4 M instructions, 17.4 a pixel, in two paths:

- **the run**: 22 228 795 pixels (85 %) equal the one before them, 12 instructions each — load,
  mask, compare, branch, then the alpha's mask, the `or`, the store and the loop's step — 267 M;
- **the memo**: 3 898 674 pixels (15 %) at about 48 each — the key rebuilt from three bytes in
  the order opposite to the word's (13 instructions), the hash and the probe, the three bytes read
  back from a stride-of-three table and written one at a time, and the word rebuilt for the next
  compare — 187 M. 15 177 of them miss and convert, which is the 24 M in `Lut::apply`.

ADR 1469 was right that the hit, not the miss, is the cost, and that a 2²⁴-entry table buys
nothing a memo answering 99.94 % does not. What it left is the *shape* of both paths.

## 2. The decision

- **A run is tested a block of eight pixels at a time.** `PreviousPixel::continues_through` folds
  eight masked compares without an early exit and the compiler makes them two 128-bit compares and
  one `movmskps`; a block wholly inside the run is written with two `pand`/`por`/stores, each pixel
  keeping its alpha byte. A block the run does not cover takes the per-pixel path, which is the
  same three answers as before. Habit 188's reading was made in the disassembly first: the branch
  survives, once a block.
- **The memo holds the word the pixel is written as.** `SampleMemo<u32>`, the converted bytes with
  a zero alpha; the key is the samples' own little-endian word with bit 32 set. A key is any
  injective packing of the tuple, and the value is the conversion of that tuple, so the bytes are
  the same; which tuples share a slot changes — on the plan the conversions' cost is unchanged
  (`Lut::apply` 24.1 → 24.0 M), on `issue19971.pdf` slightly lower (the profile's curves 201.0 → 195.3 M).

Exact by construction, and held: `a_converted_pixel_is_its_own_samples_conversion` now carries
runs of 23 pixels that cover whole blocks and end inside others, and a three-pixel tail; four
planted defects each fail it or `a_band_boundary_changes_no_pixel` (trap 13) — a whole block's
alpha made opaque, a block tested on its first pixel only, the tail left unconverted, and the first
and third samples swapped into the conversion.

## 3. The figures

**Instructions**, one steady interpretation (a run of two less a run of one), every thread:

| page | `convert_three` | the interpretation |
|---|---|---|
| `22060_A1_01_Plans.pdf` | 455.4 → 193.4 M, −58 %; 17.4 → 7.4 a pixel | 2 113.0 → 1 828.7 M |
| `issue19971.pdf`, one 2 500 × 1 364 `ICCBased` photograph | 249.5 → 206.4 M, −17 % | 1 478.0 → 1 405.6 M |

On the plan 3 265 917 blocks are tested at 13 instructions and 2 590 838 of them — 79 % of the
pixels — copied at 9 more; 5 400 632 pixels take the per-pixel path, whose memo hit is now about 26
instructions. The interpretation's other difference is the pool's division of the mask multiply
(`opacity_multiplied_in_place`), whose count moves between runs of one binary — 132.7 and 252.5 M in
HEAD's own two — and is not this change's. The display list's `--digest` is the same on HEAD and the
change on eight image pages (the plan, `issue12841_reduced.pdf`, `issue13931.pdf`, `images.pdf`,
`issue19971.pdf`, `personwithdog.pdf`, `issue14415.pdf`, `issue19802.pdf`).

**The clock**, one `--clock` hold on 2026-10-26, HEAD's export and the change's interleaved
H F F H for the turn gate and H F for the launch gate, every arm started after a wait for the
one-minute load below 3 (60 s cap, which expired before three arms at loads 3.07–3.38); the hold
ran at the wrapper's nice 19. No process of this account held the render node, and the gate's own
device readings were 11–26 % busy, which it excludes where it must. The turn gate's minimum turns,
ms:

| page, row | HEAD | change | change | HEAD |
|---|---|---|---|---|
| `22060_A1_01_Plans.pdf` turn | 63.86 | 55.97 | 56.59 | 55.72 |
| its step | 18.55 | 16.82 | 16.49 | 16.60 |
| `issue12841_reduced.pdf` turn | 37.59 | 32.63 | 32.18 | 32.90 |
| `images.pdf` turn | 36.78 | 35.35 | 35.82 | 34.71 |

HEAD's first arm was the machine's slow moment: every row of it is slower, the steps included,
which interpret nothing, and it judged 12 of 33 figures, four outside. In the other three arms, 33,
27 and 33 judged, the one figure outside is `issue19802.pdf`'s step on every arm alike, 1.88–1.94
ms against 1.10–1.80 — HEAD's as much as the change's. **The plan's turn does not see the cut**:
55.72 against 55.97 and 56.59. The reading this round did not take further is ADR 1469 section
4's: eight frames are decoded at once, one thread a frame, so a conversion's bands wait behind the
other frames' entropy decoding and what the change removes is processor time beside the longest
frame rather than on it. The instructions are the claim this ADR makes, and no band moves (ADR
1969). The launch gate banded 59
figures on each arm, judged 39 on HEAD and 40 on the change, and found five outside on HEAD and six
on the change — sub-millisecond opens, a turn under its floor, the device's bring-up and the script
worker's start, on documents with no `ICCBased` photograph;
`open_anon_mib` is within 0.08 MiB of HEAD's on all six documents and `peak_anon_mib` within 1.0 MiB.

## 4. Priced, not built

- **A conversion skipped where the space is the device's** is already the tree's:
  `convert_channels` returns before `convert_three` for `DeviceGray`, `DeviceRGB` and a mask.
  The plan's space is `ICCBased`, and nothing this tree states makes that conversion the identity on
  every tuple; skipping it would be a guess about a profile.
- **A table the memo already holds**, the exact 2²⁴ entries: the misses it would remove are 24 M
  on the plan, against a table of 64 MB a band.
- **`convert_four`'s run**, the same construction: no gate page reaches it with a run worth
  measuring, so it is not built on the strength of this page (principle 2).
