# 1749 — A paint asks for its alpha, a byte is read as a byte, and the bends' encode is its tiling

Session 1749, slot 5 of batch 114, a measure round; 2026-10-25/26. ADRs 2332, 2333; no `Q`. No
ledger row moves: both changes keep every byte, so no clause's reading changes.

**Premise.** Held: `bug1721218_reduced.pdf`'s interpretation and `bug1743245.pdf`'s encode are the
terms the turn gate names. **Hypotheses.** Neither held. The first page names no font; its
interpreting thread is the 2.8 MB group read on two planes, 83 ramps, and 65 M of RGB conversions
whose colour the paint threw away (ADR 2332). The second page has no font and no atlas; 68 % of its
encode is ADR 1375's tiling, and the two levers under it are ADR 1505's, built and dropped (ADR 2333).

**Built.** `ColourSpace::rgb_alpha`, the conversion's alpha without the conversion, and
`Compositing::paint` converting only on `Device`; `BitReader::read` taking an aligned read of whole
bytes as bytes, which is also the meshes' reader (§8.7.4.5.5 to §8.7.4.5.8). Each has a test that
fails on two planted defects. Instructions, one steady interpretation: `bug1721218_reduced.pdf`'s
interpreting thread 965.6 → 869.9 M, `personwithdog.pdf` 105.6 → 24.8 M; display-list digests equal.

**Files.** New: ADRs 2332, 2333, this record. Edited: `pdf-colour/src/{colour.rs, colour/to_rgb.rs,
colour/compositing.rs, function.rs}`, `doc/performance.md` section 3e, `doc/checks/turn-path.toml`.
I worked in `scratchpad/r1749/`.

**Gates.** Tier 1 for `pdf-colour`: `cargo fmt --check` exit 0, `rustfmt --check` on the four files
exit 0, clippy (`-D warnings`, all targets) exit 0, `cargo nextest run` 148 of 148, rustdoc with
warnings denied exit 0; `pdf-model`'s colour, spot, group, soft-mask and multitone tests 150 of 150.
`raster_golden --ignored` on the export: held 974, moved 0. Against HEAD's six arms on the export:
cpu, gpu and compute at 1x and 4x each exit 0, 968 pages at 1x and 965 at 4x, and no page differs in
frame digest or mean error (1 520 s under the lock); the comparison names 105 pages between two
different arms. Clock, one hold, HEAD interleaved: `t2-launch_path` exit 0 on both, 0 of 59 outside;
`t2-turn_path` exit 0 on HEAD and 101 on the change, its one figure of 33 outside
`personwithdog.pdf`'s turn, 6.97 ms under a 7.30 floor; a second hold's two runs (101, 101, HEAD's
between them 0) read 7.07 and 7.09, and that row's turn and seventh bands move by ADR 1969's rule
(ADR 2332 section 6), which the gate's band-file test reads (3 of 3 pass). A first hold of mine
was granted before the export's own build had run and was stopped by its process group after 368 s;
nothing of it is quoted. `cargo test -p conformance --no-fail-fast` with this record and both ADRs:
exit 0, 69 test binaries, `evidence`, `adr_status` and `records` among them.

**Unfinished.** The group page's content read once a plane, about 170 M a turn, is priced and not
built (ADR 2332 section 5); the tiling's cut count is the only lever left on `bug1743245.pdf` and it
moves pixels (ADR 2333 section 4).
