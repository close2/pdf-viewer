# 1504 — The first frame is the read, and the next change draws the page while it is read

Slot 3 of batch eighty, 2026-10-12, a measure round, the step back of the owner's performance target.
ADR 1845; no ADR 1846, no question. No ledger row moved: the partial-row word yields to the target.

**Premise.** Held: rounds 1496–1499's figures are the shape HEAD shows, re-taken on HEAD `849a5cfc`
from an export in its own target directory (`quorra` `08cb8fde…`). **Hypothesis.** Half held. The read
is the largest term of the first frame, 76–78 % on the owner's display; the scene is not the largest
of a zoom step — on the device encode is (500–536 ms against 399–417), and the window draws the step
on the processor, where the whole step is one redraw (0.90–1.24 s).

**Where the seconds go** (owner's display, quiet hold, two runs): read ends 3.61, 2.97 s; first look
4.21, 3.50; first frame **4.78, 3.83 s** (round 1497: 6.73, 5.20); last notch to the real frame **1.50,
0.96 s** (1.43, 2.11). Xvfb: first frame **4.31, 4.22 s**, gesture **1.03, 1.07 s**. A busier hold read
5.49–6.95 s and 1.68–5.64 s. Headless, pinned: read 2.58–3.05 s (slow core 3.77–3.88, unpinned
3.02–3.16); processor fit draw 0.78–0.87 s; device fit frame 1.10 s, zoom steps 1.17–1.20 s, replay
0.18 s; device up 6–36 ms; present under 1 ms. One core: `mutool draw` 7.64 s, `pdftoppm` 14.33, ours 3.77.

**Priced, one measurement each.** A, the page drawn while it is read (`pipeline_price` §6, reader and
drawer at once): first slice on the screen at 80–86 ms where the first look waits 3.0–3.3 s headless,
finished frame 3.91–4.11 → 3.39–3.46 s. B, a line kept on the device with the view a uniform
(`zoom_frame`'s replay): 1 168–1 196 → 178 ms a device step. **Picked: A** — the open is the wait with
nothing on the screen, its pixels are the whole draw's by construction, and slot 1 works the zoom's
processor lever; B is `doc/todo/69` §5. Found: quorra's lane seeds are stale (device 1 788 ns a
command seeded, 250 measured; processor 667 against 178–256), and the device is never measured on a
page they gave the processor.

**Unfinished.** No product code, by contract. The read 2.59–3.05 s against round 1498's 2.153 s: a
third hold interleaved both instruments, minima 6 % apart at load 15–28 — the sitting, not divided
quietly. Hold 1's second owner's-display run got no zoom keys; hold 2 re-took both displays.

**Files.** `crates/render-raster/examples/pipeline_price.rs` (new), ADR 1845, `doc/todo/69`, this.

**Gates.** `rustfmt --check --edition 2024` on the example: exit 0. In the export plus the example:
`RUSTFLAGS="-D warnings" cargo clippy -p render-raster --all-targets` exit 0; `cargo nextest run -p
render-raster` 100 passed, 3 skipped. `cargo test -p conformance`: exit 101 first, `records` naming
this record's placeholder; then exit 0. Two `--clock` holds (`--tree 12 --nice 0`): queued 1 373 s,
held 204 s, exit 0; queued 276 s, held 344 s, exit 0 — the example, `zoom_frame` on both lanes, the
references, Xvfb twice and the owner's display twice in each (trap 142); a third, the two read
instruments interleaved: queued 1 761 s, held 47 s, exit 0. About 6 400 s.
