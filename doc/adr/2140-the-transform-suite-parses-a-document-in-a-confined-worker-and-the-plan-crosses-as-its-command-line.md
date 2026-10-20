# 2140 — The transform suite parses a document in a confined worker, and the plan crosses as its command line

Session 1652, slot 6 of batch one hundred and one, a host round. Status: **accepted** and **built**
in `crates/pdf-transform/src/confined.rs`, `src/bin/quorra-transform-worker.rs`,
`src/bin/quorra-transform/{route,here}.rs` and `crates/confined-transport` (two additive pairs).
Context: `doc/questions/Q02` and `A02` ("Yes, implement it "now"", 2026-09-06); ADR 0800 §6, which
stated the cost this removes; ADRs 0014, 0812, 0847, 0880, 1804; RFC 0002 §13 question 3;
`doc/todo/57` §4. ADR 2141 is the fences the broker answers through.

## 1. What was true before

`quorra-transform` parsed, interpreted and decoded every document in its own process, with the
user's filesystem and network: ADR 0800 §6's "known cost, stated plainly". `pdf-vfs-worker` already
ran `pdf_transform::apply` confined, but only for the questions a mount asks, not for the suite.

## 2. The decision

**A worker by default, the in-process route a fallback the person selects, said aloud.**
`quorra-transform` is now a broker: it starts `quorra-transform-worker`, which confines itself under
`Profile::Interpreter` — `pdf-vfs-worker`'s five steps in its order, no new system call — and runs
the program. `--in-process` runs it here and prints a `note:` saying the document is parsed
unconfined. A run that reads no document (`--help`, `--remedy-sites`, a command line this program
cannot read) runs here without a word, because nothing untrusted reaches it. A missing worker is an
error naming the build command, the variable and `--in-process`; it is never a silent fallback.

**The plan's wire form is the command line that states it**, and this is the decision a later round
must not re-litigate. A `Plan` and a `Report` are rich — `archive`'s plan carries a configuration's
derivations, supplies, preservations, fonts and tool outputs, its report a conversion report — and a
second encoding of both would be a second statement of every verb's vocabulary, owned by no verb.
`plan` is a pure function of the arguments and the bytes of the files they name, so the worker is
sent the arguments and builds the plan the in-process route builds; the report never crosses, the
worker prints it and the printing crosses as bytes. **One program, two worlds**: the binary's file is
written against `pdf_transform::confined::World` — the named files, the documents, a password
descriptor, the terminal, standard output and error, each pass's outputs, the executor, a file beside
the document — and the worker takes that same file by `#[path]`. `Here` is this machine;
`Forwarded` is a frame to the broker for each, under `confined_transport::link`'s one lock, which a
font request from a drawing thread takes too. The broker answers each from `Here`, through ADR 2141's
fences. So the bytes are the in-process route's by construction, and `tests/confined.rs` holds them
so for every verb.

**Sinks and documents are the brief's descriptors in one direction only.** A document goes to the
worker as its open file's descriptor (ADR 0812), up to four, because `RLIMIT_NOFILE` is eight; a
fifth arrives as its bytes, the same `FileBytes` to every reader. An output cannot be a descriptor
the other way: `RLIMIT_FSIZE` is 0 in the confinement, so a worker writing a regular file is killed.
Each output is therefore written by the broker, gathered 64 KiB at a time in the worker; `--attach`'s
payload is a named file read like a font, not a new descriptor kind.

## 3. What it costs

- A spawn and a greeting per run, and one round trip per 64 KiB written: measured by the transform
  gate and `t3-foreign_corpus` under `--clock`, whose figures the round's record holds.
- The worker's address space is `Profile::Interpreter`'s 4 GiB ceiling; a run past it is refused by
  name where `pdf-vfs-worker`'s would be. `--in-process` is the route for a document that needs more.
- Off Unix there is no worker; the run says so and runs here (`doc/todo/35`).
- The worker links the broker's `main`, route and machine world as dead code, which the
  `#[expect(dead_code)]` on the path module says; the alternative, moving the program into the
  library, would put argument parsing and a filesystem world in the crate whose seam rule is "no
  filesystem".
