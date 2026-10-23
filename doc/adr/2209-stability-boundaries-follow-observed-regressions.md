# 2209 — Stability boundaries follow observed regressions

Status: accepted direction, 2026-10-22; follow-up implementations remain separate rounds.
Amended 2026-10-23: its acceptance of the merge with the launch gate red is superseded by ADR 2242
decision 2 — a red gate takes one of four answers and is never accepted by an ADR; the rest stands.

The batch's stability review found three concrete boundaries where independently updated
values or inferred identities can make an apparently local change affect another consumer.
The response is to tighten those boundaries while keeping independent behavioral witnesses.

## Selected colour and painting context

`pdf-model/src/content.rs` and `content/colour.rs` retain resolved colour separately from
optional RGB source metadata. `content/pattern/cell.rs` manually clears that metadata when
replacing tint. Round 1689's fixed-baseline arithmetic witness fails; round 1687's ordinary
mixed-consumer tests pass, while missing-context and stroke-bypass mutations fail them.
These are distinct results: the baseline does not establish a general q/Q restoration defect.

After the tint correction settles, consider one selected-solid value with atomic source and
fallback assignment. Keep pattern definition state, tint binding and post-conversion alpha
separate. Consumer APIs should not offer an attractive stale paint value when their intended
answer is only whether a mark exists. Preserve cold-reference comparisons across fill,
stroke, text, stencil reuse and q/Q rather than asserting an internal cache arrangement.

## Toolkit placement

GTK's adapter crosses texture pixels, logical coordinates, widget natural allocation and
retained stand-in transforms. The actual allocation witness reproduces a device-scale
mismatch. Reusing the same widget through fifteen fresh/stand-in, grow/shrink and negative
origin placements exposes state that a pure arithmetic test would miss.

A follow-up should make one immutable placement calculation name device origin/extent and
logical transform, then apply that result together in the adapter. Retain the real widget
control. Qt latency and the owner's remaining blur/jump symptoms require display evidence;
matching symptoms do not justify sharing toolkit scheduling implementations.

## Artifact identity

The native harness previously inferred a library path independently of its Cargo request.
Round 1686 replaces that boundary with exact package/profile/target artifact selection and
private writable fixtures. A stale-profile refusal is different from an optional SDK skip.

Copied timing/probe binaries still need a complete run receipt: executable, companion worker,
source population and runtime configuration identities verified before execution. This is a
remaining risk, not proof that every prior measurement mixed revisions. Extend the existing
receipt approach rather than adding another independently inferred companion path.

### Execution context is more than identical bytes

Two fixed launch comparisons make this limitation concrete. The first runs baseline,
candidate, candidate, baseline with matching workers and complete input receipts. Its
baseline is production-equivalent to fixed `b9d05ee8` (the recorded build commit differs
only in six history files). The judged `opt_demo.pdf` warm-open figures are
0.135/0.137/0.137/0.138 ms. This balanced comparison supports no differential slowdown;
it does not erase the canonical candidate's earlier 0.144 ms failure against 0.140 ms.
Both copied candidate runs fail memory bands, whereas both baseline runs pass them.

The second comparison uses the same candidate bytes in original/copy/copy/original order,
with identical explicit original workers and environment. Executable, worker and input
hashes match before and after. Original-path memory passes for all six documents twice;
both copies fail the PDF20 and bug1815476 memory ceilings. The selected-child allocated
figures remain 0.8 MiB for PDF20 and 0.6 MiB for opt_demo at the output's displayed
precision, while whole-process resident measurements and approximate mapped remainders
rise for the copies. This establishes sensitivity to execution location/context, not its
exact mechanism: inode/path, argument layout and the harness-derived cold-file directory
change together, and rounded residency figures cannot prove precise mapped-memory deltas.

Timing must be judged separately. The same original artifact records 0.145 ms and then
0.129 ms in that predeclared sequence; path alone cannot determine the warm-open result.
The two copies record 0.131/0.139 ms. Near-equal instruction counts and identical read-call
counts show no substantial added-work signal, but cannot exclude code-layout/cache latency.
The final original run passes its test, with a WTPDF turn still unjudged for scheduling;
all eight runs decline graphics bring-up for carve-out placement. Neither wrapper exit 0
nor an in-band unjudged value establishes that every gate or figure passed. Preserve all
prior timing failures, copied-memory failures and refusal reasons alongside positive runs.

Artifact hashes establish byte identity, not memory/timing-context equivalence. A future
bounded harness investigation should retain raw child measurements and the selected-child
identity, then separately control equal-length paths/arguments, same-inode hardlinks and
cold-file placement. Keep executable/worker/input receipts, balanced fixed ordering and
unchanged quiet guards; do not change product bands or select favorable repetitions to
resolve a measurement question. No such follow-up is implemented here. Full raw receipts,
independent parses and qualified interpretations remain in `/home/AI/launch-ab-1686`:
`full-abba`, `location-occa`, `cross-hold-opt-demo.json` and `location-report.md`.

Final integration records 38/39 original canonical gates passing; the other result is the
launch failure above. The fixed original-location follow-up passes, supporting acceptance
without product or band changes while keeping that original failure. Every other canonical
gate passes. Workspace nextest passes 7594 tests (53 skips), all six renderer arms retain
identical frames/errors/populations, and the restored fuzz lint passes after its intentional
property-panic annotation. No single-run all-figure or copied-memory pass is claimed.

These narrow changes can reduce invalid combinations at construction boundaries. File splits
alone do not provide that guarantee; round 1691 is a mechanical navigation improvement with
its behavior preserved, not evidence that all signature coupling is resolved.
