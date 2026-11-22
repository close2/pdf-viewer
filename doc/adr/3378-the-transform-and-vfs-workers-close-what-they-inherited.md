# 3378 — The transform suite's worker and the vfs worker close what they inherited

Session 2169. Status: accepted. Context: ADRs 1907, 1945, 3361 (whose record left the question);
principle 3. Cites no clause and moves no ledger row.

## The question

ADR 3361 put the close in one crate and five helpers. `quorra-transform-worker` and `pdf-vfs-worker`
called nothing: both are confined workers that read untrusted bytes, and ADR 1907's question was
whether either is started beside a library that leaks descriptors.

## Answer

Both are, or can be, and the close is owed on a reason that does not need GLib:

- `pdf-vfs-worker` is started by `pdf-vfs-ffi`, a library loaded into a host's process — a KIO
  worker today, a file manager's process in general — and Qt's event loop on Linux is GLib's in a
  Qt built with GLib, as distributions build it. A
  spawn there can land between GLib's `pipe2` and its `close`, which is ADR 1907's hang exactly.
- `quorra-transform-worker`'s broker is a command, and `std::process::Command` passes on every
  descriptor without close-on-exec that the broker itself inherited from whatever started it — a
  shell, a script, a build tool. A confinement installed afterwards does not revoke an open
  descriptor: Landlock and the system-call filter govern what a process may *open*, not what it
  already holds.

Both are started by `confined-transport`'s `host` with exactly standard input (the socket), output
and error, which is the crate's stated precondition. So each `main` calls
`inherited_descriptors::keep_only_the_standard_descriptors()` first, both programs `forbid(unsafe)`,
and `crates/inherited-descriptors/tests/callers.rs` names seven helpers.
