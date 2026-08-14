# Session 509 — the variable that emptied the compiler cache

**Finding:** `sccache` was not missing because our crates change every round. Its Rust cache key
contains every environment variable whose name begins with `CARGO_`, and every round of this
generation exports its own `CARGO_TARGET_DIR` — so each round has been writing a private cache that
nothing would ever read. Same source, same warm cache, fresh target directory: **79.01 % of Rust
compilations hit** when the directory is named with `--target-dir` and **0.00 %** when it is
exported, three samples of each.

Date: 2026-08-14. Argued in
[ADR 0344](../adr/0344-the-variable-that-emptied-the-compiler-cache.md). Environment and tooling
round; no code changed.

## Files touched

- `doc/environment.md` — the `sccache` note, replacing the four-hundred-and-eighty-fourth's
  one-line "0.17 % because our crates change" with the mechanism, the rule and the instrument; and
  one clause in the build-directory bullet above it, because that is where a round looks before it
  exports anything.
- `doc/todo/43-the-projects-own-turnaround.md` — a closing section: what the cache reaches, what it
  cannot reach in principle, and the honest state of that file's own denominator.
- `.gitignore` — `/doc/md/` became `/doc/md`. A pattern with a trailing slash matches a directory
  and not a symlink to one, and a worktree round symlinks that path rather than unzipping a second
  copy, so the specifications were showing as untracked in every worktree.
- `doc/adr/0344-…`, this file.

## Where the wrapper is, and is not

- `/home/AI/.cargo/config.toml`: `build.rustc-wrapper` = the absolute path of
  `/home/AI/.cargo/bin/sccache`, plus `build.target-dir` and `build.incremental = false`.
- `~/.cargo/bin` is not on `PATH`, so `which sccache` answers nothing while every build uses it.
  ADR 0264 for the third time; the file already said this about `cargo-fuzz`.
- `/home/cl/projects/render-lib/.cargo/config.toml` overrides only `build.target-dir` and inherits
  the wrapper, so **quorra's checkout already goes through `sccache`** — `cargo build -v` there
  prints it.
- The project owner's account has no `~/.cargo/config.toml`, no `~/.cargo/bin`, no
  `~/.cache/sccache`, and there is no system-wide `sccache`. **The owner's own builds do not use it
  at all**, and the cache lives inside a home directory no other account can read.

## What was measured, and what could not be

Everything against a private server (`SCCACHE_DIR` + `SCCACHE_SERVER_PORT`), because the default one
is shared with every round building at the same time and `--zero-stats` on it destroys their
measurement too. That is now in `doc/environment.md`.

The hypotheses were tested one at a time by replaying one real `rustc` command line with exactly one
thing changed. `--out-dir`, `-L dependency=`, `--extern` into another directory, `LD_LIBRARY_PATH`
and `RUSTFLAGS` all **hit** — so it was never the path, which is also why `trim-paths` was never
going to help and should not be tried a third time. Setting `CARGO_TARGET_DIR` at all, or an
invented `CARGO_ZZZ`, **misses**.

Three further things are uncacheable by construction, and together they are about three-eighths of a
round's compilation: `--crate-type bin` and `--test` (reason `crate-type`, and this workspace links
about 120 test binaries), every workspace-member `clippy` check (reason `multiple input files`,
because cargo composes `sccache clippy-driver rustc …` and the parser sees two inputs), and
incremental compilation (already off, and the reason it must stay off).

**The wall clock is not a signal on this machine and the round says so rather than quoting one.**
Three samples each: 117.3 / 227.7 / 117.7 s with the cache usable against 167.3 / 156.5 / 218.6 s
without it. The spread inside a condition exceeds the difference between the conditions, because
several rounds share twenty-four cores. `time` is no better — `sccache` compiles from its own
daemon, so the CPU time of the `cargo` tree came out the same either way. What is countable is 335
of 780 compiler invocations skipped.

## The two checkouts

Both pin 1.97.1 and share dependencies, but ADR 0222 gave this workspace `[profile.dev]
opt-level = 1, debug = "line-tables-only"` and a profile applies to dependencies too. Against a
cache warmed here, `cargo build --workspace` in `/home/cl/projects/render-lib` hits **7 of 65**
as it stands, **31 of 66** forced to this workspace's whole dev profile, and **33 of 65** with only
`[profile.dev.package."*"]` aligned. The last is the recommendation in ADR 0344 §Recommendations —
offered, not applied, because how quorra compiles its dependencies is a decision about quorra's
debugging.

## What was applied and what was not

Applied: the three documents above. Nothing else — `/home/AI/.cargo/config.toml` is already correct
and other rounds were reading it as this was written, and the owner's account and quorra's checkout
are theirs. The one thing that has to change is not in any file this round owns: it is the
`export CARGO_TARGET_DIR=…` in a round's own setup, which is why the rule went into
`doc/environment.md` where the next round reads it.

## An hour lost to a refused command

The session's setup symlinks `doc/md` and the specification PDFs from the main checkout. The command
that did it was refused by the worktree isolation check for being a compound one, the refusal was
read as belonging to the *other* call in the same message, and the conformance gate failed five
tests an hour later with `doc/md/ISO_32000-2_sponsored_EC3.md: No such file or directory`. **A
refused command produces no output and no error in the thing it was supposed to make**; check the
artefact, not the exit status, and give the isolation check one plain command at a time.

## Gates

`fmt`, `clippy --workspace --all-targets`, `nextest run --workspace`, `test --workspace --doc` and
`test -p conformance` all green — the numbers are `tools/state.sh`'s. No `Cargo.toml` profile
setting was touched, so the corpus and oracle lines were not required and were not run.
