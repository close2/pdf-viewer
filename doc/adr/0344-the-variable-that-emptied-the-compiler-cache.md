# ADR 0344 — The variable that emptied the compiler cache

Status: accepted
Date: 2026-08-14
Supersedes nothing. Amends `doc/environment.md`'s `sccache` note, which said the cache was
missing because "this workspace's crates change on nearly every round".

## The question

`sccache` has been the `rustc-wrapper` for user `AI` since the four-hundred-and-eighty-fourth
session and has been reading a hit rate near zero ever since. The note in `doc/environment.md`
blamed our own churn. That explains why *our* crates miss; it does not explain why `kurbo`,
`smallvec` and eight hundred other unchanged third-party compilations miss, and nobody had asked.
The project owner asked, and added a second half: make it work for this repository **and** for
quorra's checkout at `/home/cl/projects/render-lib`.

## Where the wrapper is, and is not

- `/home/AI/.cargo/config.toml` sets `build.rustc-wrapper` to the absolute path of
  `/home/AI/.cargo/bin/sccache`, `build.target-dir`, and `build.incremental = false`.
- That directory is not on `PATH` (`/usr/local/sbin:/usr/local/bin:/usr/bin`), so `which sccache`
  answers nothing while every build this user runs goes through it. ADR 0264's lesson, a third time.
- `/home/cl/projects/render-lib/.cargo/config.toml` overrides only `build.target-dir`. Cargo merges
  the rest from the home config, so **quorra's checkout already uses the wrapper** — confirmed by
  `cargo build -v` there printing `…/sccache …/rustc`.
- The project owner's own account has no `~/.cargo/config.toml`, no `~/.cargo/bin`, no
  `~/.cache/sccache`, and there is no `sccache` in `/usr/bin` or `/usr/local/bin`. **The owner's
  builds do not use it at all.** The cache is one user's, under `/home/AI/.cache/sccache`, inside a
  home directory no other account can read.

## What was measured

Everything below was run against a *private* server —
`SCCACHE_DIR=… SCCACHE_SERVER_PORT=… sccache --start-server` — because the default one is shared
with every round building at the same time and `--zero-stats` on it would have destroyed their
numbers along with the noise in ours.

### 1. The cache key contains every `CARGO_*` environment variable

One `rustc` command line, replayed through the wrapper with exactly one thing changed at a time:

| changed | result |
|---|---|
| `--out-dir` | **hit** |
| `-L dependency=…` | **hit** |
| `LD_LIBRARY_PATH` | **hit** |
| `--extern …` pointing into a different directory, same file contents | **hit** |
| `RUSTFLAGS`, or any variable not named `CARGO_…` | **hit** |
| `CARGO_TARGET_DIR` set at all, or changed | **miss** |
| an invented `CARGO_ZZZ`, set at all, or changed | **miss** |

So it is not the absolute path — paths are handled, and `--extern` is keyed on the *contents* of
the library rather than on where it sits. It is the variable. `sccache` folds the `CARGO_*`
environment into the Rust key because that is where `cargo` puts things a compilation can depend
on, and it cannot tell which of them the compiler actually read.

This is why `trim-paths` was never going to help, and it is worth writing down so the four-hundred-
and-ninety-second session's attempt is not repeated for a third reason: the paths were never the
problem. (It also does not parse on the pinned stable `cargo` 1.97.1.)

### 2. What that costs, on this workspace

`cargo build --workspace --all-targets` into a **fresh** target directory, identical source,
identical warm cache, three samples of each condition:

| the target directory named | Rust hits | wall clock |
|---|---|---|
| `--target-dir` on the command line | **335 / 424 = 79.01 %**, three times | 117.3, 227.7, 117.7 s |
| `CARGO_TARGET_DIR` exported | **0 / 424 = 0.00 %**, three times | 167.3, 156.5, 218.6 s |

**The hit rate is exact and reproducible; the wall clock is not, and no second count is claimed
from it.** This machine has twenty-four cores and several rounds building on them at once; the
spread inside each condition is larger than the difference between the conditions, and principle 2
says a difference smaller than the spread is not a difference. What is countable is the work
skipped: **335 of 780 compiler invocations**, at a measured average compile of 4.6–6.3 s. Somebody
who wants the seconds must take them on a quiet machine.

(`time` is no instrument here either: `sccache` runs cacheable compilations from its own daemon, so
the CPU time of the `cargo` process tree is the *same* in both conditions — 596 s against 603 s —
and measures only the compilations that never reach the cache.)

### 3. Three-eighths of a round's compilation cannot be cached at all

From the same builds, and from single-purpose probes:

- **`--crate-type bin` and `--test` are refused outright**, reason `crate-type`. Two identical
  `bin` compiles: two non-cacheable calls, no hit, no miss. This workspace links about 120 test
  binaries plus the gate binaries and examples, so 340 of the 356 non-cacheable calls in one
  `--all-targets` build are this, and it is the largest single reason on the shared server too.
- **`cargo clippy` on a workspace member is refused**, reason `multiple input files`. With both
  wrappers in play cargo composes `sccache clippy-driver <path-to-rustc> --crate-name …`, and
  `sccache`'s argument parser sees two non-flag arguments where a compilation has one. Verified
  from `cargo clippy -v`. Dependencies under `clippy` are plain `rustc` and are cached normally.
- **Incremental compilation is refused**, reason `incremental`. It is already off
  (`build.incremental = false`, and `debug/incremental/` is empty after a full build), so it is not
  a present cause — but it is the one setting that would silently undo everything here, and the two
  choices are coupled: incremental is worth a great deal in a *persistent* target directory and
  worth nothing in a fresh one, which is exactly the case `sccache` is for.

### 4. The two checkouts share almost nothing, and the reason is the dev profile

`cargo build --workspace` in `/home/cl/projects/render-lib`, against a cache warmed by this
workspace's dev build. Both trees pin the same compiler (`rust-toolchain.toml`, 1.97.1) and share
dependencies:

| quorra's dependency profile | Rust hits |
|---|---|
| as it stands — no `[profile.dev]`, so `opt-level = 0`, `debug = 2` | 7 / 65 = **10.8 %** |
| forced to this workspace's whole `[profile.dev]` | 31 / 66 = **47.0 %** |
| only `[profile.dev.package."*"]` aligned | 33 / 65 = **50.8 %** |

ADR 0222 gave this workspace `[profile.dev] opt-level = 1, debug = "line-tables-only"`, and a
profile applies to dependencies as well as to our own crates. Two projects that build `smallvec`
with different `-C opt-level` are compiling different things, correctly, and no cache can join
them.

## The decision

1. **The target directory is named on the command line or in a `.cargo/config.toml`, never
   exported.** `--target-dir <dir>` and `build.target-dir` are both invisible to `sccache`;
   `export CARGO_TARGET_DIR=…` is not. This is free, it needs no coordination, and it is the whole
   difference between 0 % and 79 %.
2. **`build.incremental = false` stays**, and its cost is written down here rather than
   rediscovered: a round that builds twice into the *same* directory loses incremental rebuild and
   gains nothing, because `sccache` cannot cache an incremental compile. The setting is right for
   the way rounds actually build — a fresh directory each time — and would be wrong for a
   long-lived one.
3. **Nothing is changed outside this repository.** `/home/AI/.cargo/config.toml` is already correct
   and other rounds are reading it as this is written; the owner's account and quorra's checkout are
   theirs. What is offered instead is §Recommendations.
4. **The uncacheable three-eighths is accepted, not worked around.** Binaries, test harnesses and
   `clippy` are outside `sccache` by construction, and the answer to them is `doc/todo/43`'s — fewer
   and cheaper links — not a different cache.

## Recommendations, for the project owner to apply or decline

- **Quorra, if cross-project reuse is wanted** — in `/home/cl/projects/render-lib/Cargo.toml`:

  ```toml
  # Third-party dependencies built the way pdf-viewer builds them, so that one sccache
  # entry serves both checkouts. Quorra's own crates keep opt-level 0.
  [profile.dev.package."*"]
  opt-level = 1
  debug = "line-tables-only"
  ```

  Measured at 50.8 % against 10.8 %. It changes how quorra's dependencies are compiled, which is a
  decision about quorra's debugging, and is therefore not ours to take.
- **A cache the owner's account can use.** There is none today. `sccache` is installed in one
  user's `~/.cargo/bin` and caches into one user's home. A cache shared by both accounts needs
  `SCCACHE_DIR` somewhere group-writable by `coders` with `umask 002` — the same argument that put
  the build directories where they are — and it is worth having only if the owner builds here often.
- **Do not retry `trim-paths`.** §1 above; it addresses a problem this cache does not have.

## Consequences

- `doc/environment.md`'s note now states the mechanism and the rule, and names
  `sccache --show-stats`'s *categories* as the instrument rather than its headline rate.
- `doc/todo/43` gains the round-turnaround half of this: what the cache can and cannot reach.
- A round that reads "0 % hits" no longer has an explanation ready that happens to be wrong. The
  older explanation was not false — our crates do change every round — it was just not the reason
  the third-party half of the graph was missing, and a plausible cause found first is how a real
  one stays hidden.
