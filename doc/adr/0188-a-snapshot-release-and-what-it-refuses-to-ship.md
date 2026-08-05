# ADR 0188 — A snapshot release, and the two things it refuses to ship

Status: accepted, 2026-08-05 (session 311).

## Context

Nothing this project builds had ever left the machine it was built on. The project owner asked for
a snapshot release on every push, with executables for Linux, macOS and Windows.

Two of those three do not exist and cannot be made by editing a workflow.

## Decision

**A rolling `snapshot` pre-release, retagged at every push to `main` that passes its gates,
carrying both Linux executables.** `.github/workflows/ci.yml`, job `snapshot`.

Four choices in it are decisions rather than defaults:

**It waits for `check` and `test`.** A snapshot binary that fails the project's own gates is worse
than no snapshot, because somebody will run it and report what it does — and the report will be
about a defect the tree already knew. `needs: [check, test]` costs the full test suite's wall
clock before anything is published, which is the right trade for a build a person will actually
execute.

**Both executables or neither.** `pdf_sandbox::WORKER_PROGRAM` is a separate program the viewer
spawns for JBIG2 and JPEG 2000 images, looked for beside the running binary, and a viewer that
cannot find it *refuses those images* rather than decoding them in process — there is deliberately
no in-process fallback. So an archive carrying only `pdf-viewer` would ship a quietly reduced
program, which is the failure mode this project spends most of its effort on elsewhere. The
archive carries both and its README says they must stay in one directory.

**`LICENSE` and `NOTICE` travel with them**, because they have to. This program is MIT and
contains BSD-3-Clause font programs and SIL OFL 1.1 Liberation Sans, and both licences oblige a
*binary* distribution to carry their notices. `pdf-viewer --licences` prints the same text and is
not a substitute: what somebody unpacking a tarball can find is a file.

**`gh` rather than a third-party action.** The publishing step holds a token with write access to
the repository, which is the last place to take a dependency this project has not read. `gh` is on
the runner.

## What it refuses to ship, and why that is not an omission

**macOS and Windows.** `pdf-sandbox` confines the JBIG2 and JPEG 2000 decoders with seccomp-BPF
and Landlock, and carries a `compile_error!` for every other platform whose text is the argument:

> There is deliberately no fallback: a sandbox that silently does nothing on another platform is
> worse than no sandbox, because the code above it would keep handing untrusted input to a decoder
> while believing it was contained.

`viewer-ui` depends on that crate, so the viewer does not build on either platform. Confirmed
rather than assumed: `cargo check -p viewer-ui --target x86_64-pc-windows-msvc` fails in
`seccompiler`, on eight system-call constants `libc` does not define off Linux.

Three ways out were put to the project owner, who chose the first:

1. **Linux now; the other two get a round of their own.** Windows has job objects and
   AppContainer, macOS has the App Sandbox; choosing between them and writing the argument down is
   a security decision with an ADR attached, not a matrix entry.
2. Build everywhere with the codecs *refused* off Linux — nothing weakened, two image formats
   unavailable.
3. Build everywhere with a weaker confinement off Linux — a separate process and an address-space
   limit, no syscall filter — reported at runtime and costing a documented departure from
   principle 3.

Recorded because 2 and 3 are still available and neither is wrong; what would be wrong is
discovering in six months that the platform gap was an oversight.

## Consequences

- **A person can run what this project builds**, for the first time, without a Rust toolchain.
- **The release is only as trustworthy as the gates**, which is the point of making it wait for
  them, and is a reason not to weaken `needs`.
- **The glibc floor is the runner's.** `ubuntu-latest` moves, and a snapshot built on it will not
  run on a distribution older than that runner. Stated in the release notes rather than solved; a
  fixed older runner or a musl target is the fix if anyone is bitten.
- **The tag moves.** `snapshot` is deleted and recreated at each push, so a downloaded archive
  cannot be re-obtained by tag. It is a snapshot; a release that can be is a different artefact
  with a version number, and this project has none yet.
