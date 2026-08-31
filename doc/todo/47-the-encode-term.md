# The encode term: ~10 ms of host work per zoom step, deliberately deferred

Status: **open — parked by argument, with a stated revisit condition.** This is the
successor to [`45-where-a-frame-goes.md`](45-where-a-frame-goes.md)'s encode item, on the
current instrument and the current numbers; that file's measurement predates page-space
scenes and the compute lane. **The condition was reported met once and was not** — ADR 0766
read a resize step's 129 ms of `encode` off `examples/zoom_frame`, whose device then ran
quorra's default `Coverage::Cpu`; the shipped window takes `Coverage::Compute` for any
moved view on a real adapter (`surface::lane_for`, ADR 0700), and on that lane the same
step is 63–66 ms with encode at 9.4–10.1 (ADR 0767, both lanes in one sitting). **A figure
quoted against this condition names its lane**, and the instrument now has the knob
(`ZOOM_FRAME_COVERAGE=compute`) that draws the shipped gesture.
Priority: 47 — performance, measured, and currently *not* the largest term.
Corpus: the owner's `tmp/Entwurf.pdf`; any page is a witness, Entwurf is the worst.
Code: `render-lib/crates/quorra-gpu/src/encode*.rs` and the record-replay seam
(`EncodeSource::RecordReplayed`, their ADR 0087); the caller's side is
`crates/render-quorra/src/scene.rs`, which since ADR 0702/0703/0705/0706 hands over
page-space scenes that survive every view change.
Instrument: `examples/zoom_frame` with `ZOOM_FRAME_COVERAGE=compute`, whose frame line
says `record-replayed` when the encode took the replay road; the frame trace's `encode`
column in a window; `ZOOM_FRAME_ENCODE_PHASES=1` switches on quorra's
`Options::instrument_encode` for the three-phase split (under record replay, `recording`
— the seat and instance writes — is ~8 of the ~10 ms).

## The state of it

A zoom or resize step re-encodes the retained scene under the new viewport. On the lane
the window takes for that gesture (890M, ADR 0767): the step is 63–66 ms, of which the
GPU kernels are 44–46 (count 14–17.5, emit+deposit 28–31), host `encode` 9.4–10.1
(record-replayed), `residency+records` 4.0–4.2, `transfer` ~5. The scene itself is
view-free — `scene` and `handover` are 0.0 on every warm step — so those three host terms,
~19 ms together, are the whole of what the host still does per step.

What has been built and measured against it:

- **Record replay** (their ADR 0087): a replayable frame re-encodes from flat records
  instead of walking the scene, held byte-identical by its own gate. Measured **no win**
  against the walk of its day — the walk is seat/instance-bound, not command-bound — but
  against the current frame the replay road is what keeps encode at ~10 ms where the cold
  walk pays 17–18 (ADR 0767 §4), and every warm step of the measured gesture took it.
- **Device-resident records** — their ADR 0084's stage 4, the walk on the device — was
  re-priced by their ADR 0091 against the kernel numbers and lost, and re-priced again by
  ADR 0767 with the same answer: it removes at most the ~19 ms of host terms and none of
  the 44–46 ms of kernels, while its two design debts (a scan-computable seat; a refusal
  that keeps its name without a new sync) are still unargued. That ordering stays correct
  while the kernels dominate.

## The revisit condition, stated once — and in the shipped lane's numbers

Take this item up only when encode is the largest term of the step **measured with
`ZOOM_FRAME_COVERAGE=compute`** — which a successful kernel-floor round
([`46-the-kernel-floor.md`](46-the-kernel-floor.md), the flatten-from-quadratics idea, the
only item left with the step's magnitude) would make true: kernels in the 20s against the
host's ~19 leaves the host terms the largest single block. The shape to price then is
stage 4 as designed — records resident on the device, a zoom step reduced to uniform
changes plus the kernels, taking the residency build and most of the ~5 ms of staged bytes
with it — with record replay's admission machinery (structural, already built) deciding
which frames qualify, and their ADR 0091's two debts argued before anything is built.
