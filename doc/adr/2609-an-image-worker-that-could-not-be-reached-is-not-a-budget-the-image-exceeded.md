# 2609 — An image worker that could not be reached is not a budget the image exceeded

Status: implemented, in `pdf-model` (`image::ImageError::DecoderUnreachable` and the conversion
from `pdf_sandbox::SandboxError`) and `pdf-transform` (`archive::transcode`, the refusal's sentence).
Session 1887, the first robustness slot of batch 129.
Context: ISO 32000-2 §7.4.9; ADR 0458 (`WorkerMismatch` is its own variant for the same reason);
ADR 2293 (one sentence for each reason a decode gives).

## The gap

`ImageError::Sandboxed { detail }` flattened `pdf_sandbox::SandboxError` into a string, so a caller
could not tell a worker that was never reached from one that stopped on the image. The archive's
JPEG 2000 transcode answered both with "not decoded … within the ordinary decoder's budget", which
is false for a worker that was not found, would not start, or was built from another tree: nothing
was asked of the image at all.

## Decision

1. **`ImageError::DecoderUnreachable { detail }`**, beside `Sandboxed`, for `WorkerMissing`,
   `Spawn` and `WorkerMismatch`. Every other answer — the decoder's refusal, a stop, a timeout, a
   malformed response, an in-process overrun — was given by a decode that ran, and stays
   `Sandboxed`. `Connection` stays there too: a pipe that fails mid-request is a worker that was
   started, most often one a bound killed.
2. **One `From<SandboxError>`** replaces the five hand-written maps, so the distinction is made in
   one place and a sixth codec cannot forget it.
3. **The transcode refuses an unreached worker in its own sentence**, naming what was not found
   and how to supply it; the budget's sentence stays for every reached worker. The redaction's
   JPEG 2000 path, whose `Sandboxed` arm also speaks of a resolution the operator could raise, now
   gives an unreached worker its general sentence, which carries the sandbox's own words.

## What it does not reach

`quorra-transform` runs its verbs in `quorra-transform-worker`, which decodes in its own process
(`Isolation::InProcess`), so `PDF_SANDBOX_WORKER` does not reach a conversion run from the command
line. The case is the library's under `Isolation::Sandboxed` — a host, a test — and it was shown
there: a JPEG 2000 conversion with the worker named as a path that does not exist was refused with
this sentence.
