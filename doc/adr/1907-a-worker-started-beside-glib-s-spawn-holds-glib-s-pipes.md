# 1907 — A worker started beside GLib's spawn holds GLib's pipes, and only the worker can let go

Session 1535, slot 2 of batch eighty-five. Status: **the mechanism is accepted; the fix is
proposed and waits for the project owner** (`doc/questions/Q430`), because every construction that
closes the descriptors needs one `unsafe` call in or beside the decoder's process. Context: ADR
1900 §6, which found the deadlock; ADR 0194 (no `unsafe` where a platform has no confinement). Drive: step 84's
`bare_session_start`. It cites no clause and moves no ledger row.

## 1. The mechanism, read off the system calls

On a display with no session bus (`DBUS_SESSION_BUS_ADDRESS` unset, no `$XDG_RUNTIME_DIR/bus`),
GDBus falls back to X11 autolaunch: `gio/gdbusaddress.c`'s `get_session_address_dbus_launch` runs
`g_spawn_command_line_sync ("dbus-launch --autolaunch=… --binary-syntax --close-stderr")`, without
`G_SPAWN_CLOEXEC_PIPES`. GTK asks for the session bus four times in its first 140 ms here, on its
own thread. `strace -f` shows each one: `pipe2([6, 7], 0)`, `pipe2([8, 9], 0)` and a third pair
on the main thread, a fork, the child's `close_range(4, ~0)` before its exec, and the parent's
`close(7)`, `close(9)` and then `poll` on 6 and 8 until both say end of file, and only then
`wait4`. Arch's `dbus-launch` says "X11 autolaunch support was disabled at compile time" and exits
1 within 7 ms, so it is a zombie until that `wait4`.

Meanwhile the opening thread reads page one, and `bug1815476.pdf`'s CCITT image starts
`pdf-sandbox-worker` with `std::process::Command`, which uses `posix_spawn` and makes its own
pipes with `O_CLOEXEC`. A spawn that lands between GLib's `pipe2` and GLib's `close(7)` hands the
worker every descriptor without close-on-exec — GLib's write ends among them. The hung process's
table says exactly that: the worker holds 6–9 (both ends of both pipes) and 10, and the main
thread polls 6 and 8 for ever, because the worker keeps them for as long as its connection lives.
**So the leak is GLib's pipes reaching Rust's spawn**, not the reverse: GLib's child closes all it
did not mean to pass, and Rust's pipes are close-on-exec. At HEAD 7 of 16 starts hung (10 s each).

## 2. Where it can be fixed, and what each costs

GLib's side is not this program's, and its design — pipes left inheritable, closed in the child —
is safe only in a process where nothing else spawns while it does. This side owes the worker
holding exactly the descriptors it was handed, which the worker's `main` already says ("Standard
error is the one descriptor pointing outside this process") and `DESCRIPTOR_LIMIT`'s documentation
("The worker inherits three and opens none"). Closing a descriptor by
number is `unsafe` in Rust's I/O-safety model (`rustix::io::close`, `libc::close_range`;
`OwnedFd::from_raw_fd`), and so is every way of doing it in the child before exec
(`CommandExt::pre_exec`). The choices are Q430's: one `close_range(3, ~0, 0)` as the worker's
first statement; a launcher binary that closes and execs the worker; `pre_exec` in `pdf-sandbox`;
or nothing. The first, built in scratch, took the hang from 7 of 16 starts to 0 of 16.

Not taken, each for its reason: telling GLib not to autolaunch (an environment change before
threads, `unsafe` in edition 2024, and it takes the bus from a machine where autolaunch works);
holding the worker's start until GTK's start-up is over (a guess at when GTK stops asking, which
costs page one its parallel read and still races a later bus request); and killing a worker found
holding a descriptor it was not given (a persistent descriptor the host itself inherited would
refuse every image).

**Gates.** HEAD's release `quorra-gtk` on a bare Xvfb, sixteen starts of `bug1815476.pdf`, each
given 10 s to map: 7 hung; with the scratch close, built from the same tree as its window, 0 of 16.
Step 84 on HEAD: `wrong`, 5 of 8, each with `pdf-sandbox-worker` holding descriptors 6–10; with
the close, `works`, 8 of 8. With the lock's four rayon threads in the window's environment the
race came up in 1 of 12 starts, which is why the step takes them out. `strace -f -e
pipe2,clone,clone3,execve,close,close_range,poll,wait4` on three starts, and the hung process's
descriptor table.
