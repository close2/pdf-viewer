# What every round does

Status: **standing** — this one is never done.
Priority: 02

A "round" here is one session's worth of work. `CLAUDE.md`'s two tracks decide *what* it
contains; this file is what it does around that, in order.

## 1. Take from both tracks

Demand-driven is what the corpus and the oracle name (todos `10`–`29`); spec-driven is the
ledger's `reported` rows and the notes on its `partial` ones (todos `00`–`09`). A project
running only the first finishes when the corpus goes quiet, which can happen with much of the
standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises.

## 2. Run the gates that can see what you touched

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets      # must be silent of lints
cargo test --workspace
cargo build --release -p pdf-sandbox --bins # trap 10: Cargo will not do this for you
cargo test --release -p pdf-model      --test corpus          -- --ignored --nocapture
cargo test --release -p pdf-model      --test oracle          -- --ignored --nocapture
cargo test --release -p pdf-model      --test text_extraction -- --ignored --nocapture
cargo test --release -p pdf-model      --test dates           -- --ignored --nocapture
cargo test --release -p render-quorra  --test corpus          -- --ignored --nocapture
cargo test -p conformance -- --nocapture
```

`doc/HANDOVER.md`'s "Verify it" has the rest — `cargo deny`, the five fuzzers, the callgrind
counters — and says which of them a change needs.

## 3. Leave the ledger non-`unreviewed`

Every clause a change touches gets its row in `doc/conformance/ledger.toml` brought up to date.
This is `CLAUDE.md`'s rule and not a courtesy: a row that describes what the code *should* do is
how this project has been wrong four times.

## 4. Sweep, after a round that adds a verb

Three greps over the ledger's notes, twenty lines of Python apiece, each of which has paid on
its first run: a note whose stated blocker has expired ("while §X does not exist", "needs §Y"),
a note claiming an entry is unread where the tree reads it, and a note whose reason is a
*capability* — "this program has no ___", "no panel", "which this is not". The third found a
`shall` that had been binding for fifty-six sessions (see `01-ledger-partial-rows.md`).

## 5. Put the binaries where a person can run them

**The agent builds into `/home/AI/cargo-target/pdf-viewer/`, which the human's shell never looks
at.** So at the end of every round, copy what a person would run into the project's own
`target/`:

```sh
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-viewer          target/pdf-viewer
install -Dm755 /home/AI/cargo-target/pdf-viewer/release/pdf-sandbox-worker  target/pdf-sandbox-worker
```

Both, and both beside each other: `pdf_sandbox::WORKER_PROGRAM` is a separate executable the
viewer spawns for JBIG2 and JPEG 2000, and a viewer that cannot find it refuses those images
rather than falling back (there is deliberately no in-process fallback — see "the sandbox is a
flag and the default is the safe one").

Build them first, in release. `cargo test` only ever builds the debug binaries, and a stale
executable is a measurement of the past — the hundred-and-forty-second session was reported as
"still lags" against a binary three hours and six commits old, one of which was the 40×
page-turn fix.

## 6. Write it down, then commit

- The ADR, if the round made a decision. The argument goes there, not in the handover.
- `doc/HANDOVER.md`: the gate numbers if they moved, one row in "How the project got here".
- The todo file: delete it if the item is done, correct it if the round changed what it owes.
