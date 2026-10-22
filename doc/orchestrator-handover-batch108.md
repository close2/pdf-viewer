# Orchestrator handover supplement — batch 108

Status: closing record for sessions 1698, 1699, 1700, 1701 and 1703.
The owner explicitly requested finishing, merging and committing this batch, then stopping.
**Do not start another round or batch without a new instruction.** Session 1702 was
prepared externally but never started; ADR 2238/2239 and its question allocation were unused.
This supplement supersedes stale current-state claims in the older handover, while retaining
that document's process history. Never change model based on its historical model names.

## Changes and evidence

- PDF/A: a narrowly proven DefaultRGB route handles the named part-4 page-intent override.
  The fixture has no catalog intent, a CMYK page intent, a neutral Gray page group and a Form
  with its own resources. Admission covers every context the global writer touches. Unknown,
  truncated, cyclic, unsupported or conflicting contexts still refuse. Neutral kTRC Gray only;
  A2B tables, arbitrary producer/supplied RGB profiles and broader consumers are excluded.
  Final scoped 656 pass/14 skip; veraPDF4 passes; final canonical archive compares 13,547
  selections with zero disagreements. Thirteen soft-mask selections remain unjudged under A61.
- Stability/spec: independent Indexed/ICC tests were committed before implementation. Fixed
  baseline passes the original thirteen and one new test; nine new tests fail intended pixel
  assertions. Candidate passes all 23. Immediate Indexed-to-ICC retains resolved components
  and profile through current-context painting, using the existing source/cache boundary.
  Five omission controls fail intended assertions. Serialized model/colour suite: 2311 pass,
  21 skip. Required work adds bounded measured cost; this is not performance neutrality.
  Section 11.7.5.3 remains partial; other Indexed bases, calibrated/Lab/spot source families,
  Indexed process-tint overprint and output-intent policy remain separate obligations.
- Qt: one synchronous C++ image sink collects a refresh's placements once and owns copied
  QImages before borrowed slices end. Ordinary query count is 1 instead of 1+2N; transitions
  query zero. No new unsafe block or persistent cache. Scoped 43 pass/1 skip and five CPU
  display scenarios pass. Retrieval cost falls, but full copy/delivery paired ratios and
  ranges establish no overall latency gain. The owner's Qt lag remains unresolved.
- GTK: opt-in completed-source provenance, generation/paint joining and finite settled
  capture instrumentation. ISO document page 20 CPU captures at fit, two key zoom-outs and
  fit return map independently by visible page edges at 1:1. Entire opaque page/interior
  text pixels match exactly. No presentation blur is added in those settled CPU states.
  This does not judge source resolution, transient stand-ins, deep zoom drift, compositor
  scaling or native Wayland. Real-adapter result and final integration receipts follow below.

## Stability lessons and failed attempts retained

Source selection, declared PDF colour space and paint context have different lifetimes.
Keep resolved immutable identity, then derive output under current context. Do not add
another cache or re-resolve resource names without independent state-transition controls.
A mutation export reused the shared Cargo target and replaced nextest's executable during
execution: the first suite had 20 failures. The preserved executable embeds the export path;
the serialized suite passes. Mutable exports require private targets; Cargo's build lock
protects builds, not later execution. Use target configuration or explicit --target-dir,
not CARGO_TARGET_DIR (the owner-loop environment is the owner's, not a precedent for builds).

Qt borrowed-vector and wrapper CXX designs failed actual Rust generated lifetime shims;
those attempts are retained. A Gray-LUT negative control exposed over-admission in the
archive proof; the final guard excludes A2B tables and the canonical archive was rerun.
GTK's first binary-target test invocation ran zero tests; the corrected library selection
runs four. The first Xvfb wrapper assumed a lock file -displayfd did not create and stopped
before viewer launch. Every failed attempt remains evidence, not a green verdict.

## Stopped state and next-session entry

Read current git status and installed-from rather than old hashes in the original handover.
Preserve owner edits, shared stash and doc/adr_revisit. Do not push. Q566 and /tmp/folding-exact
remain untouched. No mechanical 1702 patch was applied. The optional preparation under
/home/AI/prepared-1702 is not completed work or authorization to restart.

The owner GPU loop is solely for measurements requiring the real adapter/display, never
builds, installs, privileged file access or escaping the agent account. Stage built artifacts
as AI, use an exclusive clock hold, finite timeout, owned-window/PID checks and Escape with
bounded fallback. A live owner loop after handover is not an active agent job.

The next UI investigation, if authorized, should distinguish source sharpness, transient
stand-in scaling and compositor scaling. Reproduce native Wayland and deep zoom anchor
behavior independently. The existing GPU estimate investigation is recorded in ADR 2196:
compatible-view/coverage samples and seed provenance were addressed; a seed can remain
constant when no compatible measured GPU frame exists. Do not describe it as wholly
uninvestigated or claim this batch solved the user's remaining UI symptoms.

## Final renderer and display receipts

All six final renderer arms pass on immutable ae6f6fcb: 968 rows each at 1×,
965 each at 4×. Every ordered name, digest and mean-error field matches fixed
9cd2c665 exactly; only timing columns differ. Hold 1550 s, peak 3.63 GiB.
`/home/AI/final-arms-review-1698.json` contains hashes and the complete comparison.
Subsequent product-source changes: none; only display diagnostics and documentation.

The real-adapter attempts both refuse before capture. Trial 2 observes Radeon 890M/RADV,
an unlocked active Wayland session, and the correct active/focused test window, but
xdotool's requested warp leaves X11 pointer position unchanged at (1849,547), window 1041.
GTK receives no motion entry. The owned-window screenshot shows the intended PDF.
Hardware sharpness is therefore unjudged. Do not weaken calibration or infer a locked
session from this failure. No compositor cause was established. Both trials terminate;
Escape times out and the bounded TERM fallback closes only the owned process.
Receipts and screenshots: `/home/AI/gpu-evidence-1703/`; CPU results and independent
mapping script: `/home/AI/gtk-capture-1703-cpu1/`, `/home/AI/analyze-gtk-1703.py`.

## Combined tier-one validation

All checks pass: workspace formatting, warnings-denied all-target Clippy,
7,649 workspace tests with 54 skipped, workspace doctests, conformance,
fuzz formatting/Clippy/stable/nightly checks and private workspace rustdoc.
Nextest test time is 142.464 s; bounded build-and-test hold is 422 s, peak 8.59 GiB.
The closing batch check passes (21 s): population, record budgets, workspace membership,
fuzz workspace and formatting are clean. Receipt: `/home/AI/batch-check-1698.log`.

## Launch gate: retained failure and diagnosis

The canonical launch gate fails five judged timing bands. The complete balanced
baseline/candidate/candidate/baseline comparison retains four failed runs and all
236 figures; candidate warm/script medians are lower, but copied-candidate RSS
is about 1.45–1.78 MiB higher. A same-current-binary original/copy/copy/original
control also retains four failed runs. Both copy runs pass memory bands; the final
original run fails four, with 1.036–1.383 MiB more RSS than its first run at the
same pathname and bytes. There is no stable copy-path effect or isolated cause.
All sixteen guards pass; both holds take 168 s, peak .20 GiB each. No thresholds,
product code or samples were changed to obtain acceptance. Canonical failure remains.

ADR 2233 records the integration judgment: retain the bounded correctness/ownership
changes, with this launch repeatability problem explicit. Neither universal timing
nor memory neutrality is established. Full values, eligibility, frozen binaries,
source equivalence, failures and controls are in `/home/AI/launch-ab-1698/` and
`/home/AI/launch-frozen-1698/`. Do not repeat these runs looking for a green result;
future work should isolate allocation/mapping and execution-context effects.


## Closure receipts

Final canonical gates: **38 of 39 pass**, 2,920 s of summed gate wall time.
Only `t2-launch_path` fails (exit 101), with the retained integration judgment above.
All corpus, archive, VFS and confined-viewer gates pass. Full receipt:
`/home/AI/batch-gates-1698.log`. Installation and archive
happen after the final documentation commit, so their authoritative receipts are external:

- Main `target/installed-from`: exact commit and SHA-256 of all fourteen installed files.
- `/home/AI/install-1698-verified.json`: independent installed-file/hash/HEAD verification.
- `/home/AI/evidence-batch108-1698-1703.tar.zst`: complete batch evidence, including failures.
- `/home/AI/evidence-archive-1698.json`: archive hash and verified member/source population.
- `/home/AI/closure-1698.json`: final merge/install/archive/close result and owner-state checks.

Read the receipts themselves; a listed destination is not proof that a later operation
succeeded. The archive must verify before removing the shared worktree. Preserve unrelated
GPU pending files. No push and no next batch are part of this closure.
