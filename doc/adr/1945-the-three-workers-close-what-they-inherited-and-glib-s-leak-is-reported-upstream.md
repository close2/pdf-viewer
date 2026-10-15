# 1945 — The three workers close what they inherited, and GLib's leak is reported upstream

Session 1554, slot 3 of batch eighty-eight. Status: **accepted** and **built**, on the project
owner's answer to `doc/questions/Q430` ("option 1 is ok. But make a note, that I would prefer, that
we fix this (possibly even upstream)?", `doc/questions/A430`). Context: ADR 1907, which found the
leak and priced the four ways out; ADR 1900 §6; principle 3. Code: `keep_only_the_standard_descriptors`
in `pdf-sandbox-worker.rs`, `pdf-view-worker.rs` and `pdf-script-worker.rs`; `libc` as a Linux
dependency of `viewer-confined` and `pdf-script-worker` (`pdf-sandbox` had it). Tests:
`pdf-sandbox/tests/inherited_descriptors.rs`; `viewer-qt/tests/unsafe_position.rs`'s tree sweep.
Drive: step 84's `bare_session_start`. Cites no clause and moves no ledger row.

## 1. Decision

1. **Each worker's first statement closes every descriptor past the standard three**:
   `libc::close_range(3, u32::MAX, 0)` on Linux, before `serve`, before the confinement and before
   any byte of input. A failure is said on standard error and the worker exits with failure — a
   worker that could not let go of a leaked descriptor would carry it into the confinement, and a
   write end a compromised worker holds is a channel the sandbox did not grant. Elsewhere the
   function does nothing: no other platform confines these workers yet (`doc/todo/35`).
2. **The permission is lifted once per worker and pinned there.** Each binary root says
   `#![deny(unsafe_code)]` and lifts it with one `#[expect(unsafe_code, reason = …)]` on the
   function, whose one `unsafe` block is the call, with a SAFETY comment: it is `main`'s first
   statement, so nothing in the process owns a descriptor past the three yet. Every library keeps
   `#![forbid(unsafe_code)]`. `inherited_descriptors.rs` reads the three sources back and asserts
   the root's denial, the one lift on that function, the one `unsafe` being the one call, and the
   call being `main`'s first statement.
3. **What it does is asked of a worker, not of its source.** The same test starts
   `pdf-sandbox-worker` holding an inheritable write end of a pipe — what GLib's `pipe2(…, 0)`
   makes — drops this side's copy and reads: an end of file arrives at once. Planted back (the call
   taken out of `main`), both tests fail, the second after its ten-second ceiling with "the worker
   held the write end it inherited".
4. **The tree's sweep can see the lift.** `viewer-qt`'s `only_the_three_named_crates_in_the_tree_
   lift_the_denial` searched for the literal `#[expect(unsafe_code)]`, which a lift with a `reason`
   over several lines is not: it would have passed with three new lifts in the tree (trap 13). It
   now reads each file with its white space removed, up to the lint's name, and names six crates:
   the three it named and `pdf-sandbox`, `viewer-confined`, `pdf-script-worker` for their workers.

## 2. The note the owner asked for

**This is the program defending itself, not the fix.** The defect is GLib's: `gio/gdbusaddress.c`'s
`get_session_address_dbus_launch` runs `g_spawn_command_line_sync` for `dbus-launch` with pipes
made without close-on-exec, so any child another thread of the same process starts during that
spawn inherits their write ends, and GLib's parent then polls for an end of file the child never
gives. The owner prefers the defect fixed where it is, upstream if need be, and the report below is
the draft for the owner to file (a round holds no account to file it with).

## 3. Draft upstream report (GLib, `gio`)

**Title**: `g_dbus_address_get_for_bus_sync` autolaunch leaks `dbus-launch`'s pipe write ends into
children spawned concurrently, which can hang the caller.

**What happens.** With no session bus (`DBUS_SESSION_BUS_ADDRESS` unset, no
`$XDG_RUNTIME_DIR/bus`) on X11, `get_session_address_dbus_launch` calls
`g_spawn_command_line_sync ("dbus-launch --autolaunch=… --binary-syntax --close-stderr", …)`.
`g_spawn_sync` makes its stdout and stderr pipes with `pipe2 (fds, 0)` — not `O_CLOEXEC` — and
relies on the forked child closing what it should not keep. Any *other* thread of the process that
spawns a child between those `pipe2` calls and the parent's `close` of the write ends hands that
child both write ends. The parent then `poll`s the read ends until end of file, which never comes
while the unrelated child lives; `g_spawn_sync` does not return and the caller (here, GTK 4's start-
up asking for the session bus on its own thread) is blocked for the child's lifetime.

**Reproduction** (Arch Linux, GTK 4, X11 via Xvfb, no session bus): a GTK application whose
start-up starts a long-lived helper with `posix_spawn` from a second thread hangs before its window
maps in 7 of 16 starts. `strace -f -e pipe2,clone,clone3,execve,close,close_range,poll,wait4`
shows, per autolaunch, `pipe2([6, 7], 0)`, `pipe2([8, 9], 0)`, a fork, the GLib child's
`close_range(4, ~0)` before its exec, the parent's `close(7)`, `close(9)` and then `poll` on 6 and 8.
In a hung start the helper's descriptor table holds 6–9: GLib's write ends reached it between the
`pipe2` and the `close`.

**Suggested fix.** `get_session_address_dbus_launch` calls `g_spawn_sync` on the parsed command
line with `G_SPAWN_SEARCH_PATH | G_SPAWN_CLOEXEC_PIPES` in place of `g_spawn_command_line_sync`,
whose flags it cannot set — or `g_spawn_sync` makes its pipes close-on-exec whatever the flags, the
child's `dup2` onto 1 and 2 clearing the flag on the two it keeps. Either way the write ends cannot
reach a child another thread starts. Checked against GLib's `main` on 2026-10-15, the evidence the
report cites: `glib/gspawn.c`'s `g_spawn_command_line_sync` passes `G_SPAWN_SEARCH_PATH` alone,
`glib/gspawn-posix.c` opens the pipes close-on-exec only under `G_SPAWN_CLOEXEC_PIPES`, and
`gio/gdbusaddress.c` still spawns `dbus-launch` through `g_spawn_command_line_sync`; this machine
runs `glib2` 2.90.1.

## 4. What this does not do

The close covers these three workers. A child this program starts that is not one of them —
`dbus-launch` itself, a browser for a URI — inherits whatever the parent holds as before; and a
host that spawns nothing of its own still meets GLib's leak in its own descendants. Both are the
upstream fix's.
