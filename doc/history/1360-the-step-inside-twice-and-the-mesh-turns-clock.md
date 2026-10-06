# 1360 — The zoom step inside twice the CPU backend, and the mesh page's turn is the clock

Date: 2026-10-06. ADRs: 1555, 1556. The RASTER slot of batch fifty-six.

**The step.** ADR 1541's instrument first (callgrind by function, one thread, GPU lane, the 1.25×
frame): the small regions' first fills 0.230 G, 3 515 regions each one dot of 37 points over six
pixels; the two renders' `draw_encoded` 0.240 G, of it 7 138 buffers and bind groups a frame. By the
clock the order reversed: skipping every small fill (a probe arm) saved 4.5 ms, regions made ahead
on helper threads 1.6 ms (built, byte-identical, set aside), and `prepare_run` alone took 9 to 10 ms
a render. So a pass's shading quads now read one buffer of their numbers at their own dynamic
offsets through one bind group per paint and mask (ADR 1555): step 108.4 → **79.2 ms**, 1× frame
116.0 → 89.2, the CPU backend 56.0 and 47.0 in the same sitting — **1.41×, inside 2×**, and
`doc/QUORRA_FEEDBACK.md` section 67 says so. HEAD itself read 1.91–1.94× in two quiet sittings.

**The mesh turn.** Not the tree: the banded tree (`47ec7f10`) and HEAD read the same interleaved, at
load 0.5, 3–4 and 13. Not the device. The clock, at both ends: on a quiet machine a second's idle
before the child gives 11.5–11.7 ms against 8.9–9.3 after a spin; at load 5 that does nothing, and
sixteen spinners on the cores the child is not pinned to give 13.3–13.9 on both trees. The band
stays; its `why` says this (ADR 1556).

**Tables.** `doc/performance.md` §3e: four rows re-taken with `taken` 2026-10-06; `turn-path.toml`:
`bug1721218_reduced.pdf`'s bands moved to 137.0 .. 195.3 and 71.7 .. 102.9 by the file's rule.

**Proposed trap.** A light page's `turn_path` child on an idle machine lives its whole run at the
idle clock, so its turn reads a quarter high; the calibration probe declines most such children but
not all (0.675 ms beside an 11.63 ms turn), and at the other end a neighbour on the cores a child is
not pinned to slows it at a load the ceiling admits.

**Gates.** rustfmt --check on the four Rust files: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p
raster-gpu --all-targets`: exit 0. `cargo nextest run -p raster-gpu`: 685 passed. `cargo test -p
conformance`: exit 0. `render-raster --test corpus` on the CPU, GPU and compute lanes at 1× and 4×
against exported HEAD (2 593 s behind the lock): 0 pages moved on six arms, verdicts equal,
one-versus-many 0. `raster_golden`: held 974, moved 0. `turn_path` four times behind the lock, 77–110 s
each: 0 outside; 16, 16, 16 and 20 of 22 judged, the unjudged rows the lightest, on their
calibration (0.94 ms) at load 0.3–1.1 — `personwithdog.pdf` in all four, which is ADR 1556.
