# 630 — The linker one machine had, and the fifty minutes nobody had looked at

The two CI jobs the six-hundred-and-fourteenth session left red. Neither was in the tree's logic;
each was a **size** — one of a link order, one of a test's input — and each was invisible here
because this machine has more installed than the runner does.

Date: 2026-08-21.
ADR: [0463](../adr/0463-two-jobs-a-linker-and-an-interpreter-each-lost-to-a-size.md).

Touched: `crates/viewer-qt/build.rs`, `crates/pdf-syntax/src/filter.rs`,
`crates/pdf-syntax/src/lexer.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/tests/tiling.rs`, `tools/round.sh`, `doc/environment.md`, `doc/todo/52`,
`doc/conformance/ledger.toml` (§8.7.3), the ADR and this file.

## Job 1 — `test` failed at link, and the difference was one executable the runner does not have

```
call-initializers.cpp:2: error: undefined reference to 'cxx_qt_init_crate_viewer_qt'
```

`cxx-qt-build` 0.9.1 puts the initializer's **definition** in the C++ static library bundled into
`viewer-qt`'s rlib and the **call** to it in a second library linked `+whole-archive`, and `rustc`
places the second **after** the first. A linker that resolves archives in one left-to-right pass
has walked past the definition before anything asks for it. `lld` has not, because it keeps archive
members as lazy symbols for the whole link.

`qt-build-utils` picks the first of `lld`, `ld.gold`, `mold` it can run, and **GitHub's
`ubuntu-latest` has no `lld` on `PATH`**. So the runner links with `gold` and this machine links
with `lld` — the same source, two programs.

**Reproduced here**, byte for byte, by building `viewer-qt` with a `PATH` holding every `/usr/bin`
entry except `lld`, `ld.lld`, `lld-link` and `wasm-ld`. The fix is one linker argument emitted by
`crates/viewer-qt/build.rs` on Linux — `-Wl,-u,cxx_qt_init_crate_viewer_qt`, the symbol derived
from `CARGO_PKG_NAME` the way `cxx-qt-build` derives it — which enters the symbol as undefined at
the start of the link so the rlib's member is extracted when the rlib is read. It is a no-op under
`lld`. Verified both ways here: the gold-only `PATH` builds `--all-targets`, and the ordinary
`PATH` still links with `lld` (`.comment` says which).

**What it costs, and what the two rejected alternatives would have cost**, is in ADR 0463: an
`apt install lld` would have repaired the runner and left "this crate needs a linker that is not
`gold`" as a machine requirement made silently — 0450's own finding, eight sessions later; and
naming the initializer from Rust would have cost a second hand-written `unsafe` token in the one
crate whose `unsafe` position `tests/unsafe_position.rs` asserts by file and line. Dropping
`viewer-qt` from `cargo test --workspace` was rejected outright.

## Job 2 — `nightly` exceeded the hour, and CI's own log said where it went

614 recorded 42 minutes of CPU as unexplained and twice blamed `sccache`. The answer needed no
instrument here at all: `gh run view --job … --log` timestamps every libtest line.

`filter::tests::an_lzw_bomb_costs_the_window_rather_than_its_decode` finished at 20:50:43 against
19:59:58 for the test before it — **50 minutes 45 seconds for one test**, with 57 of the crate's 92
still unrun when the job was cancelled four minutes later. It decodes an LZW bomb of 7 370 880
bytes, which is what it is for, and Miri is four orders of magnitude slower than the processor.

A second offender, which CI never reached and which running the rest here found:
`lexer::tests::the_fixed_format_parse_agrees_with_the_standard_library`, about 1.8 million lexes of
a freshly allocated string.

**Two tests, two different answers, and the difference is the point.** The bomb declines —
`#[cfg_attr(miri, ignore = …)]` — and its doc comment says the cost is one input size and no code
path: `an_lzw_pump_and_the_whole_decode_agree` drives the same `Pump` over the same shape of bomb
under Miri in windows of one byte to 4096, so every line of the decoder is still interpreted, on
60 KB instead of 7 MB. The sweep does *not* decline; it takes a prime stride under Miri, keeping
every shape it has — one digit to five, three signs, the point in every position and absent — and
giving up only exhaustiveness, which is a claim about arithmetic rather than about memory.

`doc/todo/52` now says which of `filter.rs`'s four declinations is not `zlib-rs`', so the round
that deletes the three does not take this one with them.

## `tools/round.sh`'s fifth check, on its first run against a live pipeline

It printed **"CI was not asked (no token, or no network)"** for a run that was *in progress*: an
unfinished run has an empty `conclusion`, and an empty `conclusion` was the same case as an empty
answer. It asks for `status` now and has four outcomes — green, red with the command, still
running, not asked — and all four were exercised.

## The spec-driven half — §8.7.3, and the route the report was not on

The row chosen by 620's rule: its stated reason is a claim about *this codebase* rather than about
the standard — "a **stroke** whose colour is a tiling pattern … there is no path here to tile",
with one corpus document named as reported rather than silently stroked.

The claim was true of `path.rs` and false of the tree. **`stroke_glyph` had no such report**, so
§9.3.6's rendering modes 1, 2, 5 and 6 outlined a glyph in whatever solid colour was last set and
said nothing — with §8.7.2's "All patterns shall be treated as colours" making a glyph's stroke
colour exactly a path's, which is the sentence the fifty-third session already used to fix the
*fill* half on this same route. It is the shape §8.7.4's row calls "a rule about the parent needs a
test per way of becoming one", with *route* in the place of *parent*.

Reported on both routes now, with
`a_glyph_stroked_in_a_tiling_pattern_is_reported_rather_than_stroked_in_the_last_colour`, which was
watched to fail without the four-line change and pass with it — 620's other rule, that a row's
evidence has to reach what the row claims.

## Gates

The whole of `doc/todo/02` §2, since this is a fifth round, and §5's binaries after it. The
numbers are in the commit body.

**The machine carried three other rounds throughout, at a load average between 10 and 127**, which
is stated because one number here is a duration: `cargo +nightly miri test -p pdf-render
-p pdf-syntax --lib`, CI's own line, ran to the end. A loaded machine can only make that number
*worse* than the runner's, so it is the safe direction to be wrong in — and the one cross-check
available says the two machines are close: `pdf-render` finished in 174.37 s here against CI's
173.97 s on 2026-08-20. Nothing else measured was a timing.

**One honest gap in that run.** It interpreted the sweep with the stride form the sample was first
written as — about 101 values, 64 s of the 523.80 s — and the committed form adds the first hundred
so that one and two digits are present too. That is roughly a thousand extra lexes of the cheapest
strings in the set, on top of eighteen hundred. It was **not** re-measured, and the reason is worth
the sentence rather than a number: by the time the edit was final the machine's load average was
past five hundred, and a duration taken there is not a measurement of anything. What the sweep
costs is bounded by construction; what the job has is an hour.

## What could not be proved

**No CI run.** The token here is read-only for contents — `git push` answers 403, the refs API
answers "Resource not accessible by personal access token", and `workflow_dispatch` is offered only
for a workflow already on the default branch. So neither fix has been seen green on a runner; what
stands behind job 1 is a local reproduction of the failure and of the fix under the runner's
linker, and behind job 2 the interpreter's own end-to-end run of CI's exact invocation. The merge
that lands this is the first thing that can produce a run, and it owes a look at what it triggers.
