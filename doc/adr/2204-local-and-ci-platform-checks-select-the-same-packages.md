# 2204 — Local and CI platform checks select the same packages

Status: accepted. Session 1685. Context: ADRs 2184, 2190; review 1679 section 1.

## The missing boundary

A Linux workspace build cannot compile code excluded by Windows or macOS configuration.
The previous verification instructions maintained several package selections separately from
CI, and the transform package's test targets were not named in the local commands. Compiling
a library as a transitive dependency does not compile its tests, examples or binary targets.

The platform checker owns one package selection. Both CI and the local verification commands
invoke it for the same target triples, with warnings denied and all targets of each selected
package checked. The macOS selection includes the Unix file-system core and its public ABI;
Windows excludes those descriptor-based surfaces. Native toolkit hosts remain Linux's native
workspace checks. CI retains its actual executable builds and platform-native tests: a cross
check cannot establish linking or runtime behavior.

## The population is checked independently

The audit asks Cargo which package owns each binary and reads the snapshot matrix's shipped
binary lists. Every non-Linux snapshot target must have the same operating-system assignment
in the platform matrix, and every shipped binary's owning package must be checked. Unknown or
ambiguous binary names and feature-gated targets are refused, rather than silently counted as
covered. Public ABI libraries have an explicit obligation alongside the binary population.

Calibration removes the transform package, one target and a shared library, substitutes an
unknown target and binary, and makes a shipped binary require an unrequested feature. Each
altered input must fail. A fake Cargo records the actual wrapper arguments and flags and fails;
the wrapper must preserve that failure. This also checks encoded flags, which take precedence
over ordinary RUSTFLAGS, so warnings cannot accidentally remain allowed through that path.

The full local cross-check also needs target C headers for the TLS dependency. An installed Rust
standard library and LLVM compiler alone do not supply a Windows or macOS SDK. Missing SDKs
remain a failing prerequisite, not an excuse to remove a shipped package from the selection.

This does not claim that all cross-platform defects are prevented. The checks cannot exercise
Windows UI interaction, macOS confinement behavior, or toolkit native APIs absent from these
packages. They remove a maintained-list discrepancy and cover the concrete omitted-target
shape found by the stability review.
