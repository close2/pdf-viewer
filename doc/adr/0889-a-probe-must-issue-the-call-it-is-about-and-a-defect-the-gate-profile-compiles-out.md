# 0889 — A probe must issue the call it is about, in both directions: a confinement narrowed by argument needs a test that the *other* arguments still kill, and the defect it fixes is compiled out of the gate profile

Session 924. Status: **accepted**. The second of this round's two records; ADR 0888 is the decision
it tests.
Clauses: —, this is `CLAUDE.md` principle 3.
Code: `crates/pdf-sandbox/tests/confinement.rs` (`confined_probe`'s `fcntl` arm),
`crates/viewer-confined/tests/confined.rs` (`confined_probe`'s three `fcntl-*` arms),
`crates/viewer-confined/Cargo.toml` (`rustix` as a Linux dev-dependency).
Tests: the four named in ADR 0888, and the calibrations below.

## Context

Session 883 set the shape every confinement change in this tree has followed since: a probe that the
worker **can** do the thing the change is for, and a probe that it still **cannot** do what the
policy forbids — `a_confined_interpreter_can_read_a_descriptor_it_holds_where_its_offsets_point`
beside `a_confined_interpreter_cannot_stat_a_descriptor_it_holds`, whose own comment says it "is the
test that fails if somebody admits `statx` 'for the file's length'". ADR 0888 admits a rule that is
narrower than a system call number, and that shape has a third direction the older ones did not: the
*other arguments of the same call*.

Two things about writing those probes turned out to be findings rather than bookkeeping.

## 1. A probe written as the ordinary operation can pass by not running

The defect ADR 0888 fixes lives inside `OwnedFd::drop`, which asks `fcntl(fd, F_GETFD)` **only where
`core::ub_checks::check_library_ub()` is true** — that is, only where the build has library-UB checks
compiled in. `[profile.gates]` inherits `release`, so it has none.

So a probe written the natural way — hand the confined process a descriptor, drop it, report whether
the process survived — **passes under `--profile gates` by issuing no system call at all**. It would
have been green on the day the rule was reverted, in the profile most of `doc/todo/02` §2 runs under.
That is the same shape as ADR 0498's finding about the glyph quantum, arriving from the other side:
there, a gate turned a shipped setting *off* and measured a configuration nobody runs; here, a gate
profile turns a *check* off and measures a code path that is not compiled.

**The rule: a probe issues the call it is about, by name.** `rustix::io::fcntl_getfd` is a safe
wrapper over exactly one command, so the probe asks the question in every profile and the test says
which command it asked for. The end-to-end witness — a document opened and closed through the real
worker — is kept beside it and is the one that proves the *whole* path, but it is honest about where
it binds: `cargo nextest run --workspace` builds a `dev` worker, and that is the run in which it
means anything.

Naming the call by hand has a second benefit this round did not expect. The negative probe for
`F_DUPFD_CLOEXEC` is written as `File::try_clone`, which is what a future round would actually
reach for; the negative probe for `F_SETFD` has no safe standard-library spelling at all and had to
be `rustix::io::fcntl_setfd`. Neither would have existed if the probes had been written as "do
something with a descriptor and see".

`rustix` is a dev-dependency of `viewer-confined` for this and buys no dependency: `pdf-sandbox`
already uses it for the resource limits.

## 2. Three directions, and each one was calibrated against the defect it claims to catch

Trap 13 — a sweep for a defect must be run against the defect before it is believed — applied three
times, because ADR 0888's rule can be got wrong in three different ways. Each row below was produced
by editing the policy, rebuilding the worker, and running the tests:

| the policy, made wrong this way | what must fail, and did |
|---|---|
| the rule removed (`if false && profile == …`) | `a_confined_interpreter_can_close_a_descriptor_it_was_handed` — and the end-to-end witness, with `WorkerDied { detail: "killed by signal 31 (SIGSYS: a system call the confinement forbids)" }`, which is the defect verbatim |
| the rule un-narrowed (`rules.insert(SYS_fcntl, Vec::new())`) | `a_confined_interpreter_cannot_set_a_descriptors_flags` **and** `a_confined_interpreter_cannot_duplicate_a_descriptor_it_holds` |
| the number moved to the shared list (`PERMITTED`) | `a_confined_decoder_cannot_ask_about_a_descriptor_at_all` |

The third row is the one worth keeping, and it is why the decoder gained a probe for a call it does
not make. The two profiles are one file apart, and the cheapest way for this rule to stop being a
narrowing is not somebody arguing for `F_SETFD` — it is somebody tidying a conditional insert into
the list above it, in a round about something else. Nothing but a test in the *other* crate can say
so.

## Consequences

- **A confinement without a test that it kills is a claim rather than a boundary**, and a rule
  narrower than a system call number needs one test per direction it can be widened in: the call, the
  other arguments of the call, and the other profile.
- **The whole run's `fcntl` traffic is measured rather than reasoned about.**
  `strace -f -e trace=fcntl` over the witness reports seven calls, every one of them `F_GETFD`. A
  round that widens the port to a second resource re-runs that command before believing the rule is
  still sufficient.
- `doc/verify.md`'s `viewer-confined --test awkward_classes` line names "a round that touches the
  confinement, the interpreter's dependencies or `pdf-sandbox`'s allow-list" as owing that run. This
  round is the first to owe it under all three headings at once.
