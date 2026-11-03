# 1912 — CI green for its real reasons

Session 1912, a single round (pixels, dependency) on CI's three reds since batch 131. ADRs 2658 and
2659; Q776 unused.

**What.** `deny`: `async-executor` 1.14.0, yanked upstream for a use-after-free when a `Waker` fires
during `Executor::drop` (its changelog), goes to 1.14.1; `Cargo.lock` moves that one entry's version
and checksum and nothing else. `miri (pdf-render)`: `dash_end` measures a straight segment with
`sqrt` of exact `f64` squares, narrowed, instead of `f32::hypot`, whose last bit is the platform's
(ADR 2658), and the §8.4.3.6 row names the new test. `build (Windows)`: `t88_conformance` takes
`$T88_CONFORMANCE_DATA` when set and asks `tools/places.sh` only on Unix, keeping its assertion
there; elsewhere, unset, it prints its sentence (ADR 2659).

**Premise.** Held for `deny` (`cargo deny check` refused the yanked 1.14.0) and for the Windows job
(the other `places.sh` readers are `conformance`'s, which the Windows job does not run). The
orchestrator's "the data is present" did not: Annex K's data was not on this machine and the test
skipped; the Recommendation's zip was fetched, its SHA-256 matched `doc/third-party-data.md`'s, and
it was unpacked where `places.sh` names it, so the gate below decoded.

**Hypothesis.** Held. At HEAD Miri failed `each_subpath_answers_for_itself` (the end of a vertical
rule of 6 lost its dot); with `-Zmiri-deterministic-floats` all ten passed over eight seeds; with the
new function all eleven pass over thirty-two seeds of Miri's perturbation. Natively, over 199 480 297
finite `f32` pairs the new function and glibc 2.41's `hypotf` agree to the bit, so no pixel moves
here and the pixels tier 2 was not owed. `closing.rs`'s `distance` has the same shape and passed
sixteen Miri seeds; not this round's file (ADR 2658 names it).

**Gates.** `cargo deny check` exit 0. `cargo fmt --check -p pdf-render -p pdf-sandbox` exit 0;
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-render -p pdf-sandbox --all-targets` exit 0 (first run
101: an `#[allow]`, float `assert_eq!`s and an `Option` wrap, each fixed). `cargo nextest run -p
pdf-render -p pdf-sandbox` exit 0, 376 passed. `cargo test -p conformance` exit 0, 471 passed.
`MIRIFLAGS=-Zmiri-many-seeds=0..32 cargo +nightly miri test -p pdf-render --lib dash_end` exit 0.
`cargo +nightly miri test -p pdf-render --lib` exit 0, 296 passed, 16 ignored, 3807 s. `cargo test -p pdf-sandbox --test t88_conformance` exit 0 three ways: by `places.sh`, by
the variable set, and with it empty — "2 of Annex K's 10 streams decode", as held. `tools/state.sh --round 1912 docs` exit 0 alone; twice exit 101 on a rustc internal error (`pdf-render`, then `pdf-archive`, each documenting cleanly by itself) while the nightly Miri run shared the build directory. The Windows
target is not installed here, so `cargo check --target x86_64-pc-windows-gnu` was not run.
No arms: no pixel moves on this machine.
