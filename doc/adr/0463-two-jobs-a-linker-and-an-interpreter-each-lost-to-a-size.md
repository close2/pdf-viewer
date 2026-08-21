# 0463 — Two jobs, a linker and an interpreter, each lost to something nobody had measured

Status: accepted.
Session: the six-hundred-and-thirtieth.
Closes the two failing jobs ADR 0450 left; closes the Miri discrepancy 0450 recorded as open;
amends `doc/environment.md`'s `sccache` bullet, `doc/todo/52`'s list of declinations and
`tools/round.sh`'s fifth check; extends ADR 0246 (the Qt host's C++ bridge as a stated expense).

## What happened

The six-hundred-and-fourteenth session fixed four of the five red CI jobs and left two behind, one
of them created by its own fix. This round finished both. Neither was in the tree's *logic*; both
were a **size** — one of a link order, one of a test's input — and both were invisible from this
machine for the same reason, which is the finding worth keeping:

> **A machine that has more installed than the runner does runs a different program.**

## Job 1 — `test` failed at link, and the difference was one absent executable

```
call-initializers.cpp:2: error: undefined reference to 'cxx_qt_init_crate_viewer_qt'
```

`check` passed the whole time, because `clippy` links no binaries; `test` links two.

The chain, each link of it read rather than guessed:

1. `cxx-qt-build` 0.9.1 emits the crate initializer's **definition** into the C++ static library it
   bundles into the rlib, and the **call** to it into a second library linked `+whole-archive`
   (`build_initializers`, and the generated `call-initializers.cpp` is two lines).
2. `rustc` puts that whole-archive library **after** the rlib on the command line. Nothing has
   asked for `cxx_qt_init_crate_viewer_qt` at the moment the rlib is read.
3. A linker that resolves archives in one left-to-right pass therefore never extracts the member
   that defines it. `lld` does not fail, because it keeps every archive member as a lazy symbol
   for the whole link and fetches one when a later reference needs it.
4. `qt-build-utils`' `QtPlatformLinker::init` chooses the first of `lld`, `ld.gold`, `mold` whose
   `--help` it can run — and **GitHub's `ubuntu-latest` image has no `lld` on `PATH`.** So the
   runner links with `gold` and this machine links with `lld`, from the same source, and the two
   are different programs.

**Reproduced here** by building `viewer-qt` with a `PATH` containing every `/usr/bin` entry except
`lld`, `ld.lld`, `lld-link` and `wasm-ld`: the same message, the same file, the same line.

### What was changed, and what it costs

`crates/viewer-qt/build.rs` emits one linker argument on Linux:

```
cargo::rustc-link-arg=-Wl,-u,cxx_qt_init_crate_viewer_qt
```

`-u` enters a symbol as undefined at the start of the link, so the definition is pulled out of the
rlib when the rlib is read rather than skipped. It is a no-op under `lld`. The symbol name is
`cxx-qt-build`'s own `crate_init_key` — `crate_`, the package name, hyphens replaced — derived
from `CARGO_PKG_NAME` rather than written out, so a rename cannot leave it pointing at nothing.

**The alternatives were weighed and rejected, and the reasons are the cost.**

- *Install `lld` in the two CI jobs.* One `apt` line, and it repairs the runner rather than the
  tree — which leaves "this crate needs `lld`, or a linker that is not `gold`" as a machine
  requirement made silently. That is the exact failure ADR 0450 named in this same crate over
  `QPalette::Accent`, and repeating it eight sessions later would be a poor kind of symmetry.
- *Name the initializer from Rust*, so that the undefined reference sits inside the archive that
  defines it. It works and needs no linker argument, but it costs a second hand-written `unsafe`
  token in `viewer-qt` — and `tests/unsafe_position.rs` asserts the count, the file and the line of
  the one that exists, because ADR 0246 made *reviewable* `unsafe` the whole argument for taking
  the C++ bridge at all. One linker argument is cheaper than one `unsafe`.
- *Drop `viewer-qt` from `cargo test --workspace` on CI.* Rejected outright. A job that passes by
  not building the thing is not a fixed job, and what it would stop building is precisely the crate
  the tree's only hand-written `unsafe` lives in.

What the fix does *not* do is repair `cxx-qt-build`'s ordering, which is upstream's; `doc/todo/53`
is where a report belongs if this recurs after an upgrade. The argument is one line long and lives
in the build script beside it.

## Job 2 — `nightly` exceeded the hour, and it was two tests

ADR 0450 removed `--skip flate` from CI's Miri line, which was right — it excluded a third test by
an accident of spelling and could not exclude a second dependency at all — and the consequence was
that tests which had never run began to. 0450 measured `-p pdf-syntax` at 42 minutes of CPU
against CI's former 2 min 39 s, twice mis-attributed it to `sccache`, and wrote it down as
unexplained.

**The instrument that explained it was CI's own log.** `gh run view --job … --log` timestamps every
line, and libtest prints one line per test:

| | at |
|---|---|
| `filter::tests::a_code_the_table_does_not_have_stops_rather_than_guessing … ok` | 19:59:58 |
| `filter::tests::an_lzw_bomb_costs_the_window_rather_than_its_decode … ok` | 20:50:43 |
| the job cancelled, four minutes into the next test | 20:55:06 |

**One test, 50 minutes 45 seconds**, with 57 of the crate's 92 still unrun. Nothing about the
interpreter needed instrumenting: Miri is four orders of magnitude slower than the processor, and
that test decodes an LZW bomb of 7 370 880 bytes — which is what it is *for*.

The second offender is the one CI never reached, found by running the rest here:
`lexer::tests::the_fixed_format_parse_agrees_with_the_standard_library` sweeps 0..=99 999 under
three signs with the decimal point in every position — about 1.8 million lexes of a freshly
allocated string.

### Two tests, two different answers, and the difference is the point

`doc/todo/02` and ADR 0450 both say a test that must not run under the interpreter **declines by
itself, with its reason beside it**, never as a filter in a workflow. That rule says where the
reason goes; it does not say the answer is always a skip.

- **The bomb declines**, `#[cfg_attr(miri, ignore = …)]`, and its doc comment says what that costs:
  one input size and no code path. `an_lzw_pump_and_the_whole_decode_agree` drives the same `Pump`
  over the same shape of bomb under Miri, in windows of one byte to 4096, so every line of the LZW
  decoder is still interpreted — on 60 KB instead of 7 MB. The test itself is unchanged everywhere
  else. And the honest half: what it asserts is a *bound* — the window never grew, the whole route
  refuses above `max_stream_len` — which is a resource question, and `filter.rs` is under
  `#![forbid(unsafe_code)]`, so the interpreter was answering a question nobody asked it.
- **The sweep does not decline; it takes a stride.** `if cfg!(miri) { 997 } else { 1 }`, a prime,
  which keeps every shape the sweep has — one digit to five, all three signs, the point in every
  position and absent — and gives up only the exhaustiveness. That is a claim about arithmetic
  rather than about memory, and the same test still makes it in full on every other gate.

**A declination for cost is a weaker thing than one for a dependency's unsafe**, and the note above
`mod tests` now says which of the four is which, so that the round which finally deletes
`doc/todo/52`'s three does not take this one with them.

## The third thing, and it is why this round could see any of it

`tools/round.sh`'s fifth check — ADR 0450's own, written before any run existed to ask about —
printed **"CI was not asked (no token, or no network)"** for a run that was *in progress*. An
unfinished run has an empty `conclusion`, and an empty `conclusion` was the same case as an empty
answer. The check asks for `status` now and has four outcomes rather than three: green, red with
the command that shows why, **still running**, and not asked. None of the last three is rendered as
green, which was already the rule; what was missing was the ability to tell two of them apart.

## Consequences

- CI's `test` job links under `gold`, which is what the runner has, and under `lld`, which is what
  this machine has; both were run here.
- CI's `nightly` job interprets `pdf-render` and `pdf-syntax` inside the hour, and the two tests
  that no longer run at full size say so themselves. `cargo +nightly miri test -p pdf-render
  -p pdf-syntax --lib` — the workflow's own line, unchanged — runs to the end here, and the
  numbers are in the commit body. All four declinations print their reason as they decline, which
  is what ADR 0450 put them on their tests for.
- `doc/environment.md`'s "that discrepancy is open" is closed, with the instrument named.
- The general rule, which is 0450's with the sign changed: **0450 said a gate here can be weaker
  than the one that gates a push. This round adds that it can also be weaker by having *more*
  installed** — a linker, in this case — and that the way to find out is to take the thing away
  rather than to reason about it.
