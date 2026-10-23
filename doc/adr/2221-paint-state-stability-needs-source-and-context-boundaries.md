# ADR 2221 — Paint-state stability needs source and context boundaries

Status: **assessment** at the merge that took it, `9cd2c665`: it records no decision a later round
must not re-litigate, as `doc/reviews/1704-batches-104-to-108-against-the-principles.md` finding 5
reads it; its acceptance of the red launch gate is superseded by ADR 2242 decision 2 (line added by
ADR 2260).

The recurring colour regressions are an ownership problem before they are a file-size problem.
A selected colour has immutable source facts: its resolved colour space, profile and component
values. Painting has mutable context: target, rendering intent, compensation and the other
conversion parameters. A converted colour is derived from both. Retaining only that result
across a graphics-state change makes a later consumer depend on selection order accidentally.

The stability boundary is one effective-solid conversion operation shared by fill, stroke,
normal text and stencils. Graphics-state saves retain the source, including the resolved profile;
a Form resource dictionary cannot silently rebind that inherited source by name. Explicit colour
selection and pattern selection replace the appropriate source. Group target entry and return
change conversion context, while alpha stays outside the converted-colour memo. A cache may
reuse the derived result only when every source and context input agrees.

This is a narrower change than a general graphics-state redesign. The existing pattern-tint
route has different ownership and execution semantics; merging it into the solid route during
this repair would increase the number of simultaneous invariants under review. Likewise, the
confined-protocol helper extraction in round 1696 is a separately proved mechanical move, not
an explanation for colour correctness. Smaller files help review, but cannot establish that
selection and painting agree about ownership.

Round 1693 therefore establishes independent controls before the implementation. The generated
ICC tables distinguish intent changes, the arithmetic oracle distinguishes plausible but wrong
colours, and nonempty glyph and stencil controls exercise actual consumers. State-transition
controls cover save/restore, profile identity, resource shadowing, defaults, reset, component
changes, target entry/return and alpha. The old implementation fails eleven of thirteen tests;
the two passing controls remain useful guards against breaking existing recovery and reset.
The corrected staged-alpha oracle is documented in ADR 2220 rather than hidden by a wider band.

The acceptance sequence is scoped checks, isolated defect/mutation controls, separately measured
ordinary ICC/tint/mixed costs, then combined workspace and fixed-corpus rendering gates. A
candidate passing one layer does not imply it has passed the others. Checkpoint commits keep
archive, toolkit diagnostics and colour work independently reviewable before integration.
The UI investigation uses the same discipline: distinguish toolkit input, host acceptance,
core commands and presentation before attributing a missed command or blurry frame.

This does not establish universal colour support or remove the remaining partial clauses.
Additional colour families, unsupported blending profiles and native toolkit presentation
require their own witnesses. Future fixes should extend this transition/consumer coverage at
the boundary they change, rather than duplicate state or add consumer-specific cache rules.

Integration also exposed a different stability problem: the FFI presentation test asserted an
incoming face immediately after replacement Open. Its test and relevant runtime files were
identical to the fixed baseline. Opening clears the old face; rendering completes on a worker.
The initial settling budget is bounded and cumulative, so dispatch cannot guarantee completion.
A controlled test lane held the real replacement request: dispatch returned with no face and a
pending draw, then explicit release and CPU completion supplied the face. No delay or larger
timeout was needed to establish that ordering.

The lifecycle test now waits for both an accepted nonempty frame and its face within the same
original deadline. This wait alone would weaken its synchronization claim, so a separate control
seeds an already-arrived immutable face and action clock, invokes synchronization, and checks the
same Arc survives with the Off command queued. An isolated deliberate face-clear fails that
exact assertion immediately. The first calibration attempt failed only at the lifecycle deadline
and was retained but excluded from the preservation proof. This is a test repair, not a runtime
scheduling change; it separates asynchronous completion from preservation of completed state.

The canonical launch gate also failed four judged warm-open bands. Its summary listed five
OUTSIDE values, but device bring-up was not in the judged failure list. Root retained the
original run and froze the exact Cargo-identified executable, workers, inputs and unchanged
bands. The fixed baseline used the previous candidate artifact: its source differs from
`0eb4d278` only in ten documentation files. This is source-equivalent evidence, not a rebuild
labelled as that exact commit.

A predeclared baseline/candidate/candidate/baseline hold ran all six documents and all 59
banded figures four times, retaining four exit-101 results. The warm-open medians in milliseconds
were baseline/candidate: BPC .384/.3835, WTPDF 1.100/1.1055, bug1815476 .363/.3635,
and opt_demo .1565/.154. Both populations failed the last two timing bands. One baseline run
also exceeded the open-memory band, 9.008 MiB against 9.000. Instructions remained in-band.
All eight guards passed; the hold cost 165 s and peaked at .20 GiB. This supports no material
candidate warm-open regression in the compared contexts, not identical performance.

A second predeclared hold used the same candidate bytes and explicit workers in
original/copy/copy/original order. Inner exits were 101/0/101/101, with all eight guards passing;
163 s held, .19 GiB peak. Original opt_demo opens were .186/.182 ms and copied opens .140/.144;
BPC was .418/.409 versus .374/.385. Copied executables also had higher resident footprints,
although their rounded allocation figures agreed. Every memory band passed in this hold.
Device bring-up was explicitly unjudged for the adapter carve-out in all eight follow-up runs.
The one passing copy does not erase the other failures or the initial larger canonical timings.

Root accepts this diagnosed launch finding without changing product code or thresholds.
The canonical gate remains FAILED. Exact pathname, inode, loader/mapping and per-executable
input-cache effects are not isolated, and baseline-original versus candidate-original interaction
was not measured. No universal performance-neutral claim follows from two repetitions per
condition. Future harness work must make execution and input-cache context repeatable and
separate allocated from mapped memory before interpreting these narrow absolute bands.
Full logs, artifact/source-equivalence receipts and independent reviews are retained under
`/home/AI/launch-ab-1692` and in the batch evidence archive. Other integration gates retain
their own verdicts; this diagnosis is not a substitute for them.
