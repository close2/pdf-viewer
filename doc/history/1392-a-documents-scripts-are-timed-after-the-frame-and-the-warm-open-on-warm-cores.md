# 1392 — A document's scripts are timed after the frame, and the warm open is taken on warm cores

Perf slot of batch sixty-one. ADRs 1620 and 1621; no ledger row moved, no question.

**The premise, checked.** ADR 1609's `script_open spawn_ms=1.8–2.1 first_run_ms=1.1–1.7` is in
milliseconds, as its key says: 0.0018 s, not 1.8 s.
**Measured, gates profile, worker `--features engine`.** Spawn 0.85–1.72 ms (binary's load about
0.6, confinement a tenth or two; the worker is 14.3 MB PIE, 87 033 relocations); first run 0.53–0.59
ms on `opt_demo.pdf`'s `0;` (realm 0.40–0.51), 16.8–17.8 ms on `evince-LINK-46-2.pdf`'s 173 KB, where
callgrind puts Boa's AST optimizer at 70.8% of the process. Levers priced and declined (ADR 1620
section 3): a warm realm, a spawn at `Command::Presented`, a non-PIE worker; the optimizer is the
engine owner's.
**The stage.** Every first-page child of `launch_path` hands its viewer a `ScriptWorker` maker
through round 1390's `Command::Scripts` before the open, and sends `Command::Presented` after the
frame's figures: the timeline gains the open sequence and `script_open`; `opt_demo.pdf` is a new row
banding `script_spawn_ms` 0.58 .. 0.86 and `script_first_run_ms` 0.38 .. 0.57; a worker started
before any frame fails the run; no worker built prints `NOT MEASURED`. Time to first page against
HEAD, three runs each interleaved: every row inside its unmoved band, frame hashes equal.
**The warm open (ADR 1621).** Its child spins 30 ms first, the cold arm stays idle-born; three warm
bands re-taken over six runs and narrowed. Ten quiet runs of HEAD's turn gate read the Type 3 page
at 10.67–11.29 ms turn, 10.29–10.89 seventh, 11.45–11.75 step: the excursion was the busy end of
trap 110; `personwithdog.pdf`'s turn read 11.26 once on warm cores.
User AI's tasks at the first heavy run: 132.

**Handed over.** To round 1391, the sentence for `turn-path.toml`'s Type 3 row; to the worker's
owner, the `SIGSYS` at a clean end of input (ADR 1620 section 6) and the operator-chain overflow
(section 3). Other files touched: `crates/viewer-ui/Cargo.toml`, `doc/todo/02-every-round.md`,
`tools/state.sh`, `doc/verify.md` (one build line each).

**Gates.** `rustfmt --check` on `launch_path.rs`: 0. `RUSTFLAGS=-D warnings cargo clippy -p
viewer-ui --all-targets`: 0. `cargo nextest run -p viewer-ui`: 0, 148 passed, 2 skipped. `cargo test -p
conformance`: 0, 395 passed. Behind the lock, `pdf-sandbox` and the worker rebuilt inside it: `launch_path`
counted 0, 31 banded, 0 outside, in 3 s; with clocks 0, 53 banded, 0 outside, in 18 s; HEAD's
`turn_path` ten times: 9 of 10 exit 0, 33 judged each.
