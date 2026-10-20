# 2055 — The toolkit rule is a test over every target, whose toolkits are read from the toolkit hosts' manifests

Session 1609, slot 3 of batch ninety-six, a "host" round. Status: **accepted**; built as
`tools/conformance/tests/toolkits.rs`. **Amends ADR 2048 §2.2's command.** Context: ADR 2048's
rule that `viewer-ui` and `viewer-ffi` depend on no widget toolkit, and the conformance test it said
was owed.

## Decision

1. **The command is `cargo tree -p <crate> -e normal,build --target all -i <toolkit>`.** ADR 2048
   wrote it without `--target all`. That form reads the host platform's graph only, and it misses
   the likeliest way for a toolkit to arrive in a crate whose gates run on Linux: a
   `[target.'cfg(windows)'.dependencies]` table. Planted in a scratch export as `cxx` under that
   table in `crates/viewer-ffi/Cargo.toml`, ADR 2048's command printed "nothing to print" and exited
   0. The test, with `--target all`, named `cxx`.
2. **The test reads each guarded crate's whole graph once and intersects it with the toolkits.**
   That is the answer one `-i` per toolkit would give, in two `cargo tree` runs rather than thirty.
3. **The toolkits are derived, not listed** (trap 25). A toolkit is:
   - an external package that `viewer-gtk` or `viewer-qt` names directly and that none of that
     host's workspace dependencies reaches. This gives `gtk4`, `pango`, `cxx` and `cxx-qt-build`,
     and leaves out `thiserror`;
   - every `-sys` package in the two hosts' graphs that those workspace dependencies do not reach,
     which is the C libraries: `gtk4-sys`, `glib-sys`, `cairo-sys-rs` and seven more;
   - `cxx-qt`, by hand, because the workspace manifest refuses it by name. It is in no lock file,
     and the test prints that rather than counting it.

   The hosts' whole private graph was the first candidate, and it is wrong. It holds `memchr`,
   `pkg-config`, `unicode-width` and five more that `viewer-ui` reaches for reasons of its own.
4. **Calibrated twice.** In the test, the same check run over `viewer-gtk` must name `gtk4` and
   over `viewer-qt` must name `cxx`. In this round, a scratch export with `gtk4.workspace = true`
   planted in `crates/viewer-ui/Cargo.toml` failed, naming `gtk4` and the eleven packages of point 3 that come with it. A second
   export, with the Windows-only `cxx` of point 1 in `viewer-ffi`, failed naming `cxx`.

## What it costs

A `.NET` or AppKit binding is in no manifest, so the derivation cannot find one. The .NET host is
C# outside Cargo (ADR 2054), and a Swift host would be too. When a Rust crate first binds either,
its host's manifest names it and the population takes it up. The rule's shape needs no edit for
that.
