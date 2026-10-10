# 1495 — The confined window answers while its worker draws

Slot 6 of batch seventy-eight, 2026-10-10, a loop round. ADR 1827. No ledger row: the owner's target.

**Premise, held**: after round 1489 the confined window answered no key for 6.70 s of the heavy page.
**Hypothesis, half held**: not `Query::Frame` but the command before it. The worker draws a page
crossing as pixels inside `perform`, so the window's thread sat in one `handle(&Command::Resize)` for
the whole draw (1489's trace: sent 2.538 s, answered 11.016 s, frame 11.021 s); ADR 1811's shape fit.

**Built.** Each command goes to the worker on a `viewer_host::Loading` thread, waited for at most
`WARN`, and past that the title says what the worker is doing. Escape, `q` and the close act at once,
the last two killing a busy worker. Every other key is held with its modifiers, a resize as its
newest size, Tab's focus question behind its move, and what an answer causes runs before what was
held. The first-frame stamp and the report wait for a page, since the window now presents the bare
surround while it waits. Step 80 gains `key-delivered`, which is HEAD's `key-wait`.

**Xvfb, llvmpipe**, `tools/state.sh heavy`, HEAD bf2a9b00's binaries (`HEAVY_BIN`) → the tree with
the batch in (built in the hold, 05:17). Seconds by the trace — first frame; open joined; key-wait;
key-delivered: `quorra-confined` 14.47 → 10.95, 5.21 → 3.44, **8.78 → 0.93**, 8.78 → 10.87; the last
rose because HEAD traced and dropped keys pressed during the open, now held and sent (1489's after:
10.73, 3.66, 6.70). Siblings' windows: `quorra` 12.58 → 8.31, `quorra-gtk` 11.09 → 5.32, `quorra-qt`
21.74 → 6.09; their key-waits held. Peaks held: worker 1 725 → 1 728 MiB, host 131 → 132 MiB `VmHWM`.

**Owner's display** (XWayland, 890M, no key, both arms in one clock hold at load 1.5–2.5, after
binaries built 05:43): first frame; open joined. `quorra-confined` 14.19 → 13.05, 3.51 → 3.11, its
sentence at 4.11 s; `quorra` 18.16 → 14.82; `quorra-gtk` 9.12 → 6.12; `quorra-qt` 14.62 → 4.97.

**Unfinished**: a held key reaches the viewer only when the draw ends, since the worker performs one
command at a time (ADR 1827 section 3, `doc/todo/30`). No other confined drive step outlasts `WARN`,
so the held path is driven by step 80, five unit tests and a manual run: Escape mid-draw said the
abort and kept the window, `q` left at once, and neither left a `pdf-view-worker` behind.

**Files.** `crates/viewer-ui/src/bin/quorra-confined{.rs,/exchange.rs (new),/typing.rs,/screen.rs}`,
`tools/drive-windows.sh` (step 80), ADR 1827, `doc/state-of-play.md`, `doc/todo/30`, this record.

**Gates.** `rustfmt --check`, `bash -n`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ui
--all-targets`: exit 0. `cargo nextest run -p viewer-ui`: 174 passed, 2 skipped. `cargo test -p
conformance`: exit 0. `awkward_classes` (queued 1 922 s): exit 0, 0 killed. Drive whole (`--tree 12`,
queued 1 365 s, held 1 104 s): 270 works, 0 wrong. `heavy` before (queued 23 s, held 64 s) and after
(queued 837 s, held 76 s): exit 0, 4 works each; the owner's pair held 107 s. Duration 10 600 s.
