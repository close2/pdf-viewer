# 2209 — Stability boundaries follow observed regressions

Status: accepted direction, 2026-10-22; follow-up implementations remain separate rounds.

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

These narrow changes can reduce invalid combinations at construction boundaries. File splits
alone do not provide that guarantee; round 1691 is a mechanical navigation improvement with
its behavior preserved, not evidence that all signature coupling is resolved.
