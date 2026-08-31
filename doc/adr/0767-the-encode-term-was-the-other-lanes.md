# 0767 — The encode term that met the revisit condition was the other lane's, and device-resident records stay parked on today's numbers

Status: accepted — an attribution correction and a priced refusal; the one build is an
instrument knob.
Context: `doc/todo/47-the-encode-term.md`'s revisit condition, reported met by ADR 0766's
measurement (a resize step of `tmp/Entwurf.pdf` at 132.5 ms of which 129.0 was quorra's
`encode`); quorra's ADRs 0084 (the stages), 0087 (record replay, built), 0091 (stage B
re-priced), 0092 (two kernel experiments declined), 0095 (stage B built as one submission);
this tree's ADR 0700 and `viewer-ui`'s `surface::lane_for`.
Code: `crates/render-quorra/examples/zoom_frame.rs` (`ZOOM_FRAME_COVERAGE`, and the
`record-replayed` marker on the frame line).

## 1. The number that triggered the revisit was measured on a lane the window leaves

`examples/zoom_frame` constructed its device from `render_quorra::options()`, whose lane is
quorra's default — `Coverage::Cpu`, the atlas-fronted scanline rasteriser. That is the right
default for the tables every earlier ADR took with this example, and it is **not the lane the
shipped window draws a zoom or drag step with**: `surface::lane_for` takes
`Coverage::Compute` for any moved view on a real adapter (ADR 0700; quorra's 0080/0081), and
a resize step under a fit mode is a moved view on every step. So ADR 0766 §3's arm-2 row —
132.5 ms, encode 129.0 — is what `--coverage cpu` would pay, not what the person at the
window pays. `doc/todo/02` §2 already has the general sentence, learned on the glyph
quantum: **a gate that turns a shipped setting off is measuring a configuration nobody
runs.** This instance is softer — the instrument predates the lane routing, and nobody
turned anything off — but the reading is the same, and the fix is the same shape: the knob.

## 2. Both lanes, one sitting, the real adapter

`ZOOM_FRAME_COVERAGE={cpu,compute}` on `tmp/Entwurf.pdf` page 1 (58 010 commands, the worst
page), AMD Radeon 890M (RADV STRIX1), headless, ADR 0766's own sequence
(`ZOOM_FRAME_SEQUENCE=1,1.024,1.048,1.072`, resize-sized steps), minima of 3 rounds,
arms interleaved A B A B, load average 0.6–1.0 throughout:

| lane | step total (ms) | encode | of which recording | residency+records | transfer | count pass (GPU) | emit+deposit (GPU) |
|---|---|---|---|---|---|---|---|
| `Cpu` (instrument's old fixed choice) | **131.9–152.0** (183.8 on the repack step) | 128.9–152.0 | — | — | 0.4–0.9 | — | — |
| `Compute` (what `lane_for` picks here) | **63.0–66.8** | 9.4–10.1, `record-replayed` | ~8 of it (shares run) | 4.0–4.2 | 4.9–5.2 | 13.8–17.5 | 28.2–31.2 |

The Cpu arm's step 2 reproduced ADR 0766's row to the tenth (132.5 total, 129.0 encode), so
the correction is of the reading, not the run. The ISO page's step is 2 ms on either lane —
small pages are cheap both ways, which is why only the worst page can rank the lanes.

## 3. The refusal, re-priced in today's numbers

`doc/todo/47-the-encode-term.md`'s revisit condition — *encode is the largest term of the
step* — is **not met** on the lane the window takes. The step the person waits for is 63–66
ms and decomposes as: kernels 44–46 (count 14–17.5 + emit+deposit 28–31, GPU serial work),
host encode 9.4–10.1 (record replay's seat-and-instance writes, quorra ADR 0087's own
naming), residency+records 4.0–4.2, transfer ~5. Device-resident records — quorra ADR 0084's
stage 4, the walk on the device — could remove at most the three host terms, ~19 ms of a
~65 ms step, best case, while the kernels it does not touch remain 2.3× larger than
everything it removes. Against that bound stand quorra ADR 0091's still-open design debts
(a device-side seat wants a scan-computable layout; a refusal that keeps its name wants the
totals on the host, which is a sync by another door). The ordering their ADR 0091 chose
stays correct: **the kernels first** — `doc/todo/46`'s flatten-from-quadratics idea is the
only remaining item with the step's magnitude (a plausible halving of 44–46 ms), and a
successful kernel round is exactly what would make encode the largest term and the revisit
condition true for real.

So: **nothing is built on the records shape this round**, `doc/todo/47-the-encode-term.md`
keeps its parked status with the condition restated in these numbers, and the todo file now
names the lane so the condition cannot be met again by the other lane's figure.

## 4. What the sitting showed beside the question

- **Record replay earns its keep on this gesture.** Every warm compute step came back
  `record-replayed` at 9.4–10.1 ms where the cold walk's encode is 17.3–18.0 — the discount
  quorra ADR 0087 measured as "no win" against the *old* walk is real against the current
  frame, and the marker on the frame line now says which road a step took.
- **The Cpu arm's fourth step repacked the atlas** (183.8/177.3 ms, `repacked`), and the
  compute-assist reroute then drew its phases — the hybrid catching the fallout of a repack
  mid-gesture. Recorded as observed; `lane_for`'s atlas-revisit reasoning already prices
  repacks as "one cold CPU frame".
- **The compute lane's first frame is 340–350 ms** (conversion, residency, pipeline
  first-use) against the Cpu lane's 193. The window never pays it on this path —
  `lane_for`'s launch rule keeps the Cpu lane until a gesture moves the view — but any
  future change to the launch lane owes this number a look.

## Held by

The knob and the marker in `examples/zoom_frame.rs` (the commands above reproduce the
table); `doc/todo/47-the-encode-term.md` restated; `doc/todo/47-the-resize-frames.md` §3
corrected to name the lane. No shipped code changed, so no pixel gate can move.
