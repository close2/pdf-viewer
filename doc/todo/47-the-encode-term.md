# The encode term: 7–10 ms of host work per zoom step, deliberately deferred

Status: **open — parked by argument, with a stated revisit condition.** This is the
successor to [`45-where-a-frame-goes.md`](45-where-a-frame-goes.md)'s encode item, on the
current instrument and the current numbers; that file's measurement predates page-space
scenes and the compute lane.
Priority: 47 — performance, measured, and currently *not* the largest term.
Corpus: the owner's `tmp/Entwurf.pdf`; any page is a witness, Entwurf is the worst.
Code: `render-lib/crates/quorra-gpu/src/encode*.rs` and the record-replay seam
(`EncodeSource::RecordReplayed`, their ADR 0087); the caller's side is
`crates/render-quorra/src/scene.rs`, which since ADR 0702/0703/0705/0706 hands over
page-space scenes that survive every view change.
Instrument: the frame trace's `encode` column; `ZOOM_FRAME_ENCODE_PHASES=1` switches on
quorra's `Options::instrument_encode` for the three-phase split (their ADR 0368 read it
once: geometry ~79%, records ~13%, upload prep ~8%).

## The state of it

A zoom step re-encodes the retained scene under the new viewport: 7–10 ms host time on
Entwurf (890M, recorded beside quorra ADR 0095). The scene itself is view-free — scene
and handover are 0.0 on every zoom frame since the collapse move and the image flag
crossed the boundary — so encode is the whole of what the host still does per step
besides upload.

What has been built and measured against it:

- **Record replay** (their ADR 0087): a replayable frame re-encodes from flat records
  instead of walking the scene, held byte-identical by its own gate. Measured **no win**
  on Entwurf — the walk is seat/instance-bound, not command-bound, so replay moves the
  same seats through a different door. Later restructurings got the walk itself from
  15–19 ms to the current 7–10.
- **Device-resident records** — their ADR 0084's stage 4, the walk on the device — was
  re-priced by their ADR 0091 against the kernel numbers and lost: the kernels were
  35–50 ms against the walk's 14–20 (now 7–10), so stage B took the kernel path. That
  ordering was correct and stays correct while the kernels dominate.

## The revisit condition, stated once

Take this item up only when encode is the largest term of the step — which a successful
kernel-floor round ([`46-the-kernel-floor.md`](46-the-kernel-floor.md)) would make true:
kernels in the 20s against encode's 7–10 leaves encode a third of the remainder. The
shape to price then is stage 4 as designed — records resident on the device, a zoom step
reduced to uniform changes plus the kernels — with record replay's admission machinery
(structural, already built) deciding which frames qualify.
