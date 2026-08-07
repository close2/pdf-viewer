# 0222 — A round is ten minutes, and eight of them are the machine's

Status: accepted, three-hundred-and-eighty-fifth session.

## The question

The project owner asked why a round takes so long, and suspected the tests. Principle 2's rule
decides how to answer it: *"genuinely" is decided by measurement, never by assumption.* So the
first half of this round measured, and the second half changed only what the measurement named.

The unit measured is what a round actually does: **`doc/todo/02-every-round.md` §2's gate
sequence followed by §5's binaries, from a warm tree with one file in `pdf-model` touched.** A
cold build is a fresh clone; a warm no-op is nothing; the incremental rebuild after an edit is
the case a round is made of, and it is the only one worth optimising.

## What a round cost, and where

One sample of the whole sequence at `deff3f3`, on 24 cores, `touch
crates/pdf-model/src/lib.rs` first. Every number below was printed by `date +%s.%N` around the
command, not estimated.

| step | HEAD | this round |
|---|---:|---:|
| `cargo fmt --all --check` | 0.8 | 0.7 |
| `cargo clippy --workspace --all-targets` | 3.3 | 3.3 |
| the workspace's tests, **compile** | 11.9 | 5.2 |
| the workspace's tests, **run** | **235.7** | **21.9** |
| the doctest `cargo test --workspace` also runs | — | 1.4 |
| `pdf-sandbox`'s worker | 0.1 | 0.1 |
| `viewer-confined`'s two binaries, before the gates | 31.4 | — |
| `pdfref-hayro`, the oracle's fourth reading | — | 11.3 |
| corpus — compile / run | 29.2 / 4.3 | 8.3 / 3.6 |
| oracle | 22.7 / 25.7 | 6.5 / 25.5 |
| text extraction | 16.4 / 30.4 | 5.1 / 31.2 |
| dates | 7.1 / 0.6 | 3.1 / 0.6 |
| XMP | 7.9 / 0.4 | 3.4 / 0.4 |
| JPEG 2000 | 7.4 / 9.2 | 3.7 / 9.1 |
| quorra against the CPU oracle | 52.6 / 29.4 | 13.0 / 29.5 |
| conformance | 0.1 / 3.6 | 0.1 / 1.7 |
| §5's three release binaries | 77.7 | 79.3 |
| **total** | **607.9 s** | **268.0 s** |
| of which compilation | 267.8 | 142.4 |
| of which execution | 340.1 | 125.6 |

**2.27× — ten minutes to four and a half.** Two samples of the finished sequence, **268.0 s and
266.6 s**; the individual steps have two to four samples apiece and none of the differences claimed
here is smaller than the spread between them, which for the gate *runs* is about half a second.

Three other builds, for the record, in a target directory of their own so they were genuinely
cold:

| | HEAD | this round |
|---|---:|---:|
| cold `cargo test --workspace --no-run` | 50.3 s | 86.4 s |
| cold, the six `pdf-model` gate binaries | 51.4 s | 32.6 s |
| cold, `render-quorra`'s corpus gate | 73.8 s | 33.9 s |
| warm no-op `cargo build --workspace --all-targets` | 0.42–0.47 s | unchanged |

### What the linker spends

`cargo build --release --bin pdf-viewer --timings` after touching one file in `pdf-model`: the
whole build is 76.9 s and **66.4 s of it is one unit**, `viewer-ui`'s binary. That unit is fat
link-time optimisation over one code generation unit, and its `user` time equals its `real` time
— one thread inside LLVM for 86% of the wall clock, on a machine with 24 of them. The same shape
holds for every gate binary, and there were eight of them.

## What was changed, and what each buys

### 1. A profile for the gates

Every corpus gate ran under `[profile.release]`, which is `lto = "fat"` with `codegen-units = 1`
over a graph containing wgpu, vello and quorra. `[profile.gates]` inherits release and sets
`lto = "thin"`, `codegen-units = 16`: still cross-crate optimisation, still inlining across the
graph, but with work the machine can do in parallel.

| after touching one file in `pdf-model` | `release` | `gates` |
|---|---:|---:|
| the six `pdf-model` gate binaries, as the round builds them | 90.7 s | 30.1–31.2 s |
| `render-quorra`'s corpus gate | 52.6 s | 12.9 s |
| `viewer-confined`'s two binaries | 31.4 s | 8.1 s |

`[profile.release]` is untouched, and `target/pdf-viewer` and its two companions are still built
with it. That is not conservatism: those are what a person runs and what every launch measurement
in this repository is taken from.

**The claim this needed was that the gates say the same thing, and it was checked rather than
argued.** All eight ran under each profile, back to back, and their output was compared line by
line with wall clocks and target paths removed:

| gate | lines compared | verdict |
|---|---:|---|
| corpus | 98 | identical |
| oracle | 897 | identical |
| text extraction | 27 | identical |
| dates | 25 | identical |
| XMP | 29 | identical |
| JPEG 2000 | 35 | identical |
| quorra | 60 | identical |
| conformance | 515 | identical |

That is 1794 oracle page verdicts, 957 quorra pages, 974 corpus documents and 4990 citations,
every one of them the same number under both. The only lines that moved were the clocks, which
is what the comparison was for: codegen *can* move a float, and on this workspace it did not.

**And the comparison found something that had nothing to do with the profile.** Under `gates` the
oracle's reference-render count dropped by 861 with every verdict unchanged, which is
`Reference::Hayro` silently unavailable: `pdfref-hayro` is a *program*, found beside the running
binary, and nothing in `doc/todo/02` §2 has ever built it. The release copy existed only because
some earlier session ran `cargo build --release -p hayro-compare --bins`. So the fourth reading —
which never votes, but is what a person looks at on a page the three references cannot settle —
had been an accident of a long-lived target directory. It is now a line in §2, and with it the
oracle's output is identical field for field, 6173 cached and 16 produced either way.

The cost of the second profile is written down: **1.1 GB on disk** and **66 s of compilation,
once**, for object code `release` had already produced. And the gates now measure a
differently-optimised program from the one that ships, which would be intolerable if they
measured speed. They do not. Everything that takes a *timing* — `cargo bench`, the callgrind
examples, the launch measurements — stays on `release` and `bench`.

### 2. `cargo nextest` for the workspace's tests

The largest single item in the round was `cargo test --workspace` at **235.7 s**, and the reason
is structural: `cargo test` runs one test binary at a time, and this workspace has 118 of them.
The machine sat idle between them. `cargo nextest` runs every test in one global pool.

**235.7 s → 76.6 s**, on the same binaries — nextest reuses what `cargo test --no-run` built and
compiles nothing extra. What it does not run is doctests, of which this tree has exactly one, so
the gate is `cargo nextest run --workspace` **plus** `cargo test --workspace --doc`: 1308 tests
and one doctest, which is the same **1309** `cargo test --workspace` reports, with the same 9
ignored.

It is a user-local install (`~/.cargo/bin`), and `cargo test --workspace` remains exactly the same
gate for anyone without it — which is why CI keeps it. CI runs on two cores, where nextest's
whole advantage is small, and adding a tool to the job that gates the snapshot release buys
little and risks something.

### 3. `opt-level = 1` and `debug = "line-tables-only"` on the dev profile

With the binaries running in parallel, the test gate's wall clock became **one test**:
`render-cpu::strip_parallelism a_page_drawn_in_strips_is_the_page_drawn_whole`, 71.6 s of a
79.1 s run, which draws each of its scenes whole and again at 2, 3, 5, 8 and 16 strips —
six rasterisations of a real page apiece, with an unoptimised rasteriser.

`opt-level = 1` takes the whole run from **76.6 s to 20.0 s**. It is not a semantic change and
that matters more than the number: `dev` keeps `debug-assertions` and `overflow-checks` on —
they are their own profile keys, and Cargo does not touch them when `opt-level` moves — so an
arithmetic overflow still panics and every `debug_assert!` still fires. All 1309 pass either way.

`debug = "line-tables-only"` halves what the linker copies into 120 test binaries: an incremental
`cargo test --workspace --no-run` falls from **12.9 s to 7.0 s** (three samples apiece), and a
clean dev tree from **17 GB to 8 GB**.

The two costs, both measured rather than supposed:

- **A cold dev build goes 40 s to 86 s.** That is a fresh clone and a swept target directory, not
  a round; the *incremental* case did not pay for it at all and came out slightly cheaper,
  7.0 s to 5.5 s, because optimised code is smaller code to link 120 times.
- **A backtrace is less precise.** Line tables keep file and line, which is what this project
  reads when a test fails, but inlining at `opt-level = 1` merges frames, and a debugger can no
  longer show local variables. `[profile.bench]` sets `debug = true` on its own line, so
  profiling and the callgrind counters are untouched.

### 4. Two orderings, worth about 30 s between them

`cargo build --release -p viewer-confined --bins` sat in front of the gates, on the note that it
was needed "for `viewer-confined`'s tests". Those tests run under `cargo test --workspace`, which
builds the debug worker itself; **no release or gates binary in this tree names
`viewer-confined`**, checked by grep over `pdf-model`'s and `render-quorra`'s manifests and test
sources. It belongs in §5 with the other two binaries, where §5's three separate invocations
became one — three fat links running beside each other rather than one after another,
**109.7 s → 79.3 s**.

And `pdfref-hayro`'s build goes *after* the corpus gate rather than in front of it, which is
worth 7 s reproducibly (26–28 s against 19–20 s over two samples each): `cargo build -p
hayro-compare` compiles `pdf-model`'s rlib with nothing to overlap it, where `cargo test --test
corpus` compiles that rlib and its test target in one graph. Each build now sits with the gate
that reads it, which is also the more legible arrangement.

## What was measured and *not* done

### The owner's own suggestion: cache our renders

Reference renders are already cached (ADR 0020) and the oracle reports **6173 of 6189 from the
cache, 99.7%**. Our own renders are not cached, and this round did not build one. Three
measurements say why:

- **The oracle's floor is one page.** Its wall clock is 24.5–25.7 s, and the slowest single page,
  `22060_A1_01_Plans.pdf` page 1, is 8.6–9.8 s of it. A third of the gate is one document, and a
  cache saves nothing on the run that has to produce it.
- **The key would have to cover the code, and the code is why the gate is running.** Trap 10a is
  this project's own record of a stale cached render, and the rule it earned is that a key which
  cannot lie must name every input — for our own output that includes the binary that produced
  it. That binary is relinked on every round that touches `pdf-model`, which is every round that
  needs the gate. The cache would miss exactly when it is wanted and hit only on a re-run of an
  unchanged tree, which is a run that need not happen.
- **What is left is not where the time is.** After this round the eight gate *runs* are 101.6 s
  of 268.0, and the round's compilation is 142.4. A cache with an honest key could not reach the
  first and does not touch the second.

Written down in `doc/todo/43` so nobody prices it again.

### A faster linker, and `cargo clean`

`lld` is on this machine and was not adopted: with `debug = "line-tables-only"` and
`opt-level = 1` the dev link is 5.2 s for 120 binaries, and the release link is LLVM's LTO rather
than the linker's symbol resolution — `--timings` puts 66.4 s of a 76.9 s build inside one
codegen unit, where a linker swap cannot reach. It stays on the list in `doc/todo/43` as
something to measure again if the dev compile ever grows.

`/home/AI/cargo-target/pdf-viewer` was **311 GB** when this round began and **334 GB** by the end
of its measurements, and the interesting part is the ratio: a *clean* tree measured in a fresh
target directory is 17 GB of dev artefacts and 1.1 GB per release-grade profile. Essentially all
the rest is superseded output, which Cargo on stable has no command to remove — `cargo clean --gc`
is nightly-only. Swept by hand this round: `debug/`, `release/` and `gates/` deleted, **334 GB →
8.1 GB**, with `tmp/pdfref-cache` — **1.5 GB**, and about a thousand seconds of `pdftoppm`,
`mutool` and `gs` if it were lost — deliberately not touched, which is why the sweep names
subdirectories instead of running `cargo clean`. The procedure is `doc/todo/02` §5a with the cost
attached, measured on the swept tree: one cold build, **87.9 s** of
`cargo test --workspace --no-run` and **97.2 s** of the whole gates profile, with `release` on top
of that only when §5 runs. It is
disk hygiene rather than speed — the warm no-op build was 0.42 s with the directory at 311 GB.

## Consequences

- `doc/todo/02-every-round.md` §2 and §5 and `doc/HANDOVER.md`'s "Verify it" all name
  `--profile gates` and `cargo nextest`. A round run with the old commands still passes; it is
  the same gates, more slowly, against a `release` tree that is still built for §5 anyway.
- CI is unchanged and still coherent: it never ran the `--ignored` corpus gates, so no job on it
  uses the `gates` profile, and `cargo test --workspace` is still the test gate there.
- `cargo deny` clean on all four checks with no dependency moved; both cross-target checks build
  under `-D warnings`.
- The one thing to watch is the second profile's *staleness*, which is trap 10 wearing a new
  coat: `target/gates/` holds a sandbox worker and a `pdfref-hayro` that Cargo will not rebuild
  for another package's sake. Both are lines in §2 for that reason.
