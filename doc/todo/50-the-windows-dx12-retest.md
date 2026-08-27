# The Windows DX12 retest: every fix since the pathology is unmeasured there

Status: **blocked — on a machine only the project owner has.** This was point 9 of the
seven-point GPU round, excluded because it is the owner's to run.
Priority: 50 — blocked on hardware access, not on a decision.
Corpus: the owner's own windowed runs; `tmp/entwurf.trace.txt` is the standing Windows
trace this file's history comes from.
Code: nothing here — the work is a measurement.
Instrument: `target/pdf-viewer` (or a Windows build of it) with `--trace=frames` on the
owner's Intel UHD / DX12 machine: cold open, first present, a zoom gesture, a drag.

## Why it exists

The one Windows trace this project has ever seen found a genuine pathology: quorra's
`flush_atlas_tiles` issued one `queue.write_texture` per coverage tile — ~58 000 calls at
~110 µs each, **6.4 s** of transfer on DX12 against 65 ms for the same page on Linux/
Vulkan. Quorra's ADR 0078 batched the flush into one staging buffer and the Linux number
moved (windowed Entwurf first present 2721 → 1042 ms on the 890M); **the DX12 number has
never been re-taken.**

Everything since is equally unmeasured there, and DX12 is the backend most likely to
price it differently: the compute lane (their ADR 0080/0081/0083), the per-frame hybrid
(ADR 0090/0093), and stage B's one-submission chain with its end-of-frame readback
(ADR 0095) — readback timing and queue semantics are exactly where DX12 and Vulkan
diverge.

## What to run, when the machine is available

One traced session: cold open of Entwurf and of the ISO specification, first present,
ten zoom steps, a drag. Compare the trace's columns against the Linux numbers recorded
beside quorra ADR 0095. Anything that is off by a shape rather than a constant — a stall
span that exists only there, a first present that did not move since ADR 0078 — becomes
its own item with the trace attached.
