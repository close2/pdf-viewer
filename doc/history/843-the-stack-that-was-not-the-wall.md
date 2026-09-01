# 843 — The stack that was not the wall

The round was pointed at `doc/todo/46-the-kernel-floor.md`: build the
flatten-from-quadratics experiment — the last structural idea with the moved-view step's
magnitude — and land it on its numbers or refuse it priced. It was built whole in
`/home/cl/projects/render-lib`, measured on the 890M, and **declined**: quorra ADR 0096
carries the table, this tree's ADR 0770 the decision, and `doc/todo/46` is re-pointed at
the floor the three declined experiments triangulate.

## What was done

- §5 first: the release binaries were rebuilt and installed at `HEAD` before any number.
- **The experiment, whole, in quorra's `compute.rs`**: the resident arena carrying
  `push_cubic`'s quadratic conversion (the triangle lane's own geometry), the kernel
  flattening each quadratic with a closed-form chord count — no per-thread stack where
  the cubic recursion held ~170 scalars — and the Cpu↔Compute identity restated as a
  derived band under their ADR 0082 (`doc/todo/46`'s option (a)); a shared
  `tests/common/band.rs` gated it, byte-for-byte kept on straight geometry, all 632
  quorra tests green on both adapters before measuring.
- **One interleaved A/B sitting** (ADR 0767's own sequence, Entwurf p1, load < 1.1):
  step 62.0–66.7 ms before, 65.9–66.1 with the naive tolerance, 62.5–63.7 with the
  per-cubic tolerance; count 13.2–19.9 → 9.7–9.9; emit+deposit 27.8–29.9 → 35.7 → 29.8;
  first compute frame 341 → ~450. One extra sitting rescoped the coverage timestamp:
  emit ≈ 16.5, deposit ≈ 13.5. **The stack was not the wall** — the passes are bound by
  the serial per-tile arena walk and the edge traffic — so quorra ADR 0091's occupancy
  suspicion is measured and refuted, and the change was reverted whole. The transferable
  lesson (a relative flatness tolerance is the curve's, never a fragment's: ~1.7× the
  edges, +28% of emit+deposit when misread) is priced in ADR 0096.
- **The corpus gate can name the moved-view lane now**: `PDFVIEWER_QUORRA_COVERAGE`
  accepts `compute` (`tests/corpus.rs`), the lane `lane_for` takes for every moved view
  and which no corpus-scale instrument could run — ADR 0767's knob one instrument over.
  Its first survey on this machine: 957 compared, 932 agree, 22 differ, 3 refused; run
  both ways it showed the experiment's corpus divergence unchanged to the page.
  `issue1905.pdf` refusing on this lane for frame bytes (272 MB > 256 MB) is recorded in
  ADR 0770 §4.
- Quorra commit `812dd7a` (ADR 0096, docs only; the code was reverted before commit).
  **Not pushed** — this account has no push rights to `github.com/close2/quorra`; the
  owner's push is owed. The pin stays `5fb011a`, so no shipped pixel path in this tree
  moved.

## Second track

`doc/conformance/ledger.toml` §12.5 (Annotations, the aggregate), from the top of the
blame-ordered `partial` list: every claim checked against the code and kept — `/Annots`
order, the flags-and-`/OC` gate, `appearance.rs`'s complete-or-reported rule — with the
reply-state sentence found exact but narrowed: `markup::group_source` reads Table 172's
`/IRT` and `/RT` since the grouping work, while `/State` and `/StateModel` still have no
reader. The row keeps `partial` and gains its read-and-kept sentence.

## Gates

Nothing this round leaves in the tree can move a pixel (a test knob, docs, a ledger
note; the quorra pin unchanged), so the map's answer was run, on a quiet machine, after
the measurement sitting: fmt, clippy under `-D warnings`, nextest workspace, doctests,
the two `fuzz/` lines, `cargo build --profile gates -p pdf-sandbox --bins`, the
render-quorra corpus gate on its default lane (ratchets on), and
`cargo test -p conformance` (218 green). In render-lib, before the revert decision: fmt,
clippy under `-D warnings`, `cargo nextest run --workspace` (632 green), doctests.
