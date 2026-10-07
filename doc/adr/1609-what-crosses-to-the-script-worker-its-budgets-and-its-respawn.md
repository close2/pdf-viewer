# 1609 — What crosses to the script worker, the budgets it is held to, and its respawn

Status: accepted and **built**. Session 1386. Builds on RFC 0008 sections 6.2, 6.6, 6.7 and 6.8
(accepted, `doc/questions/A193`), ADR 1590 (the budgets one run is held to), ADR 1602 (one realm
per document) and ADR 1608 (the program and its profile).
Code: `crates/pdf-script-worker/src/wire.rs`, `client.rs`, `worker.rs`,
`crates/pdf-sandbox/src/lockdown_linux.rs` (`SCRIPT_ADDRESS_SPACE_LIMIT`),
`crates/pdf-script/src/engine/guard.rs` (the `repeat` stop), `fuzz/fuzz_targets/script.rs`,
`fuzz/fuzz_targets/script_wire.rs`. Tests: `crates/pdf-script-worker/tests/end_to_end.rs`;
instruments `examples/script_peak.rs`, `examples/wire_seeds.rs`.

## 1. What crosses

A **run** carries the script the worker does not yet hold — its text and the index the host gave
it — the index to run, and `pdf_script::wire`'s own request with its script empty: the site, the
event's fields, the fields the realm is told of, the page, the moment. A **reply** is
`pdf_script::wire`'s outcome, or a refusal's sentence. The worker never holds the document's bytes.

**Each script crosses once per worker, at its first run, not every script at open.** RFC 0008
section 6.2 has the scripts cross once, at open; the runner a view state hands events to sees each script with its
event, and gathering every script of a document before page one would be the eager work principle 2
forbids. A keystroke script fired per key crosses once. Every field crosses to a new worker once,
whole, and after that only the fields an event says changed. A worker started after a loss is told
every field and runs every document-level script again, in the order first run, its edits set
aside because they were made the first time.

## 2. The ceiling, measured: 96 MiB

`script_peak` (release, each script in a fresh process): a process that links the engine starts at
12.4 MiB (17.6 in debug); one statement costs 0.4 MiB more. One call at each of ADR 1590's per-call
budgets: an `ArrayBuffer` of 16 MiB +16.4 MiB, an array of 2^20 filled +8.5, `padStart` to 2^24
units +96.4, `join` over 2^20 elements +150.4 (160 ms in one native call), a typed array's `join`
at the byte budget +144.4 (372 ms), and a string doubled twenty-six times by `+=` +96.5 — which no
per-call budget sees. **The per-call budgets admit single calls of a hundred and fifty mebibytes**,
so a ceiling that admitted every one would be no bound on a script worker. 96 MiB holds what the
worker may keep whatever it runs — about 43 MiB in debug: the image and the engine as measured, and
three bounds of this crate's own, a run's frames and their copies (8), the scripts held (8) and a
stack grown to its 8 MiB — beside the largest allocation a budget admits at face value; a call whose
transient cost multiplies its budgeted size meets the ceiling and costs one named worker.
`Budget::realm_bytes` (256 MiB), which ADR 1602 added, is not what the kernel installs; the two
numbers should be one, and this ADR's is the measured one.

**One budget escape fixed.** `'x'.repeat(16777216)` is inside the string budget, and Boa reserves
room for every repetition before it charges the first to the loop-iteration limit: 384 MiB, then the
limit's stop at the 100 001st. A count past the limit is now stopped before the call with the limit
Boa would have named (`guard.rs`): the same script peaks 0.4 MiB above start, and `end_to_end.rs`
holds it in the worker.

## 3. The deadline per trigger: 250 ms

A run the engine's own budgets stop answered at worst 103 ms after the exchange began in debug and
76 ms in release, slowest of ten at a load of four; a run that finishes answers in one or two. 250
ms is more than twice the slowest and short of what a person typing reads as a hang. A watchdog
thread on the host side cancels the worker — `Canceller::cancel`, a `SIGKILL` — when the deadline
passes; no clock runs in the worker for it.

## 4. A loss, named, and replaced at the next trigger

A deadline, a death (the ceiling's abort, a `SIGSYS`, a stack overflow) or a reply that does not
decode loses the worker, and the report says which trigger: *the format script of Total did not
finish and changed nothing: its worker had not answered after 250 ms and was stopped*. The next
trigger starts another. After four losses in one document no worker is started again and every
trigger is answered *scripts stopped running for this document*. There is no pool, so ADR 1554's
`MALLOC_ARENA_MAX=1` asks nothing of this worker; `Host::start` sets it for every worker anyway.

**Boa 0.22's parser has no depth limit**: five hundred nested parentheses or array literals overflow
the worker's 8 MiB stack in a release build, three thousand `!`, `if (1)` or `{`. In the worker that
is one named loss, held by `a_script_nested_past_the_parsers_depth_is_contained`. In process — the
`Engine` stand-in, a fuzz target — it is the process. The `script` target therefore runs on a
64 MiB stack and does not run bracket nesting past 256, and the defect is the upstream's to bound.

## 5. `script_open`

`OpenCost::line()` prints `script_open spawn_ms=<f> first_run_ms=<f>`: the spawn through the
greeting (1.8–2.1 ms) and the first exchange, realm construction included (1.1–1.7 ms). Both happen
at the first trigger, after the first present, so neither is on the launch path; the launch gate's
timeline takes the line as its own (RFC 0008 section 6.6).

## 6. The fuzz targets

`script` runs `Realm::run` at one of eight sites over a script, a value and a change; `script_wire`
decodes a run and a reply from the same bytes. Each ran twenty minutes from fresh seeds, one process,
built `-s none`, behind the heavy-walk lock: `script`, 1280 seeds from `fuzz/seed_script.py`,
INITED 12 187 edges, DONE 20 438 after 778 378 runs, no artefact; `script_wire`, 29 seeds from
`examples/wire_seeds`, INITED 548, DONE 1228 after 1 271 175 166 runs, no artefact.
