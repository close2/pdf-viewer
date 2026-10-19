Status: complete
Given: 2026-10-14, in conversation — transcribed by the round
Owes: none — the workers' close was built by ADR 1945 with its tests, and the report was filed by the owner on 2026-10-19 as GLib issue 4079 (ADR 1945 §5)

> Q430: option 1 is ok.  But make a note, that I would prefer, that we fix this (possibly even upstream)?

Reading: option 1 is adopted as the question states it — the worker's first
statement closes every descriptor it inherited (`libc::close_range(3, ~0, 0)`
on Linux) before `serve` and before any byte of input, under
`#![deny(unsafe_code)]` with one `#[expect(unsafe_code, reason = …)]` and a
SAFETY comment, a test pinning the exemption's position, and the same function
in the other two workers, which are started the same way. Measured in scratch,
it took the hang from 7 of 16 starts to 0.

**The note the owner asked for, recorded so no round mistakes the worker's
close for the fix**: the leak's source is GLib's — `dbus-launch`'s pipes are
made without close-on-exec, so any child spawned from the same window inherits
their write ends. `close_range` is this program defending itself against every
library that leaks a descriptor; the owner prefers the defect itself fixed, if
necessary upstream in GLib, and the report filed on 2026-10-19 — reproduction,
system calls, and the fix GLib could make — pursues that.