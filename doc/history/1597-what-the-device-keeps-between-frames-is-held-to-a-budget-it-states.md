# 1597 — What the device keeps between frames is held to a budget it states

Slot 6 of batch ninety-four, 2026-10-19, a pixels round under the owner's switch to code
improvement: `doc/todo/75` item 5, and `content.rs`'s counted reason. ADRs 2030 and 2031. No Q
file; no ledger row moves (the work cites no clause).

**Premise.** Two parts did not hold. The adapter states no memory size (`wgpu` 30 has none, ADR
1823 §3), so the budget is no share of it; and ADR 2016's windows and ADR 1968's lanes hold nothing
on the device — the kept instance streams were the one page-derived allocation held between frames
under no bound. The hypothesis's other halves held: no pixel moved, and the witness stays under.

**Built.** ADR 2030: `Limits::max_resident_bytes`, the host's `Options::max_resident_bytes` held to
the frame ceiling; `device/residency.rs`, a charge taken before the streams are made and returned on
its drop; a refused encode keeps the refusal for its life and makes its streams per frame, shared by
the frame's own attempts; `RetainedScene::residency_refused`, `Device::resident_bytes`;
`render-raster` says it once a scene; `zoom_frame`'s `ZOOM_FRAME_RESIDENT`. ADR 2031: the reason
states the rule, and why `spelled_ordinals` reads rounds and no gate can read a count by shape.

**Measured** (ADR 2030 §3). One clock hold, the witness on the 890M headless, default against
refused, two runs a lane: the replay's transfer 1.8–2.6 ms kept against 27.2–38.1 ms refused, its
bytes 36 360 892 / 95 529 537 against 300 860 300 / 360 028 945; the witness keeps 264 499 408 bytes,
12.3 % of the 2 147 483 644-byte budget; the turn gate 33 banded, 24 judged, 0 outside.

**Proof that nothing moved.** The witness's six digests equal in both arms and HEAD's. Arms by page
against HEAD's arms: gpu and compute at 1× (968 pages) and 4× (965), 0 differing.
Plants (trap 13): the ceiling ignored, the refund dropped, two tests failing each.

**Files.** `raster/crates/raster-gpu/src/{device.rs,device/residency.rs,device/staging.rs,
device/render.rs,device/construct.rs,retained.rs,startup.rs,lib.rs}`, `tests/residency.rs`;
`crates/render-raster/src/present.rs`, `examples/zoom_frame.rs`; `crates/pdf-model/src/content.rs`
(one reason); ADRs 2030, 2031; `doc/todo/75`, `doc/state-of-play.md`. Slot 2's hunks in
`staging.rs` and `retained.rs` (ADR 2022) build on `Residency` and were left out of the export.

**Gates.** rustfmt `--check` on the twelve Rust files: 0. `-D warnings` clippy `--all-targets` on
raster-gpu, render-raster, pdf-model in the export: 0. Nextest over the three there: 2951 passed,
26 skipped. `t2-raster_golden`: 2 passed, exit 0. The arms: four, exit 0. `cargo test -p
conformance`: 442 passed, 0 failed (an earlier run's two were 1592's record and this one uncut).
`tools/state.sh --round 1597 docs`: 0. Clock hold: 348 s, exit 0. 5 700 s.

**Unfinished.** None owed. Found: the segment arena's holes stay until the device goes, uncharged.
