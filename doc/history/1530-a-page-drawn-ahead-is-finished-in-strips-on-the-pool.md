# 1530 — A page drawn ahead is finished in strips on the pool

Slot 3 of batch eighty-four, 2026-10-14, a pixels round on the owner's performance target. ADR 1896;
no ADR 1897 and no question. No ledger row moved: the partial-row word yields to the target. The
witness is the BergPDF original, downloaded again after a reboot emptied `/tmp`; its digest
`07eb7445…68e3` holds.

**Premise.** Held, and it hid a regression. Since the divided read, the one thread drawing ahead
drew 47 of the 48 pieces after the read ended. The frame drawn ahead landed at 1.73–1.75 s headless,
against 0.92–1.01 s for a frame drawn after the read. **Hypothesis.** Held in what it named, not in
its blame. What does not divide is the strip count: ten planner strips at 12 threads and at 24 (the
64-row floor under 16), and the slowest strip takes two to three times the quickest. No serial
compose follows the strips; two serial walks of 13–21 ms each come before them. The frame did land
0.10 s after the read, but the join's hand-over slowed the read by 0.4 s (ADR 1896 §2.5). On a quiet
machine the draw whole is 296–324 ms, not 490–525.

**Built.** `Underway::in_strips`: even strips at the pool's width; each batch of waiting parts in
one pass on the pool; the rest drawn there at the finish; a forbidden cut ends it with a sentence.
The page is defined by `CpuRasterizer::with_cuts`, and `Ahead` uses it. A parallel image warm lost
three A/B pairs and was reverted. Before → after, first frame (second hold, the final tree):
headless 1.74–1.75 → **0.90–0.95 s**; `quorra` on Xvfb 1.88–1.99 → **1.07–1.15 s**; on the owner's
display 1.96–2.02 → **1.08–1.17 s**. The look is 0.03–0.12 s later. The draw whole keeps HEAD's
cuts (296–324 ms on 12 threads, 299–319 ms on 24; 15 even strips 271–311 ms). No seam is seen.

**Unfinished.** The planner asked for the pool's width at a 1 000-row target: 24 even strips took
303–304 ms against 384–397 ms (exploration). It moves the processor arms and owes six of them, so it
is the next build. The join's hand-over waits for the pipelined read.

**Files.** `crates/render-cpu/src/{glimpse,lib}.rs`, `crates/render-cpu/tests/drawn_ahead_in_strips.rs`
(new), `crates/viewer-host/src/loading.rs`, `crates/viewer-host/examples/{pooled_draw (new),
drawn_while_read}.rs`, `crates/viewer-host/tests/drawn_ahead_in_strips.rs` (new), ADR 1896,
`doc/state-of-play.md`, `doc/todo/69` §7, and this record.

**Gates.** `rustfmt --check` exit 0 (7 files). `clippy -D warnings -p render-cpu -p viewer-host
--all-targets` exit 0. nextest: render-cpu 204 passed; viewer-host 263 passed and 1 skipped (the new
gate, which is ignored). `cargo test -p conformance` exit 0. Run in an export: `drawn_while_read` exit
0 (910 of 974 identical); `raster_golden` exit 0 (974 held, 0 moved); the new corpus gate exit 0
(610 pages in strips, 281 as one surface, 0 different). Two `--clock` holds, 146 s and 79 s; the
second was taken because the first build carried the reverted warm. No six arms: no list moves, and
no cut of the whole draw. Duration 9 900 s.
