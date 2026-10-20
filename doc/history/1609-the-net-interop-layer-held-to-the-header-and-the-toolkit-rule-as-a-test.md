# 1609 — The .NET interop layer held to the header, and the toolkit rule as a test

Session 1609, slot 3 of batch ninety-six, a "host" round; 2026-10-20. RFC 0010's N0 and ADR 2048's
owed test; ADRs 2054 and 2055 (the second amends ADR 2048's command). No ledger row moved.

**Built.** `hosts/windows/Quorra.Interop/`, outside the workspace: a `net10.0` project with one
cdecl `[LibraryImport]` per function in `quorra.h`, one constant per `#define QUORRA_…` and one
struct per struct, under the header's names, one file per header section, all `internal`, none of
the header's prose repeated; `Native.cs` states the type rule. `crates/viewer-ffi/tests/
header_and_interop_agree.rs` holds every function, parameter name and type, result, field and
constant to the header under that rule, so a `uint` for a `size_t` fails it. `tools/conformance/
tests/toolkits.rs` checks ADR 2048's rule over `--target all`, the toolkits derived from
`viewer-gtk`'s and `viewer-qt`'s manifests. `crates/viewer-ui/src/lib.rs` no longer says `ashpd`.

**Coordination.** Slot 6 edited `quorra.h` during the round. The C# is declared against the header
at the last run: 261 functions, 297 constants, 8 structs, slot 6's 21 additions included.

**Premises.** No `dotnet` here: held, so nothing compiled the C#, and none was installed (RFC 0010
§8.4 is the owner's). The comment stripper read the header, and the banners gave the files: held.
Both crates already satisfy the rule: held. **ADR 2048's command did not hold**: without `--target
all` it printed "nothing to print" for a `cxx` planted under `cfg(windows)` in `viewer-ffi`. The
brief said `DllImport`; RFC 0010's `LibraryImport` was taken (ADR 2054 §4). The hosts' whole
private graph is no toolkit population: it holds `memchr` and `pkg-config`.

**Calibrated.** Eight C# defects planted by a `-U0` patch, applied and reversed: a `nuint` for a
`uint`, a dropped field, a changed constant, a doubled declaration, a missing `CallConvCdecl`, a
renamed parameter, a missing `StructLayout` and a bare `bool`. The test named all eight, and named
slot 6's 21 as missing before their C# existed. In a scratch export `gtk4` planted in `viewer-ui`,
then a Windows-only `cxx` in `viewer-ffi`, each failed the toolkit test naming the package.

**Unfinished.** `dotnet build` as a CI job (the library names no Windows type, so `ubuntu-latest`
would do); N1–N4. The export is left in `scratchpad/r1609/plant/`.

**Gates.** `cargo nextest run -p viewer-ffi`: 0, 46 of 46. `RUSTFLAGS="-D warnings" cargo clippy -p
viewer-ffi -p conformance --all-targets`: 0. `rustfmt --check` on the edited Rust: 0. `cargo test
-p conformance --no-fail-fast`: 101, `toolkits` and 55 binaries green, four failures siblings'
mid-edits (`conformance`, `names`, `records`, `state_sections`). `tools/state.sh --round 1609
hosts`: 0, 45 of 45 `Command`s, 261 entry points. `cargo clippy -p viewer-ui --lib`: 101, on a
neighbour's `raster-gpu` edit.
