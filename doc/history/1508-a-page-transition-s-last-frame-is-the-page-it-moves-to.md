# 1508 — A page transition's last frame is the page it moves to, and nothing else

Slot 7 of batch eighty, 2026-10-12, a loop round on the owner's bug report of that day. ADR 1853;
no Q. Ledger: 12.4.4.1 stays `implemented`; its test list and note gain the end frame.

**Reproduced first** (`examples/presentation_fixture`, Xvfb, `xwd` as fast as it goes through a
`Wipe`): two photographs at the transition's end, up 17–32 ms on the device and 12–23 ms under
`--cpu`, held 379 585 of 800 000 pixels that were neither page — the arriving slide 1.25 times too
large and moved up, 1.25 being its own magnification. **Cause**: a transition frame is presented
as a `Placed` with the arriving page's identity at the identity target; it lands a tick later and
`Stale::settled` recorded it as that page settled at the identity (`forget` ran before the adopt),
so the next tick saw a magnification change and, where rule 5 predicted a miss, put up ADR 0378's
reprojection of two pages by one page's transform. **Premise held**; **hypothesis did not**: not
ADR 1831's looks nor ADR 1832's stand-in — older, and shown only on a predicted miss.

**Built.** `stale::Depicts` on `Placed`; a landed transition frame settles nothing and is never
retained; `presentation::End` draws the frame at progress 1 once when `/D` runs out (the clock
draws nothing then, leaving a sliver of the old page up). Tests
`a_transition_frame_settles_no_view_and_is_never_retained` and
`the_last_frame_is_the_arriving_page_byte_for_byte` (every shaped style, both `Fly` motions) each
fail with the fix planted out. Drive step 83 `transition_frames`: a three-slide show, every
distinct picture between two settled pages judged channel by channel.

**Figure, Xvfb, pictures of neither page at a transition's end**: before 2 (device), 2 (`--cpu`)
→ **0, 0**; step 83 in the whole drive: 64 + 51 and 53 + 29 moving pictures, none of neither page.
Planted build: the device arm caught it in 0 of 2 runs (a predicted miss decides), `--cpu` in 2 of
2. `quorra-qt` passes; `quorra-gtk` fails on another seam — about 2 970 pixels along the page's
edge in mid-transition frames, a face's antialiased edge row against the page's — the toolkit
host's. The owner's display was not used: both lanes reproduced it headless.

**Files.** `crates/viewer-ui/src/bin/quorra/{stale,presentation,surface,renderer}.rs`,
`crates/viewer-core/src/transition.rs` (a test), `tools/drive-windows.sh` (step 83), the ledger
(12.4.4.1), `doc/state-of-play.md`, ADR 1853, this record.

**Gates.** rustfmt --check 0; clippy `-D warnings` `viewer-core`, `viewer-ui` all targets 0;
nextest both 535 of 536, the failure `no_library_in_this_tree_knows_what_a_reprojection_is` on
slot 2's new `viewer-host/src/stand_in.rs`; `cargo test -p conformance` 0 but `records` (slots 2
and 6's in progress); drive whole (`--tree 12`, wait 6 327 s, hold 947 s) exit 0, 275 works,
1 wrong (83, `quorra-gtk`); `launch_path` (`--clock`, wait 14 s, hold 211 s) exit 0, 0 outside.
