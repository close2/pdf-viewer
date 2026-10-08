# 1769 — `quorra-transform render` states its strip count, and its oracle states the same

Session 1467. Status: **accepted** and **built**. Takes ADR 1758 section 5's last fixture item through
ADR 1742's rule: a pixel comparison states its strip count. Context: ADRs 0218, 0219, 0847, 1742,
1758; RFC 0002 section 9. Code: `crates/pdf-transform/src/bin/quorra-transform.rs` (the render
verb's plan), `src/render.rs` (`RenderPlan::strips`'s documentation), `tests/support/mod.rs`
(`oracle`), `tests/verbs.rs` (the seam's plan in `a_rendered_page_is_the_oracle_backends_raster_byte_for_byte`).

## 1. What moves, measured

`tests/gate.rs` compares page 100 of ISO 32000-2 at 150 dpi (1241 × 1754), written by the program,
with `support::oracle`'s raster of it. Both asked the machine for their strip count. Drawn with the
count stated (`scratchpad/r1467/probe/src/bin/oracle_page.rs`), pixels that differ, worst 1 level
throughout:

| asked | 2 | 4 | 8 | 16 |
|---|---|---|---|---|
| 1 | 277 | 347 | 377 | 397 |
| 2 | | 116 | 154 | 190 |
| 4 | | | 66 | 94 |
| 8 | | | | 58 |

So the brief's 277 to 397 are the figures against one strip. Between the counts a 24-, 4- and 2-CPU
machine asks for, 94 to 190 pixels differ.

The gate held only because the program and its oracle asked one machine. Worse, the program's own
output followed the machine. Page 100 written by the program unpinned, pinned to four CPUs and
pinned to two gave three different files (`afc52765…`, `a53c2f60…`, `78f48f7e…` by `md5sum`).
RFC 0002 section 9's first layer makes the same sources under the same plan the same bytes, with no
flag needed. A render verb whose bytes are a function of `taskset` is not that.

## 2. Decision

- **The command line states `render_cpu::MAX_STRIPS`** in the plan it builds for `render`. Built so,
  the program writes `afc52765…` unpinned, pinned to four and pinned to two.
- **`support::oracle` states the same**, and so does the seam's plan in the test that compares the
  seam, the program and the oracle. The comparison is then of the program's pixels on every machine.
- **`RenderPlan`'s `None` keeps its meaning**, ask the machine, for a caller that compares two renders
  of its own. Each such side asks one machine, which is every other `strips: None` in the crate's
  tests.

**Sixteen rather than one**, for ADR 1742's reason: one strip where a gate holds a digest, sixteen
where it holds a clock. This gate holds both from one run of the program, and the throughput floor
is measured on the same two hundred pages. Sixteen is also what every machine of sixteen CPUs or
more already drew. On this one the program's bytes are those it wrote before (`afc52765…` both
ways), so no figure the gate or a user holds moved. One strip would make the file ADR 0219's
undivided page. It would also make every one-page render serial, and it would change every output
already written here.

## 3. Cost, and what is left

- On a machine of fewer than sixteen CPUs a page is cut into more strips than there are threads. The
  planner's replay bound (`MAX_REPLAY`, a quarter more work at most) caps the cost, and a batch
  already spreads its pages over the pool. This was not measured on such a machine.
- **`pdf-vfs` still follows the machine.** Its confined worker states `RASTERISING_THREADS`, which is
  taken from the machine before the confinement (ADR 0847). So the page images the file-system face
  serves still depend on the CPUs it was given. That crate is not this round's, and the case is named
  here for a round that owns it.

## 4. Calibrated

- `cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture` under the lock as
  a clock run: exit 0, 183.8 pages/s against a floor of 40, page 100 the oracle's raster byte for
  byte, 224 images and 6 attachments listed.
- `verbs.rs`'s two oracle tests pinned to two CPUs and to four: pass.
- **A plant that the verbs tests cannot see.** With the program and the seam put back at `None` and
  the oracle at sixteen, pinned to four CPUs, the two tests still pass. `PDF20_AN001-BPC.pdf` page 1
  at 150 dpi draws the same bytes at four strips and at sixteen, so those tests do not exercise this
  property (trap 13). The gate's page does, by 94 pixels, and the program-bytes comparison of section
  1 is the calibration: three files asked, one file stated.
