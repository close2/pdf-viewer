# 3377 — A lint expectation holds on every target the crate builds for

Session 2169. Status: accepted. Context: round 2165's record, ADR 1333; round 2098's trap (no
`#[allow]` another crate rejects). Cites no clause and moves no ledger row.

## What fired

`RUSTFLAGS="-D warnings" cargo clippy --target x86_64-pc-windows-gnu -p pdf-sandbox` failed
`unnecessary_wraps` on `lockdown::apply_for_decoder_within`: off Linux its body was
`Ok(Confinement::NONE)`, and the lint, which spares an exported function, does not spare a
`pub(crate)` one. With `--all-targets`, `tests/confinement.rs` failed twice more off Linux: its
`#![expect]` named `trivially_copy_pass_by_ref` and `unwrap_used`, which only its Linux tests use.

CI's `platforms` job runs `tools/platform-check.sh`, a `cargo check`, on Windows, so no CI job saw
either; they are the lints a Windows developer meets.

## Decision

- Off Linux the decoder's ceiling has nothing to install, so `apply_for_decoder_within` answers
  `apply_for(Profile::Decoder)` — the profile's own answer rather than a second copy of it. The lint
  has nothing to say about a value another function returns, and nothing is silenced.
- The test file's expectation is split: the two lints every target uses stay unconditional, the
  two only the Linux tests reach are `cfg_attr(target_os = "linux", expect(...))`, so the
  expectation is fulfilled wherever it is stated.

Checked: clippy `-D warnings --all-targets -p pdf-sandbox` on `x86_64-pc-windows-gnu` and
`x86_64-pc-windows-msvc`, and on Linux.
