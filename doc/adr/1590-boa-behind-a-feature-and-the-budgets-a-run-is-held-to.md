# 1590 — Boa behind a feature in `pdf-script`, and the budgets one run is held to

Status: accepted and **built**. Session 1377. Builds on RFC 0008 sections 5, 6.1, 6.2, 6.8 and 8
and 11 item 3 (a), accepted by the owner in `doc/questions/A193` (answer 3: Boa, QuickJS the named
alternative). ADR 1591 is the bridge over it.
Context: `CLAUDE.md` principle 3 (`#![forbid(unsafe_code)]`, budgets against pathological input, a
C or `unsafe` dependency justified in writing), principle 2 (nothing eager).
Code: `crates/pdf-script/` (new), `Cargo.toml` (`boa_engine`), `deny.toml` (the licence record),
`doc/stack.md` (the row). Tests: `crates/pdf-script/tests/fixtures.rs`, `wire.rs`.

## 1. The engine, and the feature it sits behind

`boa_engine` 0.22.0, `default-features = false` (no `intl`, no `temporal`, no `float16`) plus ECMA-262's
Annex B, because `String.prototype.substr`, `escape` and `unescape` are what form scripts written for
Acrobat call; Annex B adds no package. It is an optional dependency of `pdf-script` behind
`engine`, which **no member turns on**: a build without it has `Request`, `Outcome`, the wire and the
surface, and no engine. `Cargo.lock` lists the engine's 57 packages and seven second versions
(`hashbrown`, `itertools`, `phf` and its three, `synstructure`) whatever the feature, because Cargo
locks every optional dependency; no build without the feature compiles or links one of them.

**Licence first** (habit 67): nine `Unlicense OR MIT`, fourteen `Unicode-3.0`, `ryu-js` `Apache-2.0 OR
BSL-1.0`, the rest `MIT`/`Apache-2.0` — all inside `deny.toml`'s list, so the file gains a paragraph
recording the check and no entry. `cargo deny check`: advisories, bans, licences, sources ok.

## 2. The `unsafe` is the dependency's, counted

`pdf-script`'s own code is `#![forbid(unsafe_code)]`. Boa's is not, and no engine that exists is.
Counted by `grep -rwo unsafe <crate>/src --include=*.rs | wc -l` over the registry sources:
`boa_engine` 293 in 153 266 lines, `boa_gc` 180 in 4 305, `boa_string` 97 in 5 036, `boa_interner`
23, `boa_macros` 9, `boa_parser` 2, `boa_ast` 0 — 604 in Boa's seven crates — and `regress`, the
`RegExp` engine a script's patterns reach, 59 in 45 052. **`cargo geiger` gave no figure**: version
0.13.0 rebuilds the graph in a target directory of its own, and under `tools/bounded.sh --tree 12`
with four jobs it took the agent's task count from 188 to 7 845 and the tree to 13.23 GiB in 39 s
before the bound killed it; an unbounded run of it is the likeliest cause of the session's
out-of-memory kill. The grep count is the figure, and the command is above so that it can be re-taken.
The argument is RFC 0008 section 8 item 2's: the `unsafe` sits where the JBIG2 and JPX decoders'
does under ADR 0014, in a library whose memory safety is the library's, and the guarantee is the
confined process RFC section 6.2 puts it in — RFC section 11 item 4, the next robustness round's.

## 3. One context per run, constructed only when a script runs

`pdf_script::run` constructs a context, installs the bridge, evaluates, reads the event back and drops
it: 0.34 ms for a one-statement script under the gates profile, 0.56 ms in `dev` (fastest of 20,
`fixtures.rs`). Nothing is constructed for a document with no script or a view state with no runner,
so the launch path is unchanged. Nothing persists between triggers; the persistent realm the name
tree and `global` need is the worker's. `Engine` holds a budget and a log and no context, so it is
`Send + Sync` as `ScriptRunner` requires. **No thread is spawned anywhere**: every budget is checked
on the calling thread.

## 4. The budgets (`Budget::FIELD_EVENT`)

| budget | number | where enforced | reason |
|---|---|---|---|
| wall time | 100 ms | the poll loop, between slices | the latency past which a response to a keystroke stops reading as immediate; a field script's real cost is under a millisecond |
| steps | 40 000 000 Boa cost units | the poll loop: slices of 4 096 units, counted | deterministic where the clock is not (trap 36); measured 118–129 ms for 40–50 M under load 9, so about the wall budget on this machine and the one that fires first on a quiet one |
| loop iterations | 100 000 per call frame | Boa's `RuntimeLimits` | the only limit that reaches a loop inside a callback a native makes, where no slice yields; 70 times the 1 418 fields of the census's largest form; `while (true) {}` stops in 2.5 ms (gates), 7.6 ms (`dev`) |
| recursion | 512 | Boa's `RuntimeLimits` | Boa's own default, which its test suite runs under |
| stack | 10 240 values | Boa's `RuntimeLimits` | Boa's own default |
| elements | 1 048 576 | `engine/guard.rs`, before the built-in | 16 MiB of values; 700 times the largest form's fields |
| string units | 16 777 216 | `engine/guard.rs` | 32 MiB; no field value a person reads is near it |
| `ArrayBuffer` bytes | 16 MiB | Boa's `max_buffer_size` hook | ECMA-262's own `RangeError`, which a script may catch: the allocation has not happened |

Boa has **no memory ceiling and no interrupt** (RFC section 5.1). So a script is evaluated with
`Script::evaluate_async_with_budget`, driven by a poll loop with a no-op waker — no async runtime —
that abandons the evaluation where the clock or the steps are spent; and the built-ins an argument
can make large are wrapped before the script runs: every `Array.prototype` method checks `this.length`
(`new Array(5e6)` allocates nothing, `fill` then allocates five million values in one native call),
`concat`, `push`, `unshift` and `Array.from` the length they produce, `join`, `repeat`, `padStart`,
`padEnd` the string. A guard's stop is an engine error, which Boa's catch handling passes over, so
`for (;;) try { … } catch (e) {}` is stopped too. **Judged**: the runaway loop and the five-million
array each end with a named budget, the field unchanged, and a second run in the same process
answering (`fixtures.rs`).

**What none of this bounds, named**: work inside one native call that calls back into script many
times (a `map` over a million elements whose callback loops 99 999 times per call — each frame under
its loop limit, no slice yielding), and growth through an operator (`s += s` thirty times). Both are
the confined process's to bound — its deadline and `RLIMIT_AS`, RFC section 6.8's rows — and until
that process exists the engine runs in no window. A panic reachable in Boa is the same: release
builds abort, so it is the process boundary or nothing.
