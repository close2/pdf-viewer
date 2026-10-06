# 1359 — Every host reads the outline beside page one, and the confined worker draws on every processor

The UI slot of batch fifty-six. ADRs 1553 and 1554. No ledger row moved; no question written.

**Premise checked.** `quorra-gtk` and `quorra-qt` hold their own `Viewer` (ADR 1539's `Opening`
thread), the confined window holds none — its worker is the host there — and the C ABI exposed no
section on `PageChanged` at all, so it gained `quorra_event_page_section` beside the preparation.

**The preparation (ADR 1553).** GTK and Qt ask `Viewer::preparation` at the join, run it on a
thread, look for the answer on GTK's main loop or Qt's drawing timer at `drawing::POLL`, and
rebuild the contents panel when it lands; until then the panel says it is reading the outline.
The worker starts the thread after an open's anticipation, inside the confinement; what `prepared`
raises rides on the next events frame, and a `Query::Outline` waits for the thread rather than
reading twice. `quorra-confined`'s caption now shows the section it used to drop. The C ABI gains
six symbols (`quorra_preparation_take`/`_run`/`_free`, `quorra_prepared_hand_back`/`_free`,
`quorra_event_page_section`); version and event-kind count unmoved. The drive's new step
`01-section` reads `page 1 of 3 — Chapter one` in all four windows.

**The confined page (ADR 1554).** Callgrind on one thread, `bug1815476.pdf` page 1: interpretation
121.8 M instructions (before the device is up), rasterisation 143.8 M on the pool thread
(`scan::fill_as` 94.8 M). The lever was already in the tree: `Host::start` sets
`MALLOC_ARENA_MAX=1`, so `glibc` never asks `__get_nprocs`. The pool is now as wide as the machine,
built after the confinement, wherever that variable is set; its stacks are a message-budget term.
Fifteen interleaved launches an arm, one binary built 2026-10-06 00:00 from this worktree (siblings'
`pdf-model` edits in it): device-to-first-frame median 0.054 s → 0.026 s, first `Resize` round trip
0.039 s → 0.011 s, first frame 0.146 s → 0.122 s. WTPDF and ISO 32000-2 cross as marks and did not
move (ten launches an arm over 1, 2, 4, 8, 24 threads). Round 1361's unpack is the interpretation's
half and lands before the device.

**Gates.** rustfmt --check on my twelve Rust files 0. clippy `-D warnings` on viewer-ffi,
viewer-confined, viewer-gtk, viewer-qt and viewer-ui, all targets: 0. nextest on the five: 177 and
148 passed, exit 0. `cargo test -p conformance` 0. Drive behind the lock: exit 0 in 683 s, 115
works, 0 wrong, 3 not offered — the four new rows are `01-section`.

**Hunks outside my files.** `doc/todo/42` (the three-hosts paragraph), `doc/state-of-play.md` (one
launch sentence). `viewer-core` and `pdf-model` untouched.
