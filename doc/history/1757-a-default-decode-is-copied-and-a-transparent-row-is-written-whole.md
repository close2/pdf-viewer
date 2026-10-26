# 1757 — A default `/Decode` is copied, a transparent row is written whole, and the token lever is priced

Session 1757, slot 6 of batch 115, a measure round; 2026-10-26. ADRs 2348, 2349; no `Q`. No
ledger row moves: both changes keep every byte, so no clause's reading changes.

**Premise.** Held: after round 1749 the turn gate's slowest pages are image pages whose turn is
`interp`. Not held: "ADR 2244 names what the cache holds" — its cache is the kernel's page cache,
and no cache of this program holds a content stream's tokens (ADR 2349 section 1). **Hypotheses.**
Two pages' levers clearing their ceilings held for `images.pdf` and `issue13931.pdf`, not for the
plan (the memo) or `issue12841_reduced.pdf` (the codec). The token lever's hold bounded by a cache
did not hold; it clears the memory gate only because no gate document has the construction.

**Built.** `unpack_eight_bit_device` copies a default `/Decode`'s samples four pixels a step
(§8.9.5.2); `invert_matte_in_place` fills a row whose mask is all 0 with the matte's pixel, read
from the tables (§11.6.5.2). A per-pixel first form was measured and replaced (ADR 2348 section 2).
Instructions, one steady interpretation: `images.pdf` 612.1 → 432.7 M, `issue13931.pdf` 692.5 →
457.7 M; display-list digests equal on four image pages. **Priced, not built**: the group page's
tokens kept for its second plane, about 155 M for 12.7 MB held (ADR 2349, `deferred`).

**Files.** New: ADRs 2348, 2349, this record. Edited: `pdf-model/src/image/{unpack.rs,
dispatch.rs, tests.rs}`, `doc/performance.md` section 3e. I worked in `scratchpad/r1757/`.

**Gates.** Tier 1 for `pdf-model` in the worktree: `rustfmt --check` on the three files exit 0,
`cargo fmt --check` exit 0, clippy (`-D warnings`, all targets) exit 0, `cargo nextest run` 2 202
of 2 202, rustdoc with warnings denied exit 0; six planted defects each fail a route test (3 run,
1 or 2 failed each). On the export: `raster_golden --ignored` exit 0, held 974, moved 0;
against HEAD's six arms, cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965
at 4x, and no page differs in frame digest or mean error (1 463 s under the lock). Clock, one
hold, HEAD interleaved, load 2.5 to 2.9: `t2-turn_path` exit 0 on both, 33 of 33 judged, 0
outside; `t2-launch_path` exit 0 on both, 59 banded, 0 outside, 13 not judged on each for the
device's carve-out, `open_anon_mib` within 0.007 MiB and `peak_anon_mib` within 1.1 MiB of HEAD's
on all six documents; `frame_budget`'s turns in ADR 2348 section 3. An exploration run of the turn gate on HEAD (load 8 to 15) only ranked pages.
`cargo test -p conformance --no-fail-fast` with this record and both ADRs: exit 0, 69 test binaries.

**Unfinished.** The plan's `convert_three` memo (455 M) and the codecs are the image pages' rest,
none priced to a lever; the token lever waits for a census or a budget that charges kept tokens.
