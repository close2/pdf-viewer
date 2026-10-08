# 1774 — The file-system face states its strip count, and no pixel path a comparison reads asks the machine

Session 1470. Status: **accepted** and **built**. Takes ADR 1769 section 3's last named case through
ADR 1742's rule: a pixel comparison states its strip count. Context: ADRs 0218, 0219, 0847, 1082,
1554, 1742, 1769. Code: `crates/pdf-vfs/src/serve.rs` (`STRIPS`, `RASTERISING_THREADS`, `confine`),
`crates/pdf-vfs/src/worker.rs` (`InProcessWorkers::spawn`, `InProcess::strips`).

## 1. What the brief's premise said, and what the code did

ADR 1769 section 3 named `pdf-vfs` as the last pixel path drawn by the machine's count. Read against
the code, the premise did not hold and the brief's hypothesis did. `confine` took
`available_parallelism()` before the confinement and drew with `RASTERISING_THREADS.min(machine)`
strips. `RASTERISING_THREADS` is 1 and `available_parallelism` returns a `NonZero`, so the minimum
was 1 on every machine; the machine's answer could not reach a page. The in-process route
(`InProcessWorkers`) and both corpus walks' expectations state `Some(1)` already. So the face drew one
strip, and no held digest moved, because there was none to move: `read_corpus.rs` computes each
render's expectation live, through `pdf_transform`'s own plan at one strip.

What was wrong was the code's account of itself. The count was *derived* from the thread count and
the machine, so a reader saw a machine-chosen number. Two comments said that `render-cpu` draws the
same bytes on any number of processors, which ADR 0219 found false and ADR 1742 measured: 150 of 967
first pages move, by at most 2 levels.

## 2. Decision

- **`serve::STRIPS = 1`**, a constant of its own, stated on both routes. It is one by ADR 1742's rule:
  the face's gate holds bytes (`read_corpus.rs` compares every file a mount serves with its
  generator's bytes) and holds no clock (its cost floor is a count, ADR 0894). One is also all a pool
  of `RASTERISING_THREADS` can draw at once, so it costs no time.
- **`confine` no longer asks the machine anything.** The query was dead arithmetic, and a confined
  process that asks is killed (ADR 0218), so the only place it could be asked was a place where its
  answer was thrown away. The pool is `RASTERISING_THREADS` wide; the strips are `STRIPS`.
- `RASTERISING_THREADS` keeps its value and its reason (ADR 0218). It now says that it costs speed and
  not bytes because the bytes are `STRIPS`'s.

## 3. Calibrated

All from an export of HEAD with this diff and its own `CARGO_TARGET_DIR`, under the lock.

- **`pdf-vfs --test read_corpus`**, the gate that holds the property: unpinned 198.3 s, pinned to
  the eight fast CPUs (`taskset -c 0-3,12-15`) 192.9 s. Both exit 0, and both serve 3 217 renders
  that are their generator's bytes at one strip, with 0 that are not.
- **`--test write_corpus`**: unpinned 34.7 s and pinned 41.6 s, 974 documents, exit 0 both.
- **The plant (trap 13).** `confine` was put back to asking the machine, capped at sixteen, and
  the read gate was run again pinned to eight. It failed with exit 101: 1 312 of the 3 217 renders
  were not their generator's bytes, a few bytes apart in length (`issue19120.pdf` at 300 dpi:
  51 495 against 51 488). So the gate sees a machine-chosen count, and the unplanted tree passes it
  pinned and unpinned alike.
- **Exact by construction, and run to show it.** `cargo tree` puts no `pdf-vfs` under
  `render-raster` or `pdf-model`. The six arms against `/home/AI/arms-1468/`: 5 795 page lines,
  **0 digests and 0 means moved**, every arm exit 0 with HEAD's page counts. `raster_golden`, both
  tests: held 974, moved 0; 153 pages move at some division, 8 named past the bound, as at HEAD.

## 4. Every other `CpuRasterizer::new()` outside a test, read

`grep -rn 'CpuRasterizer::new()' crates/*/src raster/*/src` prints 19 lines; `raster/*/src` names
no directory (raster's crates are under `raster/crates/`, and none of them calls it). Five lines are
inside `#[cfg(test)]` modules (`render-cpu/src/lib.rs`, `pdf-model`'s `soft_mask.rs` and
`transparency.rs` twice, `viewer-core`'s `transition.rs`), ADR 1742 section 4's fixtures. Two state
their count already: `pdf-transform/src/render.rs`'s `Some` arm and `viewer-confined/src/worker.rs`
(`with_strips(limits.strips)`). One is the same `match`'s `None` arm, which ADR 1769 kept for a
caller that compares two renders of its own. The other eleven, and two tools the grep does not reach, keep the machine's count:

- **What a window shows**: the drawing thread every window's page goes through
  (`viewer-host/src/drawing.rs`), the two toolkits' own `rasterizer` (`viewer-gtk`, `viewer-qt`
  `host.rs`), the software path's pages, surround and overlays (`viewer-ui/src/software.rs`, three
  calls), a transition's faces (`viewer-host/src/clock.rs`) and a C caller's frames
  (`viewer-ffi/src/session.rs`, `rasterise`). A window is drawn for a person and compared with
  nothing, and principle 2's latency is what strips buy it. ADR 1554 measured that for the confined
  worker. The surround is an empty list, which draws the same bytes at any count.
- **What is printed** (`viewer-ffi`'s `print_page`, `viewer-qt`'s and `viewer-gtk`'s print paths).
  A sheet goes to a device, nothing in the tree holds its bytes (`tools/drive-windows.sh` compares
  no printed page), and a 600 dpi page is where strips save the most time. The cost is ADR 1742
  section 2's: at most 2 levels, on horizontal edges at a strip's rows, between two machines.
- **`tools/hayro-compare`'s `hayro-speed`** times our page one against `hayro`'s. It is a clock, so
  ADR 1742's rule would state `MAX_STRIPS`, and on this machine the count it asks for is sixteen
  anyway. **`tools/safedocs`' survey** drops the raster and asks only whether the call returns.

**One sentence is wrong, and it is not in this round's crates.** `viewer-confined/src/worker.rs`
says, above the pool's width, that "ADR 0139's split draws the same bytes on any number of strips,
so this costs no pixel". The comment in `tests/confined.rs` above `pixels` and ADR 1554's context
line say the same. ADR 0139's own status line, amended by ADR 0219, calls that property "false and
unattainable". The decision those lines support, the machine's width for a window, stands on ADR
1554's measurement. The sentence is named here for the round that owns the crate.

## 5. Cost, and what is left

- Nothing a window draws changed, and nothing the face serves changed: the constant is the value the
  minimum already produced.
- ADR 1554 found that `confined_transport::Host::start` sets `MALLOC_ARENA_MAX=1` for every worker,
  so ADR 0218's reason for one thread no longer binds this worker either. Widening its pool is a
  speed question for a later round, and `STRIPS` stays one whatever the pool's width, for this ADR's
  reason.
