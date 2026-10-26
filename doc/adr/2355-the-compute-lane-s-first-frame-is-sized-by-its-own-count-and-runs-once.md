# 2355 — The compute lane's first frame is sized by its own count, and runs once

Status: implemented. Session 1760, 2026-10-26, a "measure" round under the project owner's switch to
code improvements of 2026-10-14. Context: ADR 2339 (the lever, priced and left for a round whose bar
it clears), ADRs 2322 and 2338; raster's ADR 0095 (the one submission, the persistent capacity, and
"the redo is the first frame's ordinary cost"). Code: `raster_gpu::compute::{dispatch_chain,
read_flag_and_total, ComputePersist}`, the overflow loop in `raster_gpu::device::render`. Test:
`compute::refusal_tests::the_first_frame_is_sized_by_its_own_count_and_runs_once`.

## 1. What the first frame did

The edges buffer has a persistent, grow-only capacity, guarded on every emit write, and the
overflow flag is read at the frame's end (raster's ADR 0095). Before any frame there is no capacity,
and the first frame took a guess: 32 edges a tile, at least 65 536. A dense window's count passes it
(ADR 2339: 246 217 counted against 99 200 guessed on ISO 32000-2 page 20 at the first notch), so
that frame overflowed, grew, and ran again whole — upload, passes and the end-wait twice.

## 2. The construction

The lane's first frame — the one with no capacity yet — is two submissions instead of one: the
sheet's copy out, the count and the scan; then a read of the scanned total (the same eight-byte
readback and map the end-wait uses, in `read_flag_and_total`), the capacity grown to it with
ADR 0095's quarter of headroom, the params written again with that capacity, and the emit, the
deposit, the copy back and the end-wait's readback in the second. Every later frame is ADR 0095's
one submission, unchanged; a later frame that outgrows the capacity still grows and runs again,
which is now the only growth road. A total past the frame budget is refused by name before the
emit, where it was refused after one. The forced growth road keeps its seam: under the
`sabotage-capacity` feature the first frame starts at one edge and takes the old road, and
`tests/capacity_growth.rs` still holds its re-run to the steady road's bytes. The frame names the
read's wait in its phases as `compute capacity sized (first use)`.

## 3. Measured

Two `--clock` holds, HEAD against this tree interleaved, three devices an arm, the read-back thread's
sequence (the page at 1.6 on `Cpu`, then three notches 400 ms apart, each the 990 × 1554 window on
`Compute` and the page after), headless on the Radeon 890M through RADV as the instrument's adapter
line printed, no process of this account holding the render node before any arm. The first hold:
loads 2.78 to 6.56 (the 60 s cap expired on its first eight arms), three passes, emit+deposit 3.1 to
4.2 ms on page 20 in both arms — the device's slow speed (trap 262). The second, with ADR 2354's
cue: loads 1.53 to 2.25, two passes on page 20 and one on the worst page, emit+deposit 0.7 to 1.8 ms
in both arms — the fast speed. The window's job, ms:

| page | hold | HEAD window 1 | this tree window 1 | window 2, both |
|---|---|---|---|---|
| ISO 32000-2 p. 20 | slow | 17.3–27.0 | 10.6–13.7 | 8.4–10.9 |
| ISO 32000-2 p. 20 | fast | 8.3–12.8 | 5.2–7.6 | 4.0–5.6 |
| `tmp/Entwurf.pdf` p. 1 | slow | 175.0–243.8 | 141.7–171.8, and 324.2 beside a 1 119 ms page | 76.9–111.7 |
| `tmp/Entwurf.pdf` p. 1 | fast | 124.4–132.1 | 96.5–99.8 | 44.2–46.0 |

On page 20 the first window now sits 1 to 2 ms above the second, where ADR 2339 put the ceiling
with every first-use cost paid; the read itself is 1.09 to 1.38 ms fast and 2.54 to 2.86 slow. On
the worst page the read is 26 ms fast and 63 to 93 slow, because its wait holds the page's first
upload of its 58 010 outlines' segments as well as the count — HEAD's figure holds that upload once
and every other part of the frame twice — so its first window stays about 50 ms above its second.
Every window's digest is HEAD's, frame by frame. The two levers are measured together; HEAD's window
1 names no compile because the frame it reports is the re-run's.

## 4. What is left, priced and not built

The sizing read waits while the host has nothing to do: the instance streams, the atlas flush and
the rest of the upload are staged after it, where the single submission staged them while the
device counted. Moving the read after that staging — the count and the scan submitted, the staging
done, the total read, the emit submitted before the content pass and before the winding lane's
draw onto the same sheet — would hide the staging inside the wait. It is a split of
`dispatch_chain` into two calls with the frame's buffers carried between them, and it is worth at
most the staging's time on the first window; not built here.

## Costs

One more submission and one map on the lane's first frame, which is where they replace a second
run of it. `dispatch_chain` carries the split as a branch after the scan.
