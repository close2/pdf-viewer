# 2233 — Stability follows source and context lifetimes

Status: accepted. Session 1699, 2026-10-22.

## Assessment

Colour selection and paint-time conversion have different lifetimes. Eager conversion
loses the selected source when intent or BPC changes. Resolving a name again at painting
can instead select a shadowing Form resource. Keep an immutable resolved profile and
components, then derive output under the current complete context. Declared PDF colour
space remains separate because operators and overprint depend on it.

Reuse the existing direct-ICC source for the immediate Indexed base. Another generic
source enum or persistent cache would add invalidation obligations without evidence
that it improves this bounded fix. Independent baseline failures and omission controls
provide stronger regression protection than two implementation paths agreeing.

The same boundary discipline applies elsewhere in this batch: global DefaultRGB writes
need all-context eligibility; Qt frame borrows end in one synchronous refresh; GTK
observations report a generation without becoming another rendering authority.
This identifies concrete coupling risks, not a universal diagnosis of every regression
or evidence that a wholesale rewrite would help.

## Test infrastructure

A mutation export reused the shared Cargo target and replaced a nextest executable
while its suite was running. Cargo's build lock does not protect later test execution.
Keep mutable exports in private target directories and verify tested artifact identity.
The failed suite and serialized passing retry are both retained in ADR 2235.

## Closing launch measurement

The canonical launch gate remains failed: three warm opens and opt_demo's script
spawn/first-run times exceed judged bands. Seven numerical values are outside,
but graphics bring-up and ISO first-page time are not judged in that run.
The exact executable and workers were frozen before further builds.

A predeclared full baseline/candidate/candidate/baseline comparison retains all
six documents, 59 figures per run and eight passing load/GPU guards. The fixed
9cd baseline is the previous candidate's immutable artifact: its entire tree
outside doc is identical, not a newly labelled literal-9cd build. All four runs
exit 101. Candidate warm-open medians are lower for every measured warm-open
population: BPC .404→.3915 ms, bug .368→.3515 and opt .1725→.145. Script spawn
.8035→.7775 and first run .5235→.5095 are inside their bands. However copied
candidate open RSS rises by about 1.45–1.78 MiB across the six documents, with
four candidate-only memory failures. This difference is retained, not dismissed.

One predeclared same-candidate original/copy/copy/original control uses identical
explicit frozen workers and verifies binaries/inputs before and after each run.
All four exits are 101, with full coverage and eight passing guards. Both copy
runs pass memory bands; the final original run fails four. Original first/last
RSS rises by 1.036–1.383 MiB at the same pathname and identical bytes. The rounded
open allocation is unchanged for BPC (.8 MiB), bug (.7), XFA (.6) and opt (.6);
these rounded values do not isolate exact mapped memory or raw allocation.
This is not a stable copy-path effect. Source, loader/mapping, ASLR and changing
runtime state are not independently isolated. Both diagnostic holds cost 168 s,
peak .20 GiB; all sixteen guards passed. No sample was dropped or gate retried
until green, and no band or runtime code was changed for the measurements.

The orchestrator accepts the bounded correctness/ownership changes while keeping
the launch gate FAILED and its repeatability problem open, as with ADR 2221.
The paired timings do not establish a candidate slowdown in these populations;
the RSS observations establish instability even with unchanged source and path,
not proof of universal memory neutrality. Future measurement work must separate
allocated and mapped memory and control execution context before attributing such
narrow-band changes to code. This is a documented integration judgment, not a
passing performance claim. All logs and figures: /home/AI/launch-ab-1698.
