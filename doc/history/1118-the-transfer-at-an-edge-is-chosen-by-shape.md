# 1118 — §11.7.5.2's transfer function at an antialiased edge is chosen by shape

2026-09-16. Files: `crates/render-cpu/tests/transfer_edge.rs` (new), `doc/adr/1125-…` (new),
`doc/todo/13-the-transfer-function.md`, `doc/conformance/ledger.toml` (§11.7.5.2's note), this file.
Every other path in `git status` is a sibling's. No pixel moved: the round settles a design question
and plants its fixture. §11.7.5.2 stays `partial`.

## The contract, and what it turned out to be

Owned `render-cpu`, `pdf-render`, `raster`; contract §11.7.5.2's last shape at an antialiased edge.
The clause chooses the transfer function at a point by the topmost object with a **nonzero** object
shape there — so a partly covered edge pixel is inside the object and takes its function on the
*composited* colour, applied where §11.7.5.3's NOTE puts it, "only when all colour compositing has
been completed". This tree applies §10.5's transfer *before* compositing, so at an edge it draws
`blend(transfer(object), backdrop)` where the clause asks for `transfer(blend(object, backdrop))`.
ADR 1125 is the reading.

**The census** (measure first, trap 8): `examples/transfer_function_census` over 963 opened corpus documents: **13 state a `/TR` or `/TR2`,
exactly one states a real one** (`issue6931_reduced.pdf`), painting one fully opaque image — no
translucent overlap, no shading. The correct construction has **no oracle witness**.

## Why the channel was not built

`doc/todo/13`'s per-pixel transfer-identity channel needs, per pixel, the topmost opaque object's
function index. Both backends composite through `tiny-skia`, which exposes no per-pixel
topmost-object hook — so the index is a whole second rasterisation pass, in each backend; building it
in `render-cpu` alone would make the oracle and `render-raster` disagree at every transferred edge,
regressing their one shared witness. Principle 1: not started until it can finish across both
backends. Deferred, decision recorded so the next round executes.

## The fixture (trap 13) and its numbers

`render-cpu/tests/transfer_edge.rs`, an opaque grey (0.25) fill with an antialiased edge over white
under an inverting transfer: interior 0.75 = `transfer(0.25)`, the orderings agreeing at full
coverage; the half-covered edge reads **0.875** (what the pipeline draws) against the clause's
**0.375** (`transfer(blend)`), a gap of **0.50** — 127 levels of 255; the no-transfer control is
unchanged. Three tests, all pass.

**Gates**: tier 1 on the three owned crates, `cargo test -p conformance`, and `raster_golden`
(change detector, no rendering code touched). Exit codes in the report.
