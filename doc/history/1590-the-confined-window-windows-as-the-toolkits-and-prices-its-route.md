# 1590 — The confined window draws a window where the toolkits do, and prices a page's route

Slot 6 of batch ninety-three, 2026-10-19, a host round under the owner's switch to code improvement.
ADRs 2016 (ADR 1994's bound across the wire) and 2017 (`doc/questions/A477`'s build, which the
owner's checkout holds). No Q file; no ledger row moves.

**Premise.** It held: the worker held a window only past `MAX_PIXELS`, and the decoder refused one
under it. **The hypothesis did not**: the host holds no list of a page that crosses as pixels, so the
worker answers the rule, and `Oversize` carries nothing new. The brief's "steps 82 and 84": step 84
is the bare-session start; the confined window's steps are 82's third and fourth parts.

**Built.** ADR 2016: `windowed::worth_a_window`, the toolkits' rule stated again and held equal to
`viewer_host::oversize::worth_a_window` by two tests through a dev-dependency; the worker's
`Turning::draws_a_window_of` for a host that said `Listed`; the decoder admitting such a window;
greeting `PDFVCF16`; a trace line per window the worker sent; step `82-zoomed-window` judging the
confined window. ADR 2017: `FRAME_ROUTE` and `Confined::route`, the worker recording a route and
moving the page at the next command, `Framed::bytes`, and `quorra-confined`'s `pricing` with the
crossing counted, saying `window chosen` or `device chosen` each priced frame.

**Measured** (ADR 2017 §3). One clock hold, the witness, llvmpipe under Xvfb, HEAD's binaries
against the export's, alternated: twenty notches settled in 66.3 and 47.8 s against 17.8 and 18.2 s;
a press's next frame 160 and 159 ms against 4 and 6 ms (median); final photographs 0 pixels apart.

**Files.** `crates/viewer-confined/{Cargo.toml,src/lib.rs,src/protocol.rs,src/protocol/windowed.rs,
src/worker.rs,tests/confined.rs}`; `crates/viewer-host/src/oversize.rs`; `crates/viewer-ui/src/bin/
quorra-confined.rs` and `quorra-confined/{exchange,screen,pricing}.rs`; `Cargo.lock` (one line);
`tools/drive-windows.sh` (step 82's third part); ADRs 2016, 2017; `doc/todo/30`, `doc/state-of-play`.

**Gates.** rustfmt `--check` on the ten Rust files: 0. `-D warnings` clippy `--all-targets` over
viewer-confined, viewer-host, viewer-ui: 0. Nextest over them: 596 passed, 4 skipped. Three plants
(trap 13), each failing its tests; the route's integration test missed the one in `decide`, so a
store test was added for it. `cargo test -p conformance`: 439 passed, 1 failed (`records`, on 1586's
record). `tools/state.sh --round 1590 docs`: 0; `confined`: 0, killed 0. The drive's confined steps from the
export: 3 works, 0 wrong; HEAD's `82-zoomed-window`, the control: wrong. Clock hold: 233 s, exit 0.
About 7 700 s.

**Unfinished.** The confined processor screen windows a marks page only past the bound (ADR 2016
§3). `doc/ui-boundary.md` names the greeting `PDFVCF14`, stale since before this round.
