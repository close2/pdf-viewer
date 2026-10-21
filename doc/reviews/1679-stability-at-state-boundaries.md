# 1679 — Stability at state boundaries

Status: **reviewed**, against `5ded4916` and the independently owned changes of this
batch. Requested by the owner on 2026-10-21: improvements regularly break other behavior; examine
whether intertwined design is responsible. This review changes no production code.

## What the evidence supports

There are concrete opportunities to reduce the number of things a change must keep synchronized.
They are concentrated in the archive pipeline, derived graphics state, platform capabilities and
host interaction state. A large source file is a useful place to investigate, but moving its
functions into smaller files does not by itself remove these dependencies.

The evidence does **not** establish that every recent regression has the same cause. The Windows
compile failure and the macOS assertion are demonstrated failures. The archive's parallel wiring
is a demonstrated maintenance burden. The stale-dialog concern below remains a hypothesis until
the question guards and event sequences establish reachability. These deserve different claims
and different tests.

## 1. Make platform capabilities a boundary contract

**Observed failure.** CI run `39555931043` on `5ded4916` failed Windows because
`pdf-transform/src/confined.rs` used Unix descriptor conversion and `FileBytes::descriptor`
without a platform boundary. macOS failed the large-grid encoder test because it asserted Linux
confinement. The Windows CLI already had an announced in-process route in
`pdf-transform/src/bin/quorra-transform/route.rs`; the missing distinction was below that route.

`doc/verify.md` names cross-target checks for the sandbox, rendering, confined viewer, UI and ABI,
but the baseline list omitted the transform package. A Linux workspace check cannot validate code
excluded by a target's `cfg`. This is a coverage gap with a specific witness, not a need for more
random test volume. Round 1674 supplies the immediate fixes and targeted platform checks.

**Next structural step.** Keep descriptor operations in the Unix transport implementation and
expose the byte route independently. Distinguish what a worker does from what the operating system
enforces; a successful encode is no proof of confinement. Have one maintained platform-check
package list consumed by both the local command and CI, including each shipped binary's dependency
graph. Avoid another prose list that can drift.

**Acceptance test.** The old unguarded descriptor call must fail the Windows check, and an
unconditional enforcement assertion must fail a non-Linux test. Keep the Linux enforcement
assertion: removing it would make all platforms green by deleting the question.

## 2. Give an archive remedy one prepared payload

**Observed coupling.** `archive/decision.rs`, `archive/prepare.rs` (`Prepared` and `Owed`) and
`archive/rewrite.rs` (`Rewrite` and `Rewriter`) carry related but independently maintained pieces
of each remedy. Adding the widths-array remedy requires a requirement identifier, decision,
rewrite description, preparation result, refusal, application and report count. The corpus also
needs to know which invariant the remedy deliberately changes.

A read-only `git diff-tree` census of the last fifteen batch commits found changes to
`decision.rs` and `rewrite.rs` in every commit, and to `prepare.rs` and `mod.rs` in fourteen.
The batches changed between eighteen and 143 production source files. This measures exposure to
coordination mistakes; it does not count regressions or prove causation.

**Next structural step.** Pilot a typed prepared payload for one remedy family. Its preparation
returns either the exact operation that can be applied or a named refusal. Application consumes
that payload and returns its report, instead of independently rediscovering the applicable objects
and maintaining a second count. Keep the decision catalogue declarative and exhaustively matched.
Do not introduce a general plugin framework or migrate every remedy at once.

The second-verdict boundary also loses useful evidence: `hold_the_output_to_the_target` computes
a full validation report, but `Achieved` retains failing requirement identifiers rather than the
font, code and actual advances behind each finding. Diagnosing `316267.pdf` therefore required
temporary instrumentation. Preserve typed findings through the refusal/report boundary so a
regression can be explained without changing the program to recover facts it already computed.
A decision currently admits one loss, although one font rewrite can lose both machine-face choice
and substitute-width fitting. Any extension must represent both explicitly before it can authorize
such a rewrite; one permission cannot silently stand for another.

**A demonstrated write-ownership defect.** In `316267.pdf`, several font dictionaries share a
font descriptor but state different advances. Preparation proves each font's replacement program
separately, then `substitutes.at.insert(descriptor, ...)` uses the shared descriptor as its write
key. A later space-only font can replace the program proved for the earlier font. A focused
shared-descriptor test fails at the second verdict before the fix. Round 1671 isolates the
replacement descriptor per font where sharing would make these writes conflict, retaining the
original descriptor entries. The structural invariant is that a proof's object scope must match
the scope of the write it authorizes; smaller source files alone cannot establish it.

Ordering must stay explicit. `Prepared` already documents real dependencies: replacing an XMP
packet precedes edits by span, and metadata sees the packet left by earlier edits. Hiding that
order behind arbitrary registration would make stability worse. A typed plan should make
conflicting writes to the same packet visible before serialization.

**Acceptance test.** For the pilot family, exercise validator finding → decision → prepared
operation → serialized output → second verdict. Assert the promised preservation property, not
just that the second verdict passes. Round 1671's widths tests compare both drawing and advances;
a conforming output with changed marks is still wrong. Plant a missing application and an incorrect
report count separately and require the small tests to reject both.

## 3. Separate graphics-state inputs, cached paint and diagnostics

**Observed coupling.** The interpreter's `black_generation_stated` flag can be set when a pair is
declared, while the report concerns a color actually painted under that pair. Graphics state is
saved and restored; a departure already encountered is monotone. Giving these two lifetimes one
flag makes declaration and use easy to confuse. Round 1675 moves the diagnostic to paint events.

There are also several conversion paths: selected fill and stroke colors, patterns, images and
shadings. A new image-paint witness in round 1675 failed: `content/image.rs::conversion_under_intent`
constructed a conversion without passing the current black-generation pair. The solid and shading
paths supplied it. This is a concrete example of several call sites independently assembling what
should be one conversion context. The corrective color-output test passes in that round: the
image changes from `[51, 102, 154, 255]` to the expected `[52, 62, 79, 255]`.

A separate solid-color defect is now reproduced in both the baseline and candidate. Selecting
RGB `(0.2, 0.4, 0.6)` before applying `BG(k) = 1-k`, `UCR(k) = k/2` paints
`[51, 102, 154, 255]`; applying the pair before selection paints the expected
`[52, 62, 79, 255]`. The pair in force at paint is identical. The selected color caches a
conversion whose dependencies subsequently change. `pdf-model` example
`black_generation_order` preserves this witness; it is separate from the image-path fix.

**Next structural step.** State the complete dependencies of a cached paint value. Either retain
source color plus an immutable conversion context until paint, or key the derived paint by the
relevant state generation. Choose after measuring the affected path; do not add work to every mark
without a price. Keep departure accumulation separate from the save/restore graphics state.

**Acceptance test.** Compare equivalent operator sequences around color selection, parameter
changes, `q`/`Q`, Form entry/exit and repeated painting. Include unused declarations, a selected
but unpainted color, Gray and CMYK controls, and image/shading paths. A test that only appends a
`gs` operator cannot establish a rule whose condition is painting.

## 4. Check asynchronous answers against the state they answer

`viewer-ffi/src/window.rs` stores the current question in `Option<Asked>`;
`put_the_question` assigns it and `answer` consumes it. The owed question and answer do not carry
an explicit generation. A managed host can queue dialogs while the library's timer continues.

This is a plausible stale-answer hazard, but `question_waiting` and the callers' guards may prevent
replacement. The current UI round audits those guards and invalidates replaced queued cards.
Do not label an unreachable sequence a demonstrated bug, or add ABI machinery solely from this
shape of code.

**Next structural step if the sequence is reachable.** Carry a request identity through the owed
record and its answer, and reject an answer to an expired request. Centralize cancellation when
the document or window closes. A host should not need to reconstruct native lifetime rules from
the order in which records happen to arrive.

**Acceptance test.** Drive open → question → cancel/replace/document switch → delayed answer,
including a withdrawn script prompt and window closure. Check both the returned status and that
the newer request was not answered. These are state-machine tests; compiling the C# and checking
the C header cannot cover them. Windows interaction and XAML validation remain separate checks.

## 5. Make parallel work reviewable in small semantic units

The current batch has explicit ownership and captures the unchanged baseline before edits. During
this batch a cross-target check encountered a sibling's incomplete CFF API edit; the subsequent
font tests passed after the correction. That transient failure is expected in a shared worktree,
but it is not evidence about a finished change.

Keep independent implementation in parallel, and integrate shared semantic changes sequentially.
For a cross-cutting architecture change, first ask an agent for a read-only plan and exact
invariants. Give one writer the approved file set. Frozen exports already provide isolated
measurement inputs; use their commit, patch population and binary hashes when attributing a
regression. A file's current contents are not proof of what a previously built binary contained.

The rendering gates also have a scope boundary: cold corpus frames cannot test a defect that
exists only after retaining a page and changing its view. The kept-line work must compare retained
and fresh draws across zoom, edge-admitted lines, adjusted widths, quarter turns, reflections and
overlap order. A clean cold-frame digest is not evidence about the replay path. Give each stateful
optimization a sequence witness alongside the ordinary page corpus.

For future batches, consider separate topic commits or worktrees for independent rounds and a
dedicated integration step. This would make bisection and review more precise than a single broad
batch commit, at the cost of conflicts and additional build storage. This review does not change
the current batch's prescribed commit procedure.

## Recommended sequence

1. Finish the current narrow fixes and their cross-boundary witnesses; retain all required corpus
   and platform gates. Do not mix a pipeline redesign into those fixes.
2. Pilot the typed archive payload in one family, with preservation and second-verdict tests.
3. Audit derived paint dependencies and add operator-order fixtures before changing caching.
4. Complete the dialog lifecycle audit; add request identities only if the guards do not already
   establish the invariant.
5. Keep god-class splitting and comment cleanup as behavior-preserving work, and measure success
   by fewer independently maintained invariants as well as shorter files.

The follow-up should report which planted failure each new small test detects, which shared state
was eliminated, and which files a new operation must still edit. Those are more useful stability
claims than another total test count.
