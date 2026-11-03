# 2659 — A test asks `tools/places.sh` only on Unix, and takes a set variable everywhere

Session 1912, a single round (pixels, dependency). Status: **accepted** and **built** in
`crates/pdf-sandbox/tests/t88_conformance.rs`. Amends ADR 2656's "a Rust test reads through `bash -c
'. tools/places.sh; printf %s "$NAME"'`". ADR 2658 is this round's other decision.

## The decision

`directory()` takes `$T88_CONFORMANCE_DATA` as it stands when the environment sets it to anything but
the empty string (which `places.sh`'s `:=` also counts as unset), on every platform. Only where it is
unset does it ask for a default: on `cfg(unix)` through `tools/places.sh`, with the assertion that a
failure to answer is loud kept as it was; elsewhere the answer is `None`, and the test prints its
existing "not on this machine" sentence and passes.

## Why

CI's `build (Windows)` job runs `cargo test -p pdf-sandbox`, and since 3370c5e9 it failed there with
"tools/places.sh did not answer: " and an empty stderr: the `bash` a Windows process finds need not be
a shell that can read the script's Windows path. `places.sh` is a POSIX script whose defaults are the
project's Unix machines' directories, so off Unix it has no default to give; asking it there is a
question with no answer, and the assertion turned that into a failure where the test had printed its
sentence. Keeping the assertion on Unix keeps a broken `places.sh` loud where it is the machine's own,
which is the reason ADR 2656 gave for asking it at all.

Of the other Rust readers of `places.sh`, `tools/conformance/tests/evidence.rs` reads its text,
`batch.rs` copies it beside `batch.sh` into a scratch repository and runs that script, and
`bounded.rs` drives the lock scripts that source it — all Unix tools by construction, in `conformance`; the
Windows job runs `cargo test -p pdf-sandbox -p pdf-render` only, and none of the three is in it.
