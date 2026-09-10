# 945 — A gate that was not failing, it was not running

Date: 2026-09-10. A fifth round, so `doc/todo/02` §2 ran whole and §5 rebuilt the binaries.
Files: `tools/state.sh`, `doc/todo/02-every-round.md`, `crates/pdf-archive/src/table/fonts.rs`,
`crates/pdf-fuse/src/main.rs`, `crates/pdf-transform/src/bin/quorra-transform.rs`,
`tools/pdf-retrieve/src/main.rs`, `doc/habits.md`.

Three things described something they no longer described, and the full sequence found all three.

**`tools/state.sh` named a package that does not exist.** Its "quorra against the CPU oracle"
section ran `-p render-quorra`; the adapter crate is `render-raster`, which is what `A05` decided
and what `doc/todo/02` §2 has said throughout. `cargo` answered that the package ID matched
nothing, the runner reported that no line matched its pattern, and the sequence carried on — so
**the gate was not failing, it was not running**. Corrected, it runs and passes first time: 958
pages compared, 930 agree, 21 differ, 7 refused, raster at 1.39× the CPU backend. Two statements
of one command drift when only one of them is executed; every other package the script names was
checked against `cargo metadata` rather than by eye.

**§5's install loop had been putting September's binaries under `target/`.** The build line
produces `quorra-retrieve`, `quorra-transform` and `quorrafs`; the loop named the three
pre-rename names. It could not fail: cargo removes nothing it no longer produces, so the old
artefacts were still there for `install` to find, and `target/quorra-retrieve` did not exist at
all. `cargo metadata` is now recorded as the authority on that list.

**And three programs introduced themselves by their old names**, `quorra-retrieve` printing
`usage: pdf-retrieve`. The five window binaries already said the right thing, which is why nobody
noticed.

The lint line the script deliberately does not run caught the rest, and the reason it does not run
it is honest: `state.sh`'s whole subject is figures and a silent lint run has none. So a round
reaching for the script owes `cargo fmt --all --check` and the clippy line here — with
`RUSTFLAGS="-D warnings"`, because the workspace's levels are `warn` so an ordinary build stays
usable and CI turns them into errors. A run without the flag is a weaker gate than the one that
gates a push.

`doc/habits.md` gained the two instrument failures of the week: a gate read through a filter that
hides its answer — `state.sh quick` grepped for the words "fail" and "error" instead of its exit
status, `cargo fmt --check` read through `head -5` — and a stale test binary in the shared build
directory answering for a tree that no longer exists, which cost four agents time and which none
of them suspected first, because its failures look like findings.
