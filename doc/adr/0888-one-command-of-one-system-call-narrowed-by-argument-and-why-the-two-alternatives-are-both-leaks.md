# 0888 — One command of one system call, narrowed by argument: a confined worker can give a descriptor back, and the two alternatives to admitting it are both leaks with arithmetic behind them

Session 924. Status: **accepted**. The decision `doc/todo/61` §3 exists to take, taken.
Clauses: —, this is `CLAUDE.md` principle 3.
Code: `crates/pdf-sandbox/src/lockdown_linux.rs`
(`PERMITTED_INTERPRETER_NARROWED`, `FCNTL_COMMAND_ARGUMENT`, `F_GETFD_COMMAND`,
`restrict_system_calls`), `crates/pdf-sandbox/src/lockdown.rs` (`Profile`).
Tests: `crates/viewer-confined/tests/confined.rs`
(`a_confined_interpreter_can_close_a_descriptor_it_was_handed`,
`a_confined_interpreter_cannot_set_a_descriptors_flags`,
`a_confined_interpreter_cannot_duplicate_a_descriptor_it_holds`,
`a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers` — no longer
`#[ignore]`d), `crates/pdf-sandbox/tests/confinement.rs`
(`a_confined_decoder_cannot_ask_about_a_descriptor_at_all`).
Opened by ADR 0880 §6 and trap 32; ADR 0889 is this round's other record, about the probes.

## Context

ADR 0812 hands a confined interpreter **a descriptor per document opened on disk**: the host opens
the file, `sendmsg` with `SCM_RIGHTS` puts it beside `Command::Open`, and the worker reads it where
the document's offsets point with the one `pread64` that ADR admitted. `Command::Close` drops it.

Dropping an owned descriptor is a system call the allow-list did not have.
`std::os::fd::OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close`, under
`core::ub_checks::check_library_ub()`, to catch a double close — so a build with library-UB checks
compiled in is killed by `SIGSYS` at the close, and a release build is not. Session 920 found it
while building `doc/todo/59`'s resource port, avoided it *there* by sending the face as bytes rather
than as a descriptor, and correctly declined to fix it for the document (ADR 0880 §6): widening the
allow-list is what `doc/todo/61` exists to refuse, and refusing it deliberately is that item's job
rather than a side effect of a round about fonts.

It sat as a witness — `a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers`,
`#[ignore]`d with the reason on it — which is the worst shape a defect can have: **the shipped
binary is fine and every debug build dies**, so the thing a person runs by hand works and the gate
that would see it is switched off.

## The four candidates, priced

### 1. A seccomp rule for `fcntl` narrowed by argument to `F_GETFD` — **chosen**

seccomp-BPF can compare a system call's *scalar* arguments, which is exactly what `fcntl`'s command
is: `SYSCALL_DEFINE3(fcntl, unsigned int fd, unsigned int cmd, unsigned long arg)`. `seccompiler`
already offers the mechanism — a non-empty rule vector matches only where its conditions hold — and
`restrict_system_calls`'s own comment named it as the thing nothing had needed yet.

**What it admits.** `F_GETFD` reads the close-on-exec flag of a descriptor the process already
holds. It takes no path, opens nothing, creates nothing, changes nothing, and answers about
descriptors the worker was given rather than about the machine. The most a hostile interpreter
learns from it is which of its eight descriptor numbers are open — which `close` and `read`, both
long on the list, already tell it. **The rule is in fact strictly weaker than what the list already
permits**, and that is the sharpest form of the argument: `close(n)` is unconditionally allowed for
*every* `n`, so a hostile interpreter could already enumerate its descriptor table, and could
already do the destructive version of it. `F_GETFD` answers the same question without shutting
anything. Against that, `openat`, `socket`, `execve`, `statx`, `fstat`,
`lseek`, `dup`, `dup2` and `dup3` are exactly as absent as before, and so is every other command
`fcntl` dispatches on: `F_SETFD` cannot clear close-on-exec, `F_DUPFD` and `F_DUPFD_CLOEXEC` cannot
manufacture a descriptor, `F_SETFL` cannot turn a blocking read into a spin, and the locking
commands and `F_ADD_SEALS` are unreachable.

**It is on the interpreter's list only.** A decoder is handed no descriptor and gives none back, so
`Profile::Decoder`'s list did not move — and
`a_confined_decoder_cannot_ask_about_a_descriptor_at_all` is what fails if somebody tidies the rule
into the shared one.

**`Dword`, not `Qword`, and the reason is the kernel's declaration rather than a preference.** The
command the kernel dispatches on is 32 bits, so a 32-bit comparison compares what the kernel will
act on. A 64-bit comparison would refuse a request whose upper half is dirty and whose lower half
the kernel reads as `F_GETFD`: stricter, therefore safe, but a filter that disagrees with the call
it is filtering is how an argument-narrowed rule becomes wrong somewhere else. `seccompiler` emits
its own architecture check in front of the program, which is the other half of what makes argument
comparison sound.

### 2. Keep the descriptor alive for the worker's lifetime — **refused, on arithmetic**

Never drop it: hold every document's descriptor in a table that outlives the document. This is the
option that sounds like a design and is a leak with a longer name, and the refusal is a number
rather than a taste. `DESCRIPTOR_LIMIT` is **8**. The worker inherits three, so five documents may
be open at once — and if closing gives nothing back, the fifth document a person opens and closes is
the last that worker can open at all, after which `recvmsg` truncates the ancillary data and every
further open is refused by name. That is a **user-visible functional defect in the release build**,
introduced to fix a defect that only debug builds have. Raising the ceiling does not rescue it: the
leak is one descriptor per document opened, so any ceiling is a document count.

### 3. A wrapper that forgets rather than closes — **refused, as the same leak**

`mem::forget` on the `File`, or `into_raw_fd` and drop the number. It is option 2 written smaller
and it leaks by construction, which is how this record judges it: the descriptor is gone from the
program and still in the kernel's table, under the same ceiling, with the same arithmetic. It has
one property option 2 lacks, and it is a bad one — the leak is invisible at the call site, so the
next round to read the code would not know the count was bounded at all.

### 4. Compile the worker without library-UB checks — **refused, and it is the tempting one**

The check is `cfg(ub_checks)`, which follows `debug-assertions`; a `pdf-view-worker` built with them
off never issues the call, which is exactly why the release build passes. Refused for three reasons,
each sufficient. It turns a soundness check off **in the one process that parses untrusted bytes**,
which inverts principle 3 rather than narrowing it. It takes this tree's own overflow checks with
it, in the binary the fuzzers exist for. And it would make the defect *unobservable* rather than
absent: the next descriptor the worker is handed by some future port would have the same problem and
no build left that could see it.

## Decision

**Admit `fcntl` for the interpreter profile, narrowed by argument to `F_GETFD`, and for nothing
else.** It is the only conditional rule in this crate and the constant that carries it says so:

```rust
if profile == Profile::Interpreter {
    rules.insert(
        PERMITTED_INTERPRETER_NARROWED,
        vec![SeccompRule::new(vec![SeccompCondition::new(
            FCNTL_COMMAND_ARGUMENT,
            SeccompCmpArgLen::Dword,
            SeccompCmpOp::Eq,
            F_GETFD_COMMAND,
        )?])?],
    );
}
```

`F_GETFD_COMMAND` is a literal with `const _: () = assert!(libc::F_GETFD == 1)` beside it, so a
platform where the two differ is a compile error rather than a filter that permits the wrong
command.

**`doc/todo/61`'s rule survives this, and is not bent by it.** That item forbids answering an
*environment probe* with a permission — "the moment an environment probe is allowed to widen the
policy, every future crash has a cheap fix that costs the boundary a little, and the boundary is the
product". This is not one, and the distinction is the same one that item drew when it recorded the
fifth instance: nothing is being asked about the machine, no library is sizing itself, and the fix
is not "the worker wanted something, so the filter grew". It is the standard library checking a
precondition on a resource the worker legitimately **owns**, at the one moment it gives that
resource back — and the alternative is a worker that can be handed a thing it can never return.

The four earlier instances stay fixed the way they were fixed, and the table in `doc/todo/61` now
says which shape each was, because the rule is only useful if the two shapes can be told apart:

| shape | what to do | instances |
|---|---|---|
| a library asks the machine about itself | answer it **before** the lockdown | 902, 911, 914, 917 |
| the standard library checks a precondition on a resource the worker was **given** | the resource crosses another way, or — where it cannot — one command, narrowed by argument | 920 (the face: bytes), 924 (the document: this rule) |

**And the second row has an order.** Sending the resource another way is still the first answer, and
ADR 0880 §6's face is the case where it worked. The rule is what is left when the resource is a
descriptor by construction, which the document's is: ADR 0812's whole point is that a 6 GB file must
not cross as bytes.

## Consequences

- **The witness runs.** `a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers`
  loses its `#[ignore]` and is a gate under `cargo nextest run --workspace`, which is the profile
  where the check it is about is compiled in.
- **The measured `fcntl` traffic of the whole run is `F_GETFD`, seven times, and nothing else.**
  `strace -f -e trace=fcntl` over the witness's test binary and the worker it spawns; one of the
  seven is the worker's document descriptor, arriving without `FD_CLOEXEC` because `SCM_RIGHTS` is
  where it came from.
- **Three probes and a calibration**, which is ADR 0889's subject.
- **`Profile`'s own documentation said the two profiles differ "in exactly two things"**, which had
  been false since ADR 0812 added two calls that are neither threads nor address space. It now names
  the third difference and the reason all of them share: the second profile is handed a descriptor.
- **Nothing else in the tree changed.** No host, no protocol, no vocabulary; `git diff` outside
  `crates/pdf-sandbox/` and the two test files is empty.
