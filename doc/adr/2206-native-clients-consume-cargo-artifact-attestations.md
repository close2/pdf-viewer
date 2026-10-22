# 2206 — Native clients consume Cargo artifact attestations

Status: accepted; scoped validation passed. Session 1686.
Context: the C and KIO clients in `viewer-ffi` and `pdf-vfs-ffi`.

The integration executable chose `<target>/<profile>` from its own location, but
its nested `cargo build` omitted a profile. A gates test could therefore refresh a
debug library and link a stale gates library. Successful Cargo exit and an existing
filename were insufficient evidence: batch 105 encountered exactly that mismatch.

The three affected integration files include one test-only module from
`tools/conformance/test-support/cargo_artifacts.rs`. It resolves workspace package identities
once through Cargo metadata and checks the executable against that metadata's
native target directory. The output name `debug` maps to Cargo's `dev`; `release`
and custom names such as `gates` retain their names. An unexpected or cross-target
layout is a failure, not a skip. A future cross-target harness needs an explicit
compiler and runner contract rather than guessing from optimization settings.

Each library or worker request names the exact package identity, target and profile.
The build returns JSON; only its matching compiler-artifact message may authorize
the path, with the requested target kind and crate type. Exactly one dynamic library
or worker executable must resolve directly inside the selected profile directory.
The callers pass that library path to the C linker or CMake and that executable to
the worker environment. No unrelated successful build can attest a pre-existing file.
A `fresh: true` artifact remains valid: Cargo verified the request, and rebuilding
unchanged output or testing timestamps would defeat incremental builds.

This boundary changes test infrastructure only. `serde_json` and `tempfile` are dev
dependencies. The included modules live under the existing conformance source root,
so source scans read them without another workspace package or root exception. Production discovery,
ABI declarations and worker protocols do not change. Missing optional native
prerequisites retain their explicit skips; build or profile mismatches never do.

Pure controls cover native profile mapping, exact target/package/kind selection,
missing and ambiguous artifacts, fresh output and a seeded wrong-profile library.
An isolated copy with profile verification removed must fail that last control.
Real dev and gates clients remain necessary: parsing a synthetic attestation does
not prove a library links or a confined worker answers.

Validation: scoped development nextest passed 99 tests. The gates-profile C suites
passed 7 and 8 tests, and the KIO suite passed 7, including actual plugin loading,
browsing, deletion and insertion. Printed artifact receipts name the gates libraries
and workers. Six isolated helper controls passed; removing profile verification
made the seeded wrong-profile control fail with exit 101. Scoped formatting and
warnings-denied all-targets Clippy passed. Full batch validation remains separate.
