# 834 — The Windows traces arrive, and the sharp pass learns a budget

Date: 2026-08-31. On `main` directly, from `f928da88`.

ADR: 0761 — the sharp pass is declined where a prediction says the machine cannot afford it.

Touched: `crates/viewer-ui/src/bin/pdf-viewer/renderer.rs` (the gate, its constant, three
tests), `doc/todo/50` (rewritten: the retest's first data arrived), `doc/todo/47-the-resize-frames.md`
(one attribution note), `doc/todo/README.md` (50's index line), `doc/QUORRA_FEEDBACK.md` (§41),
`doc/conformance/ledger.toml` (§8.11.2.3 read and kept), this file.

## The round

The owner ran the Windows build on their Intel UHD / DX12 machine and left four files under
`tmp/win/`. Read whole, they answered `doc/todo/50`'s standing question and asked three new ones:

- **quorra ADR 0078 is confirmed on DX12** — the 6.4 s atlas flush reads back as 2.6 ms of
  transfer on the same page's first frame. The retest the todo existed for is done.
- **The worst term in the trace was ours**: ADR 0699's settled-view 2× sharp pass ran 8 867.6 ms
  through the compute lane on that adapter, uninterruptible, while the person's zoom waited
  twelve seconds for its first real frame and the presents — DX12 queue operations behind the
  pass's submission — blocked for up to 5.25 s. The fix this round could take from Linux is a
  prediction gate: the pass is declined where four times the last built frame's cost exceeds
  400 ms, the budget ADR 0699 had already priced and accepted. Constants argued between the two
  measured machines in ADR 0761; three unit tests carry the two witnesses.
- **The compute lane itself is ~35–150× the 890M's cost there** where raw throughput predicts
  12–20× — quorra ADR 0091's occupancy suspicion fits, and it is upstream's to attribute
  (`QUORRA_FEEDBACK.md` §41.1), with the submission-granularity ask beside it (§41.2: one
  submission is also the unit of present latency).
- **`--backend gl` panics in wgpu-hal 30.0.0's one-second context try-lock** — checked against
  our own usage first, and it is upstream's limitation, not misuse (§41.3).
- The trace's 1.3 s first `resize` was attributed before anything was built on it: it is the
  launch path's interpretation step (same instant, same duration in the launch table), not
  `doc/todo/47`'s 9–19 ms resize-frames item. That file says so now.

The spec-driven bite: §8.11.2.3 (Intent), the oldest unread ledger note by blame, read against
`intents_of` and `covers` — all three of its claims hold, and the read is recorded in the note
with the two boundary readings the code makes deliberately.

Gates: the core five (fmt, clippy `-D warnings`, nextest workspace 2820 passed, doc tests, the
two `fuzz/` lines) plus `cargo test -p conformance` (218 passed) — the change is `viewer-ui` and
documents, which under the change→gate map reaches no corpus gate and can move no gate pixel.
§5's binaries rebuilt and installed (round.sh had flagged `target/pdf-viewer` older than HEAD).

What only Windows can verify is written as runs in `doc/todo/50`: the re-run of the traced
session against this build, the `--coverage cpu` lane A/B, the GL backtrace, and what
`entwurf.3.trace.txt` (three lines, then nothing) actually was.
