//! Process isolation for untrusted document parsing and rendering.
//!
//! Confines the code that handles untrusted input to an unprivileged process with no
//! filesystem and no network access, using seccomp-BPF to restrict system calls and
//! Landlock to restrict filesystem reachability. The privileged process holds the
//! only file descriptor for the document and passes bytes across the boundary;
//! rendered tiles return through shared memory.
//!
//! # Why this is not redundant with Rust
//!
//! Rust removes memory-corruption bugs from code we write. It does not remove them
//! from code we link: JBIG2 and JPEG2000 have no mature pure-Rust decoders, and both
//! are historically severe attack surfaces — the FORCEDENTRY zero-click exploit was
//! a JBIG2 integer overflow. Nor does Rust prevent resource exhaustion, so this
//! crate also owns the memory and time budgets that bound decompression bombs and
//! pathological content.
//!
//! `unsafe` is permitted here because seccomp and Landlock are raw system-call
//! interfaces. It is confined to those calls.
//!
//! Implemented in Phase 5D.
