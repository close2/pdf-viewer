# 1351 — A turn is judged on a quiet device, and the curve search ends at a bar

Instruments slot of batch fifty-four. ADRs 1537, 1538. No ledger row moved; no question written.

**Device check (ADR 1537).** `turn_path` reads amdgpu's `gpu_busy_percent`, twenty readings over
half a second, before each child and after it; a child bracketed above `device_busy_percent = 20`
(new key in `doc/checks/turn-path.toml`) is printed with the figure and not judged. A device with
no counter is printed unread. Calibration: idle 0–1%; a planted compute loop
(`keep_the_device_busy`, the test binary's `busy` phase) 88–100%; quiet children 1–6% before and
6–9% after. With the plant running the whole gate, 20 of 20 figures read `not judged (device 99%
busy against 20%, …)`, every one above its band's ceiling (steps 41–96 ms), and the run exited 0
in 765 s. Under the plant the calibration probe read 1.24–1.59 ms too (one package, one memory), so
a saturating neighbour was refused under the wrong name; trap 101's was partial. The two ignored
tests share a mutex, so `-- --ignored` never runs the plant beside the gate.

**Ed448 (ADR 1538).** Only a stable release on RustCrypto's line or a package with an audit
covering the arithmetic and RFC 8032 section 5.2.7's verification reopens the swap. The re-check is
`doc/todo/65`'s command block, not a section, because a section may not wait on the network. Run
on 2026-10-05: `ed448-goldilocks` 0.14.0-pre.15, no `bp512`, no audit named. One dated list.

**Documents.** state-of-play: the kept meet (1517), the fontless show and one lookup (1521), two
`zune-jpeg` patches (1520), a field's characters and GTK's node (1501, 1516), Annex O's eleven
(1523). PLAN: the oracle holds what it reports (1522). crate-map: 1517, 1521, 1516. HANDOVER: the
merge's order with the conformance run in main (trap 100) and the prune between `close` and `open`
(1526), the pointer rule (1525), the curve rule, the waiting patches. `todo/README` row 36. Navigation
before/after: 14 → 15 absent (fixed one of mine; the rest siblings'), 28 undefined, history 0.

**Found.** `main-checkout` counted the two `zune-jpeg` patches as applied ("whose base it no longer
pins"); they now print `waiting:` on Q227, and `doc/environment.md` names the line. It prints
nothing about `target/` — `tools/state.sh binaries` does. Records 1340–1345 are 35–40 lines, each
with **Gates.**; `batches`: 5f1bdc55 is 50242 s, 6 figures, 6 rounds, no remainder. No `bug1721218`
row in `doc/performance.md`: owed in `doc/todo/36`. `batch.sh check` said "over" for any failing record; it now says which.

**Gates.** `rustfmt --check` on `turn_path.rs` 0. Clippy `-D warnings -p render-raster
--all-targets` 0 on an export of HEAD with my file (101 in the worktree, a sibling's scratch timer
in `scene/own_space.rs`). `turn_path` non-ignored 3 passed. `bash -n` state.sh, batch.sh 0.
`cargo test -p conformance` 0 but `records`, on 1346's record in progress. Behind the lock:
calibration test 0 (88.3% planted); the gate planted 0 (0 of 20 judged); the gate in the worktree
101 (`personwithdog.pdf` turn 11.43 ms over 10.80, a sibling's in-flight edit); the export of HEAD
with my harness 0 (18 of 20 judged inside, personwithdog 9.83 ms).
