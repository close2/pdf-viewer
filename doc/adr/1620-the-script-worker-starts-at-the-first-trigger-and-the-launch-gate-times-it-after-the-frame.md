# 1620 — The script worker starts at the first trigger, and the launch gate times it after the frame

Status: accepted and **built**. Session 1392. Builds on RFC 0008 sections 6.2 and 6.6 (accepted,
`doc/questions/A193`, its answers 5 and 6), ADR 1604 (`Command::Presented` from every window) and
ADR 1609 (what crosses to the worker, and `OpenCost`). Uses ADR 1616's `Command::Scripts`.
Context: `CLAUDE.md` principle 2, "[n]othing on the launch path waits for warmth".
Code: `crates/viewer-ui/tests/launch_path.rs` (the script stage, `KeptWorkers`,
`print_script_stage`, `started_before_the_frame`, `script_worker_is_here`),
`doc/checks/launch-path.toml` (`script_spawn_ms`, `script_first_run_ms`, the `opt_demo.pdf` row),
`crates/viewer-ui/Cargo.toml` (`pdf-script-worker` for the test host).

## 1. The premise, read in the units it was printed in

ADR 1609 section 5 recorded `script_open spawn_ms=1.8–2.1 first_run_ms=1.1–1.7`. The key says
milliseconds and `OpenCost::line` prints milliseconds: a spawn of 0.0018 s, not 1.8 s. Nothing a
reader feels, and the measurements below are why that stays true on the largest library the corpus
holds.

## 2. Measured, in the gates profile, the worker built `--features engine`

The worker is 14.3 MB, a position-independent executable with 87 033 relative relocations. Pinned to
the four Zen 5 cores, load 2.9 to 9.9, five runs an arm unless said:

| | `spawn` | `first_run` | the open sequence |
|---|---|---|---|
| `end_to_end`'s one-statement format script | 1.41–1.60 ms | 0.96–1.15 ms | — |
| `opt_demo.pdf`, a library of `0;` | 0.85–1.05 ms | 0.53–0.59 ms | 1.55–1.81 ms |
| `evince-LINK-46-2.pdf`, 173 216 bytes in two requests | 1.03–1.72 ms | 16.8–17.8 ms | 20.4–22.3 ms |

**The spawn.** From asking to the greeting's first byte, 200 spawns: 0.82 to 1.06 ms (minimum to
median) against 0.17 to 0.25 for a 4 MB Rust program that prints one line. The dynamic loader states
246 726 cycles for the worker (`LD_DEBUG=statistics`, 68% of it relocation). Under `strace
--seccomp-bpf`, exec to the standard library's start read 1.04 to 1.33 ms against 0.48 to 0.65 for
the small program, and the worker's `main` to its greeting — two limits, Landlock, `NO_NEW_PRIVS`,
the filter — 0.26 to 0.40. So of a millisecond's spawn about 0.6 ms is the binary's own load and a
tenth or two the confinement.

**The first run.** In process, the realm is 0.40 to 0.51 ms — 2.06 M instructions, of which the host
object model's installation is 1.13 M — and `opt_demo.pdf`'s script 0.05. On the largest library the
first request is 15.2 to 15.7 ms, and callgrind by function over that process says where: 242.5 M
instructions, `Realm::run` 228.2 M, `Script::parse` 215.9 M, and of that **Boa's AST optimizer
(`Optimizer::apply`, its constant-folding walker) 171.6 M — 70.8% of the process**. Parsing proper is
about 44 M and evaluation about 12 M.

## 3. The levers, each decided

- **A realm built after the first present and kept warm** would take the realm's 0.4 to 0.5 ms off
  the first trigger. Not taken. The first trigger already comes after the first present, and a realm
  built ahead of it is a realm built for every scripted document whether a script ever runs.
- **The worker started at `Command::Presented` rather than at the first trigger.** RFC 0008
  section 6.2 and the owner's answer 5 say the worker starts at the first trigger the level lets run;
  section 6.6 and answer 6 say the open sequence runs after the first present. For a document with an
  open-time script the two are one moment, since the open sequence *is* its first trigger. For one
  whose scripts are all a field's, starting at the present would move one spawn and a small first
  run, 1.4 to 2.3 ms, off the first keystroke, and spend a process on every scripted document nobody
  types into. A keystroke that pays it once pays less than a frame at 60 Hz. **Not taken: the spawn
  stays where ADR 1609 put it**, and the gate holds the other half — no worker before the frame.
- **The binary's load**, about 0.6 ms. Linking the worker non-PIE would drop the relocations and
  give up address-space randomisation in the process that runs hostile scripts: principle 3 over
  0.6 ms after the first present. Not taken.
- **Boa's optimizer.** With `Context::set_optimizer_options` emptied, the largest library parses in
  4.7 to 5.2 ms against 13.6 to 15.0 and evaluates in 1.8 against 2.0. On three other libraries
  (`REDHAT-1167020-11.pdf`, `poppler-LINK-82-0.pdf`, `5712688.pdf`) the two read the same within
  their spread; on a 2 500-term chain of constants the optimizer wins, 1.6 ms against 4.8. And a
  left-associative chain overflows Boa's 8 MiB stack either way — 20 000 terms in the optimizer,
  15 000 in the bytecompiler without it — which `pdf_script`'s nesting bound does not see, because it
  counts brackets; in the worker that is one named loss. **Not taken here**: the engine is not this
  round's crate and the evidence is one library in four. The numbers are the engine's owner's.

## 4. The gate's script stage

Every row's first-page child hands its viewer a runner as a window at `on` does — `Command::Scripts`
before `Command::Open`, a maker of `ScriptWorker`s, which start nothing when made — so what a reader
who lets scripts run pays before page one is inside every `first_page_ms`. Once the frame's clock,
its hash and the memory high-water are read, the child sends `Command::Presented`, and the open
sequence starts the worker at its first trigger. The timeline gains two lines, the run prints
`OpenCost::line()` verbatim beside how many workers started, and a worker started before the frame
is a failure judged on any machine, since it is a count. A row whose document has no script reached
by the open sequence prints that no worker was started.

`doc/pdf.js/test/pdfs/opt_demo.pdf` is the row that bands the stage: 5 642 bytes, one page, a form
whose Table 32 name tree holds one script, so the worker starts at the open sequence and its first
run is the realm and nothing else — a figure that moves with the spawn and the engine, not with a
document's library. Each of its two figures is the quickest of nine first-page children of its own.
The worker is built by `cargo build --release -p pdf-script-worker --features engine --bins`; where
it is not beside the test binary the stage is printed `NOT MEASURED` and counted, as a missing
`valgrind` is.

## 5. What the gate saw

Under `release`, the worker 9.5 MB: three runs of this tree interleaved with three of HEAD's, load
4.5 to 11.5, then three more of this tree at 1.1 to 2.0. Every row's frame hash is the same in both
arms and every `first_page_ms` sits inside its unmoved band in both; the arms differ by at most
2 ms a row and the quicker arm changes from row to row (the five-page row 24.4 to 26.9 ms against
HEAD's 23.4 to 25.2, `bug1815476.pdf` 26.9 to 27.5 against 28.8 to 29.2). No row started a worker before its
frame in any run. Two rows other than the banded one start one at the open: `bug1815476.pdf` and
`xfa_filled_imm1344e.pdf`, spawn 0.71 to 0.75 ms and first run 0.63 to 0.70, after the frame. On
`opt_demo.pdf` the quickest of nine read 0.684 to 0.709 ms for the spawn and 0.449 to 0.475 for the
first run, and the bands are the check file's rule over them.

## 6. Found on the way, and handed to the worker's owner

At the end of its input the worker returns from `main`, and the standard library's teardown calls
`sigaltstack`, which `Profile::Script` does not allow: the process dies by `SIGSYS` (exit 159) and
`systemd-coredump` is invoked. `Host`'s drop kills the worker first, so a window does not meet it;
a worker whose host died would.

