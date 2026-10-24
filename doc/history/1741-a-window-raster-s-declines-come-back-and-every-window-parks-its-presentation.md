# 1741 — A window raster's declines come back, and every window parks its presentation

Session 1741, slot 6 of batch 113, a host round; 2026-10-24. ADRs 2316, 2317; no `Q` (Q604 not
needed). No row moves; §12.4.4.1's note names the new parking. I worked in `scratchpad/r1741/`.

**Premise.** Held: no call handed a C window raster's declines back; the confined title missed a
worker's pixel page; `quorra` did not park; GTK under Wayland never parked (a nested KWin's
minimise sets `SUSPENDED`, never `MINIMIZED`); the instruments skipped dead locks and killed their
servers; the ranking overruled the device-select layer. Not held: `quorra` kept one presentation
per window, so a tab change while presenting needed code, not only a drive step; no input changes
a tab while presenting, so step 100 has a link do it; `Toplevel::state` truncates the bit, so it is
read off the property (a first form parked nothing); the GTK floor needs no owner's word.

**Change.** `quorra_raster_declined` (header, .NET, counts, C driver) over `Session::window_declined`;
`Framed::declined`, one wire byte a page, `Marks::declined_here`, the confined title; stand-ins say
nothing, decided (ADR 2316). `quorra` parks on occlusion and `is_minimized` and keeps a `Presenting`
per tab; GTK reads `SUSPENDED` under `v4_10`; `toolkit_launch/x_lock.rs` frees a dead own-user lock
and stops with `TERM`, shared with GTK's stand-in tests and restated in the drive; the layer's first
adapter is taken where `MESA_VK_DEVICE_SELECT` or `DRI_PRIME` chooses (ADR 2317). Drive steps 98 in
`quorra`, 100 new, 89's confined leniency removed.

**Measured.** Nested KWin, 2 s slides minimised 4 s: this tree parked, HEAD's binary turned three
slides unseen. The confined title names the worker's page 1, HEAD's not. Planted out, step 98 in
`quorra`, step 100 and the llvmpipe child test fail.

**Gates.** On my export, HEAD plus this round's hunks alone (its 33 paths checked equal): `rustfmt
--check` per file 0; `cargo fmt --check` on the eight crates 0; warnings-denied clippy `--all-targets`
on them 0 (101 once on GTK, a `doc_markdown`); crate-scoped warnings-denied rustdoc 0; `cargo nextest
run` on the eight 1548 of 1554, the six the entry counts (then 69 of 72 in `viewer-ffi`), two of
`cargo_artifacts` and the C driver's two, which cannot find Cargo's profile in an export, and one
`confined` test that fails in HEAD's export too; in the worktree `viewer-ffi` and `viewer-confined`
188 of 190, the two the siblings' entry points, the C driver passing; `cargo test -p
viewer-confined` 118 passed; `tools/dotnet-check.sh` 0. The six arms, exit 0 each: 5 799 pages by
page against HEAD's six arms, 0 digests differ. Drive, dev binaries, exit 0: 415 works, 9 wrong, 10
not offered; of the wrong, `88-archive-refused` thrice and `66-signature-policy` are HEAD's, and
GTK's `73`, `74` and Qt's `01-center-window` work rerun, while `82-retained-page` lost its Left key
on HEAD's binary twice in three too. Step 83: 12 of 12 works at loads 3.8 to 48.3 during a
sibling's arms walk. `t2-toolkit_launch`, one `--clock` hold at load 2.07: exit 0 and 0, every
figure in its band, the harness taking `:160` from a dead server's lock and leaving none; `cargo
test -p conformance` 463 passed, and `--test evidence --test adr_status --test records` 0.
