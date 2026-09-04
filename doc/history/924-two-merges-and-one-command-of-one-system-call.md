# 924 — Two merges, and one command of one system call

2026-09-04. Argued in
[ADR 0888](../adr/0888-one-command-of-one-system-call-narrowed-by-argument-and-why-the-two-alternatives-are-both-leaks.md)
(the decision) and
[ADR 0889](../adr/0889-a-probe-must-issue-the-call-it-is-about-and-a-defect-the-gate-profile-compiles-out.md)
(the probes). `doc/questions/Q26` is the half a round should not decide for itself.

Merged: `round-921` (the instruction-file sweep, ADR 0882, `Q25`) and `round-920` (the resource
port, ADRs 0880 and 0881, `Q24`).

Touched, beyond the two merges: `crates/pdf-sandbox/src/lockdown_linux.rs`,
`crates/pdf-sandbox/src/lockdown.rs`, `crates/pdf-sandbox/tests/confinement.rs`,
`crates/viewer-confined/tests/confined.rs`, `crates/viewer-confined/Cargo.toml`, `Cargo.lock`,
`tools/state.sh`, `doc/todo/61-…`, `doc/todo/34-…`, `doc/traps/instruments-and-reports.md`
(trap 32), `doc/state-of-play.md`, `doc/HANDOVER.md`, `doc/todo/02-every-round.md`, two ADRs, one
`Q` file, this file.

**No clause is touched, so no ledger row is owed.** Checked rather than assumed: no row of
`doc/conformance/ledger.toml` cites `lockdown_linux.rs`, `lockdown.rs` or the allow-list, and the
two rows the merge brought — §9.8.1 and §9.10.2, round 920's — are `partial` and were read line by
line during the merge rather than trusted to the auto-merge.

## 1. The two merges, and what each auto-merge got wrong

**`round-921` conflicted in one line and both sides of it were half stale.**
`doc/todo/02` §2's `pdf-vfs` row: round 921 had rewritten its trap-10 distinction — which build
lines produce `pdf-vfs-worker` and which leave whatever an earlier round left — and still described
`--test awkward_classes` as a third gate line, because it branched at `23dd2b2f` and round 919
(`bb5e13c8`) had deleted that test and merged its population into `--test read_corpus`. The
resolution takes 921's rewording and 919's population.

**`round-920` auto-merged clean and one row of it was wrong**, which is why the instruction to check
rather than trust is worth having. Round 920 added **trap 32** to `doc/HANDOVER.md`'s trap index and
not to its trap-group table; round 921 had just repaired that same table for traps 14, 30 and 31 and
had added the rule to `doc/todo/02` §6 that a new trap gains *both* entries. So the merge produced,
for the third time in three rounds, the two halves of one page disagreeing — and this time the fix
was already written down. The instruments row gains 32.

**And 921 left a line in `tools/state.sh` that 919 had already killed.** That round's whole finding
was that `state.sh` did not run the sequence `doc/todo/02` §2 says it runs, and its repair added a
`vfs` section — with a third walk, `-p pdf-vfs --test awkward_classes`, which no longer exists.
`section_vfs` now has two walks and a sentence saying why there is no third: the confined *viewer*
inherited the name and its walk is `doc/verify.md`'s rather than §2's. Every `-p <crate> --test
<name>` pair in the script was then checked against the files on disk; the other nineteen exist.

## 2. The decision: `fcntl`, narrowed by argument to `F_GETFD`

`doc/todo/61` §3's, taken. ADR 0888 has the argument; what belongs here is the shape of it.

The defect is session 920's and it is the worst shape a defect can have: **a confined worker is
killed by `SIGSYS` when a document is closed, in every build with library-UB checks compiled in, and
passes in every release build.** ADR 0812 hands the worker a descriptor per document; `OwnedFd::drop`
asks `fcntl(fd, F_GETFD)` before `close`, under `core::ub_checks::check_library_ub()`, to catch a
double close.

Four candidates were priced. Three are refused, and two of them on **arithmetic rather than taste**:
keeping the descriptor alive, and a wrapper that forgets rather than closes, are the same leak
written twice — `DESCRIPTOR_LIMIT` is 8 and three are inherited, so a worker that never gives a
descriptor back refuses every document after the fifth a person opens and closes. That is a
user-visible defect in the *release* build, introduced to fix one only debug builds have. The fourth
candidate — compile the worker without library-UB checks — is the tempting one, and it turns a
soundness check off in the one process that parses untrusted bytes.

So: `fcntl` on the **interpreter** profile alone, permitted only where its command argument is
`F_GETFD`, and killed for every other command. It is the only conditional rule in the crate.
`Dword` rather than `Qword` because the kernel declares that argument `unsigned int`, so a 32-bit
comparison compares what the kernel will act on.

**`doc/todo/61`'s rule survives rather than bends**, and the item now carries the distinction that
makes it usable: a *probe* asks the machine about itself and is answered before the lockdown; a
*precondition* is the standard library checking a resource the worker was **given**, and is answered
by sending the resource another way — which is what a face does (ADR 0880 §6) — or, where it cannot
cross another way, by one command narrowed by argument. The document's descriptor cannot cross
another way: ADR 0812 exists precisely so that a 6 GB file does not cross as bytes.

## 3. The probes, in three directions, each calibrated

`a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers` loses its `#[ignore]`.
Beside it:

| probe | direction | crate |
|---|---|---|
| `a_confined_interpreter_can_close_a_descriptor_it_was_handed` | the call is permitted | `viewer-confined` |
| `a_confined_interpreter_cannot_set_a_descriptors_flags` | `F_SETFD` still kills | `viewer-confined` |
| `a_confined_interpreter_cannot_duplicate_a_descriptor_it_holds` | `F_DUPFD_CLOEXEC` still kills | `viewer-confined` |
| `a_confined_decoder_cannot_ask_about_a_descriptor_at_all` | the **other profile** still kills | `pdf-sandbox` |

**Trap 13 three times**, by editing the policy, rebuilding the worker and running the tests:

- the rule removed → the first probe fails, and the witness fails with
  `WorkerDied { detail: "killed by signal 31 (SIGSYS: a system call the confinement forbids)" }`,
  which is the defect verbatim;
- the rule un-narrowed (`rules.insert(SYS_fcntl, Vec::new())`) → the second and third fail;
- the number moved into the shared `PERMITTED` list → the fourth fails.

**The finding that generalises is ADR 0889's and is about the test rather than the policy.** A probe
written the natural way — hand the process a descriptor, drop it, see whether it survived — issues no
system call at all under `--profile gates`, which inherits `release` and compiles the check out. It
would have been green on the day the rule was reverted, in the profile most of `doc/todo/02` §2 runs
under. So a probe issues the call it is about *by name*, and `rustix::io::fcntl_getfd` is a safe
wrapper over exactly one command. `rustix` is a Linux dev-dependency of `viewer-confined` for it and
buys no dependency: `pdf-sandbox` already uses it for the resource limits.

## 4. What was measured rather than reasoned about

- **The whole run's `fcntl` traffic is `F_GETFD`, seven times, and nothing else.**
  `strace -f -e trace=fcntl` over the witness's test binary and the worker it spawns. One of the
  seven is the worker's document descriptor, which arrives without `FD_CLOEXEC` because `SCM_RIGHTS`
  is where it came from.
- **`doc/verify.md`'s class sweep of the confined viewer**, which that file names as owed by a round
  touching the confinement, the interpreter's dependencies *or* `pdf-sandbox`'s allow-list — the
  first round to owe it under all three at once. `view-awkward: killed: 0, in 14.4s`, peak 3.96 GiB
  over the tree under `tools/bounded.sh --data 12 --tree 12`.

## 5. What a round after this should know

- **The first gate run of this round was invalidated by its own author**, and it is trap 16 wearing a
  new coat. The full sequence was started on the merged result and passed `fmt`, `clippy`,
  `nextest --workspace`, the doctests, both `fuzz/` lines, the corpus gate and the oracle; then
  `crates/pdf-sandbox/src/*.rs` was edited while it ran, `pdf-sandbox`'s build identity is a hash of
  exactly those files plus `Cargo.lock`, and every following line refused to start against a worker
  the tree had moved under. **Nothing is compiled by a doc edit and everything is by a source edit**:
  a round with a background gate sequence may write ADRs and history while it runs and may not touch
  a `.rs` file or `Cargo.lock`.
- **And the second run was invalidated by a kill that reported success.** The harness wraps a
  background command in a `bash -c` of its own; the wrapper and the script are each a process-group
  leader, so `ps … | grep | head -1` names the wrapper and `kill -- -<its pgid>` leaves the script
  running, reparented to init. Two gate sequences then ran side by side, appending to one log *by
  name*, and the tell was a `fixed_documents` heading sitting under `nextest`'s compile output in a
  file that has one of each. `doc/environment.md` carries the rule: kill every distinct `pgid` the
  pattern matches, and confirm with a second `ps`.
- **And the third run was lost to the one trap this round had been warned about in writing.** A doc
  comment in `crates/pdf-sandbox/src/lockdown_linux.rs` cited "ADR 0880 §6";
  `every_citation_names_a_clause_that_exists` reads a `§` after any acronym-and-number that is not
  ISO 32000-2 and refuses it, because such a `§` "would pass by landing on one" of the standard's
  clauses. `ADR 0880 section 6` is the spelling. The cheap guard, for a round whose diff touches
  source comments: `cargo test -p conformance` costs seconds and is the last line of a sequence that
  costs half an hour.
- `Q26` asks the owner whether moving the allow-list is a round's decision at all, and flags the one
  clause of `Q24`'s proposed principle-3 wording — "the renderer's system-call set does not change" —
  that this round makes fragile if it is read as a standing claim rather than as a statement about
  the port.
