# Q430 — May the image decoder close the descriptors it inherited, with one `unsafe` call before it reads a byte?

Source: round 1535, ADR 1907, reproducing ADR 1900 §6's deadlock.

## What was found

On a display with no session bus, `quorra-gtk` hangs before its window maps in 7 of 16 starts of
`bug1815476.pdf`. GLib asks `dbus-launch` for a bus through pipes it makes without close-on-exec,
and `pdf-sandbox-worker`, started from the thread that reads page one, inherits their write ends
whenever its start lands inside GLib's spawn. GLib then waits for an end of file that the worker
never gives, because the worker lives as long as its connection. ADR 1907 §1 has the system calls.

## Why it needs the owner

Principle 3: "`#![forbid(unsafe_code)]` on every crate that touches PDF bytes; this is
compiler-enforced, not a convention." Closing a descriptor the program does not own is `unsafe` in
Rust's I/O-safety model, both in the worker and in the child before its exec, so every
construction that fixes this puts one `unsafe` call in or beside the decoder's process:

1. **The worker's first statement**: `pdf-sandbox-worker.rs` becomes `#![deny(unsafe_code)]` with
   one `#[expect(unsafe_code)]` function, `close_range(3, ~0, 0)` (Linux), run before `serve` and
   before any byte of input. A test like `viewer-qt`'s `unsafe_position.rs` pins where it is.
   It costs one system call per worker. Built in scratch, it took the hang to 0 of 16 starts.

   ```rust
   #[cfg(target_os = "linux")]
   #[expect(unsafe_code, reason = "closes by number what no object in this process owns yet")]
   fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
       // SAFETY: `main`'s first statement, before anything in this process opens a descriptor.
       let closed = unsafe { libc::close_range(3, u32::MAX, 0) };
       if closed == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
   }
   ```
2. **A launcher binary** of its own crate, which never sees PDF bytes, closes and execs the worker.
   `forbid` stays whole, but it costs one more exec per worker, and every place that copies the
   worker beside a program (`tools/batch.sh`'s arms and install, the exports) must copy it too.
3. **`CommandExt::pre_exec` in `pdf-sandbox`**, which costs `pdf-sandbox` its `forbid`, and it
   takes `posix_spawn` away from `std`, so every worker start becomes a fork of the whole viewer.
4. **Nothing**: the deadlock needs a start without a session bus, where GLib autolaunches, and it
   stays a race on GLib's side.

## What the tree does meanwhile

Nothing is changed in the spawn. The drive's step 84 (`bare_session_start`) starts each window
eight times with no bus and says `wrong` with the descriptors the worker holds when a start hangs.
The drive's other steps run on a private bus and are not affected, and neither is a desktop
session with a bus.

## Recommendation

**Option 1.** The call runs before the worker has read anything, so no untrusted input reaches
it, which is the reason principle 3 gives for its rule. It makes true what the worker already says
about itself: "The worker inherits three and opens none" (`DESCRIPTOR_LIMIT`'s documentation).
And it covers any host and any library that leaks a descriptor, not only GLib. The same function
would then belong in `pdf-view-worker` and `pdf-script-worker`, which are started the same way.
