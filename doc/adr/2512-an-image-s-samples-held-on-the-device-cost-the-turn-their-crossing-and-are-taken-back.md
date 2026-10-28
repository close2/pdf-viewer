# 2512 — An image's samples held on the device cost the turn their crossing and the plan sheet's step its textures, and are taken back

Status: rejected. Session 1839, batch one hundred and twenty-four, a "measure" round; the construction
was built, held byte-equal to the host by its own test, measured in one clock hold against HEAD, and
taken back because it moves the plan sheet's turn and step the wrong way. Context: `doc/questions/Q662`
and `A662` (item 1, "an image's samples held resident on the device as a texture first, on its own,
keeping the oracle's bytes"), RFC 0013 sections 4 and 5, ADRs 0089, 1433, 1493 (the textures a zoom
step fills), 1863 (one clock hold), 0042 (a warm-up says what became of it). Code: none remains; the
tree's image path is HEAD's. I worked in `scratchpad/r1839/`.

## 1. The premise, read against the code

RFC 0013 prices the plan sheet's step (`22060_A1_01_Plans.pdf` p. 1) as "52 million image samples
crossing again on each step". That is not what the code does. raster-gpu already keeps one texture per
image and one per (image, horizontal factor, vertical factor) reduction for the device's life, and
fills each only in the squares a frame samples and no earlier frame wrote (ADR 1493). What a step pays
is the host re-reducing, at the new integer factor, the cells its window needs, and writing them across:
`frame_budget` reads HEAD's turn at 4 088 662 bytes in 81 uploads and its step at 13 534 128 bytes in
none. The lever is therefore the host's reduction and its write, not a re-upload of samples, and its
ceiling is the step's transfer stage: 5.66 to 5.92 ms of a 20.93 to 21.25 ms step in the hold below.

## 2. What was built

- **A second implementation of `area_averaged_cells` and the premultiply after it, in a compute
  shader**, in integers throughout: the band edges ⌊i·samples ÷ cells⌋, the block's Σc·a and Σa, each
  colour rounded as the host rounds it, then (c·a + 127) ÷ 255. A grid equal to the image's own is a
  block of one sample, so one entry point made the unreduced texture's texels too. Its integers hold
  for sides under 65 536 and blocks of at most 65 536 samples (65 536 · 255 · 255 plus the rounding
  term stays under 2³²); anything larger went to the host.
- **The samples as a storage buffer charged to the device's residency budget**
  (`Limits::max_resident_bytes`, beside the retained encodes), copied across the first time a frame
  wanted them, in one band a thread; a charge the budget refused was kept for a public
  `Device::image_source_refused` and that image's texels were made by the host.
- **Each square recorded as a dispatch into a buffer at the copy's row alignment and a buffer-to-texture
  copy**, the cells encoder submitted ahead of the frame.
- **The pipeline compiled only on the warm-up thread**: after the compute lane on a window's device,
  and last in the warm set on a device that did not ask for the lane, so that a caller who waits for
  warmth meets one path from its first frame. A frame never compiled it, and until it existed the host
  made the texels. Its placement inside the warm set is what `encode_threads_nested`'s counters needed:
  with the compile after the set, whether the first frame took the device's path was a race, and
  `bytes_uploaded` differed by one square's parameters between thread counts.
- **Two tests**: the device's texels byte-equal to the host's at six extents under both filters on
  llvmpipe (the host's arm forced by a one-byte budget), and a refused image drawn by the host and said.
  Calibrated: a rounding division planted as a truncation fails both, 162 197 bytes apart.
- **The corpus, against HEAD's six arms**: every frame digest of the tree with the construction is
  HEAD's, by page, in all six arms (968 pages at 1× and 965 at 4× in each, none differing). How many of
  those frames took the device's path rather than the host's was not counted, so this says the
  construction moved no page, not that the shader drew every one.

## 3. The figures

One `--clock --nice 0` hold, 2 049 s, HEAD (b70dcd09) and the round's tree interleaved, each arm started
once the one-minute load fell below 3 or after 60 s; it did not fall below 3 before most arms, and the
load is beside each figure. No process of this account held the render node before any arm (other
accounts' processes are unreadable to it). Arms are compared only on `pp_dpm_sclk`'s middle line.

| arm (plan sheet p. 1, 1 600 × 1 000) | load | sclk | turn ms | step ms | step transfer ms | step bytes |
|---|---|---|---|---|---|---|
| `frame_budget`, HEAD, pass 2 | 6.85 | 633..744 | 75.41 | 21.25 | 5.92 | 13 534 128 |
| `frame_budget`, this tree, pass 2 | 5.00 | 636..817 | 86.83 | 29.65 | 11.22 | 5 317 248 |
| turn gate, HEAD, first | 2.84 | 823..1 076 | 53.09 | 15.97 | — | — |
| turn gate, this tree, second | 9.64 | 843..1 165 | 65.51 OUTSIDE | 20.79 OUTSIDE | — | — |
| turn gate, HEAD, second | 16.32 | 670..1 015 | 54.62 | 16.31 | — | — |

Pass 1 of `frame_budget` and the tree's first turn-gate arm ran at different lines (HEAD at 600 MHz, the
tree at 626..788; the tree's turn gate at 600 MHz, load 9.39 rising to 31) and are not compared; they lean
the same way. The band is 42.6 .. 61.8 ms for the turn and 11.2 .. 18.3 for the step. On the other image
pages of the turn gate's second pair the device's path helps some steps and hurts others:
`images.pdf` steps 6.41 ms against HEAD's 10.42 and 11.04, `issue13931.pdf` 1.98 against 6.14 and 6.67,
`issue12841_reduced.pdf` 13.26 against 3.45 and 4.49; every turn of an image page is slower (plans
+11 to +12 ms, `issue12841_reduced.pdf` +11 to +15, `images.pdf` +7). The launch gate is unmoved: both
arms exit 0 with 0 figures outside, cold bring-up 18.5 ms in each.

**Where the time went**, from the same tree built with a stage probe, in the hold (two arms: one plain
at 600 MHz, load 4.94; one with a wait after the cells' submission on the middle line, 1 212..1 496 MHz,
load 4.31): the samples' crossing is 104 360 320 bytes and costs 16.4 to 33.7 ms on the frame that
first wants them, which on this page is the turn; recording the five squares a frame needs costs 0.17
to 0.41 ms and the submission 0.45 to 0.95 ms; the shader's own device time, read by the wait, is 5.9
to 6.3 ms for the turn's five squares and 2.3 to 3.7 ms for the step's four. The step's image fill as
a whole is 11.9 to 16.1 ms with no crossing in it. An exploratory run after the hold (bare, load 33, not a figure) placed the rest in
creating the four textures the step's new factor needs, 11 604 464 bytes, at 17 to 32 ms, where HEAD's
whole step transfer, which creates the same four, is under 6 ms. The adapter's carve-out is 512 MiB, of
which 414 MiB was in use when it was read; a 104 MB buffer resident beside the textures is consistent
with the driver moving memory to make room, and that mechanism is not proven here.

## 4. Decision

**Taken back.** The construction keeps the bytes and moves the figures the wrong way on the page it was
for: the plan sheet's turn pays the samples' crossing (+11 to +12 ms, out of its band) and its step
pays more for its textures than it saves in reduction (+4.5 to +8.4 ms, out of its band). By the rule
that a change which moves a figure the wrong way is taken back or argued, and with nothing to argue it
by on the page the slot names, the tree's image path stays HEAD's. ADR 1493's cache and the host's
reduction are what a zoom step uses.

## 5. What would make the lever pay, priced

- **The crossing off the turn.** The samples would have to cross where no frame waits for them — on a
  thread of their own after page one, with the host making texels until the copy is resident — or the
  turn gate's plan-sheet line pays 16 to 34 ms for them.
- **The step's textures at HEAD's cost.** Until the step's texture creation is shown to cost what it
  costs without the resident samples, the device's path saves at most the step's transfer (5.7 to 5.9
  ms) less the shader's 2.3 to 3.7 ms, and the textures' extra time is larger than that.
- **What the other pages say.** `images.pdf`'s step fell by 4.0 to 4.6 ms and `issue13931.pdf`'s by 4.2 to 4.7; a lane
  chosen per image by a prediction of the two costs would take those and leave the plan sheet's to the
  host, and that prediction is not yet a measured model.

A rebuild starts from section 2's construction and its two tests, which are the parts that held.
