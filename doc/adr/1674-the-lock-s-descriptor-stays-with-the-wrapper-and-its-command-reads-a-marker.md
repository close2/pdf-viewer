# 1674 — The lock's descriptor stays with the wrapper, and its command reads a marker

Session 1419. Status: **accepted** and **built**. Pays what ADR 1659 section 4 left owed to
`tools/bounded.sh` and `tools/batch.sh` together; amends ADR 1646 section 1's "the descriptor is
inherited by the command". Supersedes nothing.
Context: ADRs 1646, 1659, 1662, 1671; traps 130, 131.
Code: `tools/bounded.sh` (`lock_open_in`, `held_by_marked_ancestor`, `lock_held_here`, `lock_take`,
the command's subshell, `--held`, self-test case 8), `tools/batch.sh` (`holds_the_lock`).
Tests: `tools/conformance/tests/batch.rs`
(`arms_held_runs_under_the_wrappers_marker_and_a_daemon_it_leaves_holds_nothing`), `bounded.rs`
(the self-test, and `every_instruction_a_person_reads_spells_the_lock_as_the_wrapper`).

## 1. What a handed-down descriptor cost

ADR 1646 passed the lock's descriptor to the command so that the lock stayed held while anything
the walk started still ran. A build inside the lock starts `sccache`'s server, which outlives the
build by design, and on 2026-10-07 one held the machine's lock 1 115.3 s after the walk that started
it had ended (ADR 1659 section 4; ADR 1671 section 1 replays batch sixty-four without that hold at
2 230.6 s less queue). ADR 1659 built the fix and took it out, because `tools/batch.sh arms-held`
proved it ran under the lock by finding that very descriptor in its own `/proc/$$/fd`.

## 2. Decision

- **The command runs without the descriptor; the subshell that waits for it keeps it.** The command
  and the `tee` beside it are started with the descriptor closed, and the wrapper's subshell, which
  lives exactly as long as they do, holds it. So the lock ends with the command and not with what
  the command left running, and a wrapper killed while its command runs still leaves the lock held
  by that subshell until the command ends — the half of ADR 1646's reason that was right.
- **The command reads `HEAVY_WALK_HELD_BY`, the subshell's pid.** A marker is only an inherited
  word, and a daemon keeps it after the walk, so it counts as a hold only while the process it names
  is an ancestor of the asker and has the lock's descriptor open. `tools/bounded.sh --held` answers
  that, and a descriptor the asker has itself (a bare `flock <lock>` above it) is a hold as well.
- **`holds_the_lock` asks the wrapper** rather than answering a second time: one reading of the
  marker, in the file that writes it.
- **A `--lock` run under a marked hold takes nothing and writes no line.** Without the descriptor it
  would open a second description and queue behind its own ancestor for ever; under it, the outer
  wrapper's line is the hold, and a second line would count the hold twice.
- **The output's copy is a descriptor of its own number.** The subshell used to `exec 3>&1`, and an
  inherited lock from a bare `flock` is often on 3, which closing it would have closed under the
  command's standard output.

**Calibrated** (trap 13), each defect planted in a copy of the wrapper: the descriptor handed down
again — case 8 fails on the daemon that kept the lock; the marker not exported — `--held` says no
inside a hold; the marker read without the parent chain — `--held` says yes in the daemon after the
hold; the nested run not recognised — it queues behind its ancestor until the case's 20 s timeout.
`batch.rs`'s new test failed both of its plants the same way (the record says how).

## 3. What stays

A `sampler` the wrapper abandons in the kernel still holds a copy of the descriptor until its `ps`
returns; it is the wrapper's own child, not the walk's, and has never been seen to. A bare `flock`
round a command hands the descriptor down as it always did; that is the caller's lock, and every
instruction a person reads now spells the wrapper instead — `fuzz/seeds.sh`'s header,
`doc/verify.md`'s two lines, `tools/main-checkout.py`'s re-seed line and four census doc comments.
Four files still spell it, held by name in `bounded.rs` as a list that may only shrink: three are
the script crates' (slot 1's this batch), and RFC 0008's command block.
