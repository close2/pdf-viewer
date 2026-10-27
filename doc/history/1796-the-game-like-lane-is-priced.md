# 1796 — The game-like lane is priced against the compute lane

Session 1796, slot 9 of batch 119, a "pixels" round at the owner's word of 2026-10-26; ADR 2426; ADR 2427 unused; Q662.
No ledger row moved; no lane, gate, pixel or held figure moved. RFC 0013 written, `doc/rfc/README.md` indexed.

**Question.** The owner's: why not send the page to the device as "GPU elements", as games do, and are PDF's paint
instructions "completely incompatible to GPU painting instructions?" Answer (RFC 0013 section 6, ADR 2426): not
incompatible. Fills under both rules, curves, glyphs, strokes outlined once, solid paint, images, shadings and clips map
onto hardware primitives; coverage as area, §10.7.4's no-disappearance for a path, backdrop-reading compositing and
byte-identical output across adapters do not.

**Built.** `crates/render-raster/examples/resident_triangles/` (an example, not a lane): stencil-then-cover fills with
Loop–Blinn quadratics decided per sample, strokes dashed and outlined once by `kurbo`, images as textured quads, one
render bundle, the view a 32-byte uniform; `RESIDENT_ORACLE` compares with `render-cpu` drawing the carried marks under
the corpus gate's three bounds. In `render-raster`, not under `raster/` as the brief said: it reads documents.

**Measured** (Radeon 890M, headless, 800 × 1600, one `--clock --nice 0` hold of 105 s, both passes): a step of ISO p. 101
0.87–1.15 ms at 8× MSAA (0.69–0.86 ms at 4×) against the compute lane's 1.2–2.6 ms; pp. 101–102 as a column 1.27–1.49 ms;
`22060_A1_01_Plans` 1.30–2.38 ms against 5.6–39.7 ms; `issue19802` 0.44–1.42 ms against 1.3–3.3 ms. Loads 2.92 at every
arm but the first (4.18, cap expired); `pp_dpm_sclk` at its middle line, 1 093–1 648 MHz, before and after each arm; the
render node held by no other process of this account. Against the oracle the text pages sit inside the corpus bounds at
every rung, and 11–14 % of the pixels it inks at 1× get no ink; the plan sheet is outside them at 0.5×–4× (minified scans
without averaging). RADV and llvmpipe drew ISO p. 101 385 and 162 pixels apart, each by one sample's 32 levels.

**Premise and hypothesis.** The premise ADRs held as read, but the brief's "ADR 2242's bounds" is not what ADR 2242 holds
(it is a review's ruling); the oracle bounds used are `crates/render-raster/tests/corpus.rs`'s. The hypothesis held on the
dense page in direction (0.37–0.80 of the compute lane's step), not as an order; on the heavy page its premise failed: the
plan sheet is 16 images of 52 254 938 samples and 64 marks, and the compute lane's step there is image samples crossing.

**Instruments.** The session worked in `scratchpad/r1796/`, with an export of HEAD plus the probe of its own; its target
was seeded by reflink and deleted at the end. No Xvfb, no window, no loop.

**Gates.** The hold: exit 0, 16 arms each exit 0. `cargo fmt --check -p render-raster` exit 0 and `RUSTFLAGS="-D warnings"
cargo clippy -p render-raster --all-targets` exit 0, both in the export (a sibling's mid-edit of `pdf-model` broke the
worktree's build, habit 138); `rustfmt --edition 2024 --check` on the four files exit 0. `cargo nextest run -p
render-raster` not run: only an example was added, which no test reads; `tools/state.sh docs` stopped before its
grant, since rustdoc builds no example. `cargo test -p conformance --no-fail-fast` exit 101,
one target red (`records`): one failure was this record's unfinished gate sentence, now written; the other is rounds 1789's and
1790's records past 40 lines, not this round's.
