# 3361 — Every confined helper closes what it inherited through one crate

Session 2165. Status: accepted and built, on the project owner's answer to `doc/questions/Q979`
(`doc/questions/A979`, 2026-11-20: name `pdf-media-worker` and `quorra-sound` in the tree's sweep
under ADR 1945, recorded in an ADR, and a clean follow-up consolidating
`keep_only_the_standard_descriptors` into one shared crate is welcome). Context: ADRs 1907, 1945,
3297, 3317; `doc/questions/A430`; principle 3. Cites no clause and moves no ledger row.

## Context

ADR 1945 put one `unsafe` call — `libc::close_range(3, u32::MAX, 0)`, `main`'s first statement —
into each of three workers' programs, each lifting `unsafe_code` once for it, and named their
three crates in `viewer-qt`'s `only_the_named_crates_in_the_tree_lift_the_denial`. ADRs 3297 and
3317 wrote the same function a fourth and a fifth time, into `pdf-media-worker` and `quorra-sound`,
without naming either crate, and the sweep failed on main. Every confined helper the tree adds
would repeat the copy and widen the list again.

## Decision

1. **`inherited-descriptors` is the one place the close is.** A crate of one function,
   `keep_only_the_standard_descriptors() -> std::io::Result<()>`: on Linux the `close_range`
   above, under `#![deny(unsafe_code)]` with one `#[expect(unsafe_code, reason = …)]` and the
   SAFETY comment ADR 1945 wrote; elsewhere `Ok(())`. `libc` is its Linux dependency.
2. **The five programs call it, and forbid `unsafe`.** `pdf-sandbox-worker`, `pdf-view-worker`,
   `pdf-script-worker`, `pdf-media-worker` and `quorra-sound` (the last behind its `device`
   feature) each call `inherited_descriptors::keep_only_the_standard_descriptors()` as `main`'s
   first statement and say `#![forbid(unsafe_code)]` at their root; their own copies are deleted,
   and so is `libc` from `viewer-confined`, `pdf-script-worker`, `pdf-media-worker` and
   `quorra-sound`'s dependencies (`pdf-sandbox` keeps it for system-call numbers).
3. **The sweep names the crate, not the helpers.** The list is `inherited-descriptors`,
   `pdf-vfs-ffi`, `viewer-display-link`, `viewer-ffi`, `viewer-qt`: five names where there were
   seven, and both of A979's helpers are covered by the first. A new helper calls the same
   function and adds no name.

## The cost, written down

**A safe function whose soundness is a position.** Closing descriptors by number is sound only
while no object in the process owns one past the three, and that is a property of *where* the call
is, which no signature states. As five private functions it was each program's SAFETY comment; as
a public safe function any crate could call it later and close a descriptor an `OwnedFd` holds. An
`unsafe fn` would state that honestly and put an `unsafe` block — and a lift — back into each
program, which is what A979 asked to stop. So the obligation is discharged by a test instead of a
type: `crates/inherited-descriptors/tests/callers.rs` reads every source under `crates/*/src` back
and fails unless the sources naming the crate are exactly the five programs, each forbids `unsafe`,
calls the function once, and calls it as `main`'s first statement; and it pins the crate's own one
lift and one `unsafe` as `pdf-sandbox`'s test pinned the three. `pdf-sandbox`'s
`tests/inherited_descriptors.rs` keeps the behavioural half: the decoder's worker, handed an
inheritable write end, lets go of it before reading.

## What is not changed

`quorra-transform-worker` and `pdf-vfs-worker` do not close what they inherited and did not
before; whether either is started beside a library that leaks is ADR 1907's question for them, and
each would call this function rather than copy it. The defect is still `GLib`'s, and the owner's
upstream report (ADR 1945 §2) stands.
