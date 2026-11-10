# Specification audit — working checkpoint

Baseline: `e85f8545eae1c95a5ff7f92b7535c3a7a535dabc`, after the owner's pull on
2026-11-10. This audit checks specification coverage, implementation, and the meaning
of test assertions separately. It is in progress; this checkpoint is not a claim
that every completed ledger row has been verified.

The baseline ledger contains 883 rows, including 775 marked `implemented`.
Its 2,303 distinct test references all name existing files. Some references name
support files or whole targets rather than executable tests. Finding or executing
a reference does not establish that its assertions prove the associated feature.

## Latest checkpoint

The latest broad run covered nine crates: **5,713 tests run, 5,670 passed, 43 expected audit
failures, 53 skipped, no unexpected failures**. Several failures share a cause. This includes
the presentation, held-operation, font-width, border and annotation-state tests. Production behavior, ledger statuses and
historical ADRs remain unchanged. Selected assertion reviews and isolated mutation checks have
exposed both implementation gaps and passing tests unable to detect their stated defect.

This run includes the PaintType, OpenType, n-up watermark, markup, annotation-state and ink additions.
The observed failures exactly match the committed manifest.

This remains a partial audit. The reproducible continuation inventory and per-clause/ADR progress
are committed in [the audit evidence directory](1983-specification-audit/), with full local working data
under `scratchpad/r1983/spec-audit/`; no whole-clause or whole-ADR completeness percentage is
inferred from test counts. Detailed checkpoints below preserve the scope of each run.

## Source coverage

The source inventory accounts for all eight normative annex roots and all 411
table captions through a ledger clause or ancestor. This establishes structural
coverage only. Individual table entries, conditions, defaults, notes, and errata
still require semantic review. The source's duplicated Ruby/warichu heading is
corrected by Issue 133; it is not a missing ledger row. Text after the main
bibliography includes errata material and must not be counted as duplicate clauses.

The original PDF's annotations have been indexed locally. The Markdown conversion
can omit corrected table text, so a Markdown-only review is insufficient.

## Findings validated so far

* Annex Q.2: `blends` checks whether any blend-mode array member is non-Normal.
  It should use the effective first recognized mode. `[/Normal /Multiply]`
  incorrectly marks an otherwise opaque page transparent.
* Annex Q.2: the survey marks a page transparent when `gs` sets a transparency
  parameter, even when `Q` restores the opaque state before any element is drawn.
  The incorrect page classification reaches PDF/A transparency checks.
* Clause 7.4.3, EC3 Issue 293: both ASCII85 decoder routes accept `z~x` as ending
  successfully. The effective erratum requires `>` immediately after `~` and an
  error otherwise. Separate regressions fail for the buffered and windowed routes;
  the valid marker and canonical partial-group controls pass.
* Annex I.2's completed-row explanation says no PDF-insertion operation exists.
  `pdf-transform` now has one. Inserting PDF 2.0 content into a PDF 1.7 document
  produces two pages but leaves the effective version at 1.7. This misses a
  recommendation, not a mandatory requirement. The reverse-order control preserves
  the newer destination version. Newer-version and unmet-requirement warnings also
  omit the annex's recommended warning about possible inability to change or save.
* Clause 7.9.4: metadata editing publishes `D:20261301` (month 13) and
  `D:2026010` (an incomplete day). Its validation checks the prefix, initial year
  digits, and alphabet rather than complete field shapes and ranges. ADR 0855
  documents that restricted check, but it does not meet the producer's date
  requirements. Two writer regressions fail; valid partial and zoned controls pass.
* Clauses 7.9.2.2.2 and 14.9.2.1: a Unicode `/ActualText` string's language
  escape is removed from visible text but does not override the prevailing
  language in speech. The ordinary `en-US` control passes; escaped `es` is
  incorrectly spoken with `en-US` metadata.
* Clause 7.9.2.3 and Table 246: FDF `/Status` requires PDFDocEncoding, but its
  reader applies text-string BOM detection. Valid bytes `FE FF`, independently
  checked in the parsed fixture, become empty text rather than `þÿ`.

The new Annex Q test module has eight passing tests and two failing regression
tests for these defects. An earlier apparent annotation-blend failure was caused
by a malformed test fixture and has been discarded. The corrected fixture passes.
Production implementations and ledger statuses have not been changed.

The new overprint test initially passed with a deliberately inverted stroking
flag. Its expected-output assertions were strengthened. The correct implementation
now passes; independent mutations that invert the flag, reverse `OP`/`op`
precedence, or remove fill inheritance each fail the test. These experiments ran
in an isolated worktree, whose production source was restored afterward.
An LZW reset assertion was also strengthened: literal-only codes could not detect
an ignored reset. It now reuses a code with a different dictionary meaning after
the reset. Ignoring the reset in the isolated worktree changes the output from
`ABABDEDE` to `ABABDEAB`, failing the test; the control passes and source is restored.

## Execution and remaining review

Baseline suites for syntax, the document model, colour, fonts, signatures, archive,
transforms, viewer core, rendering, scripting, and host crates have run successfully.
Of the 2,303 distinct references, 2,278 map to observed passing executions, 12 to
wholly ignored tests, five to targets with passes and ignored tests, and eight to
support or non-Rust sources without a corresponding execution. These are reference counts, not unique test counts or verified features.
Ignored tests and the Windows test program require separate accounting.

The ledger-specific conformance target passed eight tests. The broader conformance
run encountered an existing documentation-command failure: the vendored
`doc/zune-image/crates/zune-heic/README.md` names nonexistent package `zune-benches`.
Nextest was absent and has been installed locally under the excluded audit tools
folder. Completed waves and their scoped gates are recorded below; a later
addition is not covered by an earlier successful gate.

Annex Q, the ASCII filters and RunLength, and Annex I have reviewed feature matrices
with explicit gaps and limitations. Annex I's first-error suppression recommendation
still needs assessment across error channels. The initial syntax and graphics batches
contain useful source inventories and selected verified tests, but their remaining
features must not be inferred complete from generic implementation pointers.
Further narrow batches are running. Missing unit tests are authorized; production
fixes are outside this audit's current work.

The lexical follow-up has four passing tests. Object/xref-stream tests now include
valid hybrid self-entry and pure `/Prev` fixtures, plus unsorted object-stream
members with adjacent values. Graphics-state follow-up tests cover UCR2/TR2
precedence, transfer reset, and smoothness propagation. The smoothness test measures
function approximation error; an earlier assumption about increasing sample counts
was rejected. Tree writer/reader tests check numeric rectangle bounds, flat-root
shape, byte ordering, and key/value pairing. Broader claims within those clauses
retain explicit residual gaps.

## Resume data and budget

Reviewed findings, selected ADR decisions, the expected-failure manifest, gate summaries, and
continuation inventories are committed in `doc/reviews/1983-specification-audit/`. The dedicated
branch is `audit/specification-evidence-2026-11-10`; reviewed batches are committed and pushed
regularly. Failing audit regressions deliberately assert the required behavior and remain enabled.
The branch is evidence for follow-up fixes, not a claim of a passing implementation. Raw source
extracts, account logs and unreviewed test drafts remain local.

Local working data is under `scratchpad/r1983/spec-audit/`: `baseline.json`,
`ledger.json`, `queue.json`, the source annotation index, per-reviewer feature and
row matrices, execution logs, mutation results, and `usage.jsonl`. This directory
is locally excluded from Git because it includes licensed source extracts.
`reviewer-brief.txt` states the evidence standard for fresh reviewers.
`dispatch.json` and `clause-progress.json` record active and deferred scope.
`reconcile.py` assembles `effective-features.json` using baseline ledger statuses and
parent review overrides. It preserves superseded pilot evidence and separates
execution from verdicts; overlapping review records are not a feature denominator.

The live weekly allowance started at 52%; the check at 11:00 UTC on November 10 was 60%.
The owner allocated approximately 40 percentage points of the full weekly limit.
Stop assigning new batches at 87% and reserve the remainder through approximately
92% for review and handover. Weekly allowance is not a fixed token quantity;
cached input, model choice, and other account usage prevent a reliable conversion.
Use `usage.py` for fresh readings and account for a reset before comparing them.
Preserve unreviewed requirements in the backlog rather than weakening the audit.

## Follow-up checkpoint

The five changed crates completed a Nextest run: 4,686 passed, ten expected defect
regressions failed, and 50 were skipped (`nextest-five-crates.log`). This includes
the separate property-list language regression. Subsequent additions need their
own recorded execution; these figures describe that run only.

Clause 7.3.10 now has a further reproduced defect. A valid one-page PDF defines
object `4 0`, while its catalog contains a reference to undefined `4 1 R`.
Both a cold lookup and resolution after caching `4 0` return its integer value
instead of null. The valid-generation controls pass. The two regressions and five
basic-object controls are recorded in `basic-objects-resolve-target.log`.

The Type 3 follow-up has three passing tests for rotated width advancement,
glyph graphics-state restoration, and actual knockout compositing. Graphics
checks now include nested LIFO restoration and construction without painting.
Further Type 3, stroke, and path-painting review is underway; producer constraints
remain requirements even where they do not require malformed-input rejection.

An apparent full-table LZW reset failure was a stale compiled mutation artifact,
not a production defect. Rebuilding unchanged source restored the passing reset
controls. Mutation scripts now use their worktree's separate target directory.
The full-table fixture was additionally corrected for the encoder/decoder table
lag and checked for novel adjacent byte pairs; its current result is recorded
separately from the superseded draft. The conformance rerun still encounters the
same vendored `zune-benches` documentation-command failure.

The allowance reading at 10:05 UTC was still 57%, against the 52% starting point.
Review continues within the previously stated dispatch and handover thresholds.

The next expanded run, now including CPU renderer tests, completed with 4,960
passes, 13 expected regression failures, and 50 skips
(`nextest-six-crates-followup.log`). Formatting and warning-denied Clippy passed
for the six touched crates; workspace rustdoc passed. The additional failure is
Type 3 Table 111 transparency handling: a `d1` glyph applies `/ca 0.5` instead of
ignoring it. The paired `d0` control resolves the same resource and correctly
applies half opacity before the failing assertion is evaluated.

Baseline reference accounting has been refined without changing execution:
the earlier 17-reference ignored category comprised 12 wholly ignored references
and five target-wide references containing both passes and ignored tests.
`test-execution.json` now distinguishes those mixed targets and records whether
matching was by named test or by target. Neither category proves assertion quality.

The 10:33 UTC allowance reading was 59%, seven points above the starting value.
The audit expanded to file structure, stream objects, Annex K, and the real host
route for Annex O embedded documents. Those active shards remain subject to
parent review and execution; their draft verdicts are not accepted findings.

The path-painting follow-up now has eight passing model/raster tests plus a
passing CPU cap-versus-join test. They check exact command geometry and colours,
implicit closure of disconnected subpaths, and single-object compositing versus
separate-object controls. Alpha checks allow one 8-bit quantization level, well
below the difference between one and two compositions. The new raster evidence
is specifically for the CPU backend. Four further passing viewer tests cover
multiword search ordering, both zoom offsets at two scales, reversed nested
structure-element order, and matched-empty-element fallback from page two.
The latest targeted logs and feature records distinguish these from residual
producer constraints and from the pending embedded-document host test.


## Current extension wave

The actual Qt embedded-document route is now tested: the fragment requests the
embedded PDF, policy permits it, arrival opens a new tab, and the fragment selects
its second page. The host regression passes; all 47 enabled Qt tests pass, with
one skipped. Warning-denied Qt lint passes. Annex O additionally has a passing
OpenAction-versus-fragment ordering test. Producer placement advice stays pending.

Two new stream tests pass: external streams are refused on all three public data
routes, while identical embedded data decodes; the corrected Example 3 has its
449-byte encoded extent, including the ASCII85 terminator, and decodes correctly.
The original PDF corrected a transcription error in the draft fixture.

XFA tests now read nonempty values from whole-XDP and packet-array forms and
verify both saved PDF `/V` and XFA datasets after text changes, including XML
metacharacters. A viewer-protocol test proves the report identifies XFA. Automatic
UI reporting, other field kinds, and the remaining producer constraints are not
inferred verified. These four new tests pass.

Two further defects have passing controls and failing regression assertions:

* §7.5.6: incremental serialization omits a custom second-class trailer entry
  from the newest trailer. The reader's merged trailer masks the omission, so the
  assertion checks the newest section's own dictionary. Source retention, applied
  language edit, two sections, `/Prev`, `/Root`, and `/ID` are checked first.
* §7.10.4: when the first stitching bound equals the domain's lower endpoint,
  evaluation selects the second interval instead of the singleton first interval.
  It returns `0` instead of the first encoded value `0.75`. Seven function controls
  pass, including ordinary boundary selection, final-endpoint handling, singleton
  functions, exponential evaluation and input/output clipping.

Warning-denied lint passed for the new syntax, model, viewer-core and colour
files (`clippy-new-wave-fixed.log`). The expanded eight-crate Nextest run passed
5,198 tests, failed the 15 expected regression assertions, and skipped 52
(`nextest-eight-crates-wave3.log`). Later tests need separate execution. Sampled
and calculator functions, and Annex L's role-mapped merge structure, are under review. Production implementations and ledger statuses remain
unchanged.


The owner also authorized reconsidering ADR decisions within the same budget.
`adr-inventory.json` lists all 2,233 ADR files and their direct ledger-note links;
that is an inventory, not a review. Priority goes to decisions behind departures,
newly demonstrated defects and stale scope premises. Reviews will distinguish
individual decisions within an ADR, historical justification, current evidence,
owner-ratified policy, and experiments still needed. Historical ADRs remain intact.


## ADR and calculator follow-up

The first decision reviews retain the ledger vocabulary, with a qualification:
aggregate logic proves status relationships, not that a parent clause has no
requirements of its own. The eight current aggregate headings were checked;
Annex K required the original PDF because its Markdown heading is misplaced.
ADR 1035's temporary departure policy was superseded by the owner's A63 and later
ADRs; it is not a current defect.

ADR 0855's metadata date check still needs reconsideration: the demonstrated
invalid dates are producer output. Other decisions in that ADR have distinct
outcomes and successors; the whole ADR is not rejected because of this fault.
The `/Size` census supporting ADR 2346 was rerun: 66 current corpus files have
reachable entries above `/Size`, with 65 catalogs above it. Its historical timing
and patched-reader comparison were not rerun and are not presented as current.

The core Annex L merge construction still holds. The new role-mapped merge test
passes. Preserving inherited source structure remains a reasonable derivation
policy, with a qualification: it does not guarantee that arbitrary input becomes
Annex L-conforming output. A source precondition or validation is needed for that
stronger claim. Historical ADRs and ledger statuses remain unchanged.

Calculator diagnostics reproduced three silent error fallbacks: extra outputs are
truncated, Boolean outputs become numbers, and stack overflow loses values and
returns a padded result. These conflict with §7.10.5.3's error requirements. The
public evaluator returns only numeric values, so the observations stay in local
audit artifacts rather than becoming tests that demand the incorrect behavior.
Four correct-behavior calculator tests pass, including an input-dependent program
with an abstract peak of 100 stack entries. Ten sampled-function tests also pass.

The normative PostScript Language Reference, third edition, was retrieved from
[Adobe](https://www.adobe.com/jp/print/postscript/pdfs/PLRM.pdf), with provenance and
hash in `source/PLRM3-source.json`. The older “not held” limitation is now removed
for this audit; arithmetic semantics are being reviewed against it. The 11:17 UTC
weekly allowance reading was 61%, nine percentage points above the start.

The ADR 0855 follow-up inspected the actual staging, refusal and metadata assertions, then ran all 22 VFS write tests successfully. The separately inspected real confined-worker consent test also passed. The similarly named test in `a_write.rs` uses an in-process worker; its name alone was not accepted as confinement evidence. Live FUSE callback behavior remains untested in this audit.

The date-related XMP hypothesis was corrected during code review: an invalid date is omitted from supplementation after its property is scheduled for removal, so an existing packet date can be removed rather than left unchanged. That consequence still needs a dedicated observation; the independent invalid-date writer regressions already establish the grammar defect.


## Direct-reference and assertion follow-up

The direct PLRM3 review changes the bit-shift decision in ADRs 0369/0371:
right shifts must insert zero bits. The warning about arithmetic on positive
values does not permit sign extension on negative values. A valid runtime-input
regression checks the sign after shifting without fixing an integer width; its
positive control passes and its negative-input assertion fails. Existing tests
that require sign extension test a deliberate departure, not this requirement.

The remaining calculator tests were revised before promotion: Boolean results are
consumed by explicit conditionals so every function returns numbers; XOR includes
both-true inputs; copy uses distinct multiple items; roll checks both directions,
wrapped counts and the untouched stack prefix. Seven tests pass and the zero-fill
regression fails. Two arithmetic boundary tests pass. Nine invalid-arithmetic
cases were observed through the public evaluator in a temporary target, then that
target was removed; no permanent test demands the incorrect fallback values.

Two isolated mutation experiments confirm sensitivity of the exponential and
sample-array tests: ignoring the exponent and reversing dimension order each
fails its intended assertion, with passing unmodified controls.

The filespec URL-escaping defect proposed by a reviewer was withdrawn. Its input
would violate the already-encoded URL form's own requirements. A valid encoded
URL now passes through the Viewer to the exact expected host request, but that
proves neither ordinary file-byte conversion in a URL filesystem nor network
fetching. The host's current target-root path refuses absolute URLs as local
imports. Three model file-spec tests and two embedded-file tests pass; no-cache
behavior and conditional writer fields remain separate residuals.

Two page-resource tests pass: the nearest ancestor dictionary is inherited
without merging, and an explicitly empty page dictionary stops inheritance.
These assert the parsed page model, not rendered resource use.

The next eight-crate run (`nextest-eight-crates-wave4.log`) ran 5,249 tests:
5,233 passed, the 16 expected audit regressions failed, and 52 were skipped.
Warning-denied lint passed for the four crates touched in this wave. At 11:38 UTC
the weekly reading remained 62%, ten percentage points above the start.


## Attachment writer and inapplicable-condition follow-up

Two more actual writer regressions fail, bringing the targeted expected failures
to 18 (the latest full eight-crate run still predates these two). Document-home
attachments can enter the EmbeddedFiles name tree without Table 43's required
`Desc`. The PDF/A-4f preservation route can emit a catalog-associated embedded
stream without Table 45's required `ModDate`. Both tests first exercise a positive
control and allow the writer to refuse an input it cannot publish conformingly.
The known-date control compares parsed dates, allowing normalization of spelling.
Warning-denied lint passed for both affected crates.

These findings justify revisiting the specific optional-field decisions in ADRs
0814 and 1270. They do not settle what truthful description or date source the
product should use. Historical ADRs and production behavior are unchanged.

All 32 baseline `inapplicable` rows received a condition review, separately from
feature enumeration. Generic writer preservation of deprecated constructs deserves
an explicit policy qualification, but the review has not established a mandatory
conformance failure from that fact. The proposed Web Capture naming defect in
ordinary merge was not accepted: §14.10.3.4 must be read in its content-set context,
including the page-set identifier it requires. Several existing ledger notes
already distinguish shared attribute consumers from layout and opaque preservation
from semantic authorship; the audit records those corrections. This supports
retaining ADRs 1461/1535's method while keeping semantic review separate from the
quotation gate.

The bitshift causal experiment also completed: the unmodified isolated worktree
passed seven operator tests and failed the new negative right-shift assertion;
using logical zero-fill there passed all eight. The experiment was restored and
no production implementation was changed in the main checkout.


## Table inventories and ADR assertions

The Catalog/Page shard now inventories all 32 Table 29 and 33 Table 31 entries.
Forward references remain unreviewed obligations at their owning clauses; they do
not count as verified behavior. Compound records were narrowed where the cited
assertions covered only one part. A new split-writer test passes: it selects the
second of two pages and checks output Catalog/Pages/Page types, the indirect tree
link, immediate parent, one child, count, resources and exact selected MediaBox.
The Table 30 count requirement is general; ADR 2456's narrow archive repair does
not narrow that standard requirement. Its noninteger-count remedy also does not
prove correctness of every integer Count.

Six concrete decisions from ADR 2440 were reconsidered separately from the catalog
shard's generic geometry summary. The opt-in display-list approach and distinct
product/stated-style defaults remain justified, with clipping and compositing
limitations explicit. One new policy test passes for requested/unrequested
guidelines in ordinary, DeviceGray-group and separation contexts. The inspected
existing guideline, geometry, page-tree, shared-resource and tab-order suites ran
51 tests successfully; those results do not imply complete coverage of each row.

The annotation AF claim already had a real headless click test and an off-square
control; the draft audit had missed it. Three additional pointer tests pass,
checking the exact extracted payload, absent AF, and unrelated page AF. Their
fixture includes an explicit appearance and associated-file date. Two strengthened
example tests pass with deterministic embedded Type 3 glyphs: the MCAF association
covers only Hello between outside glyphs, and the corrected MathML MIME is asserted.
The old example file still contains the obsolete MIME and guarded range assertion;
it is not described as corrected by this audit. At 12:00 UTC usage remained 63%.


## Metadata, resources, and recovery-policy qualifications

The metadata inventory separates all nine Table 349 entries and the four date
reconciliation rules. A new public update regression fails: removing `/ModDate`
leaves Catalog `/PieceInfo` present, violating Table 349's conditional requirement.
The test first checks optional removal without PieceInfo and preservation during
an unrelated edit. Another test passes for actual update and merge output: both
Table 347 names, decoded UTF-8 bytes, and a non-ASCII title. The fixtures have
complete xref tables and PDF 2.0 identifiers where applicable.

ADR 1473 already explicitly expanded ADR 1212's three-property packet mapping;
the draft's claim that ADR 1473 retained that stale premise was rejected. The
version-dependent metadata policy remains justified. Its general ModDate-presence
obligation is separate from ADR 2748's archive-specific reconciliation policy.
Full ISO 16684-1 grammar and marked-content metadata placement remain unreviewed.

Three new resource-scope tests pass with actual painted colors: page/form/page
same-name lookup, inherited page resources, and byte-distinct names through an
indirect category dictionary. A fourth fails on a project-policy expectation:
a hidden missing ExtGState lookup is not reported although state set there can
persist into the visible fill after EMC. A defined-state control checks the
post-EMC alpha and proves the inner fill is hidden. This is malformed-input
reporting-policy consistency, **not an additional ISO conformance failure**.

ADRs 0343/0359's prefix recovery remains a defensible disclosed choice, but its
additive/subset rationale is not valid: later paint can cover earlier marks,
and corrupt compressed bytes need not decode to the original producer's prefix.
ADR 0183's undefined standard-font-name fallback is likewise a recovery heuristic;
standard font names do not reserve arbitrary resource labels. ADR 0115's existing
font-cache test catches leakage into a form lacking that font, but does not prove
two fully defined different fonts remain distinct. These qualifications do not
change production behavior or historical ADRs.

The next eight-crate suite ran 5,264 tests: 5,244 passed, 20 expected audit
regressions failed, and 52 were skipped. The failure set exactly matches the
recorded manifest, including the separately classified reporting-policy case.
Warning-denied all-target lint and package formatting checks pass for all eight
crates. At 12:14 UTC the weekly meter was 64%, twelve points above the start.


## Pattern repair: the guard mattered, but did not settle the claim

The first pattern-resource experiment did not publish a changed picture: final
archive validation refused it because the newly reachable marks introduced device
color without an output intent. The draft's unconditional reachability claim was
therefore too broad. Adding an independent one-point page mark makes color handling
part of the original conversion plan and removes that unrelated masking condition.
The explicitly resourced pattern control preserves its picture; the missing-resource
case then publishes 57,604 dark pixels where the source reader paints four, for
both PDF/A-2b and PDF/A-4. The two new regressions fail on that preservation check.

This is evidence to revisit ADR 2500's mechanical-preservation claim and its
pattern assignment, while retaining ADR 1059's no-pattern-fallback reader policy.
The source's absent required pattern Resources is malformed. The finding is about
the project's advertised preservation during repair, not an ISO requirement for
how every reader must render malformed input. ADR 2523's guard on copying forms
across differing page contexts does not guard this initial assignment. The targeted
expected-failure manifest now has 22 entries; the latest broad run predates these two.


## Font identity, print clipping, and assertion review

A new font-cache test passes with two fully defined Type 3 fonts sharing `/F1`
across page/form/page scopes. Exact red-square, blue-triangle, red-square output
checks both dictionary identity and restoration. The print test also passes through
actual viewer print commands and CPU rasterization: PrintArea controls the extent,
while PrintClip independently removes an outer mark and retains an inner mark.
Its sample positions are independently calculated pixel coordinates, rather than
values transformed using the production matrix being tested.

The viewer-preference inventory separates existing deliberate departures from
unimplemented host behavior. Ignoring `/Enforce` is already explicit in ADR 1180;
ignoring `/HideMenubar` is already explicit in ADR 1145. The implemented §12.2
ledger label conflicts with those retained qualifications. Qt's print bridge also
only forwards the first PrintPageRange pair, while GTK handles multiple pairs.
That host finding is source evidence, not a new executed failure. Native Windows
probe coverage exists in CI; this audit did not rerun Windows or certify every
viewer preference on every host. No ledger status or historical ADR was changed.

The new inline-image dictionary and public redaction tests pass. They check EC3
abbreviated-key precedence in both orders and ignored unlisted keys, then independently
measure emitted Length and decode redacted versus retained samples. Three more tests
pass for an inline stencil without ColorSpace, DefaultGray substitution, and LZW
abbreviation/extent/decoded samples. Draft failures caused by expecting white from
an explicitly transparent raster, and by a weak Gamma2 color contrast, were corrected
as test defects. The stronger Gamma4 control distinguishes default substitution.

A named-destination FitV test passes after replacing an invalid string OpenAction
with a GoTo action dictionary and choosing a viewport that makes FitV differ from
Fit/FitWidth. It also asserts page selection, not merely queryable page geometry.
A text-state test passes with embedded glyph metrics at two font sizes; its half-scale
comparisons use half-scale controls because the glyph advance also contracts.
A reference-location check now resolves 1,283 exact Rust function citations in accepted
feature records; file-level references remain explicitly weaker evidence. This check
is not semantic review and does not establish completeness.

An additional proposed inline-image defect is withheld pending review of token
boundaries. More importantly, parent review of the original PDF annotation coordinates
shows EC3 #20 amends the **normative** separator exception, not NOTE 2: the struck
words are “one of its filters” in the paragraph after Table 90. NOTE 2 already used
“final or only.” Both inline-image inventories are being corrected before acceptance.
At 12:35 UTC usage remained 65%, thirteen percentage points above the start.


## Destination regressions and a corrected errata interpretation

Four additional viewer regressions fail after their positive controls pass:

- FitBH uses the wrong vertical coordinate with a non-origin CropBox: expected
  origin y=-80, observed 0. FitH's independently calculated control passes.
- FitBV similarly treats an authored x coordinate as a display-list coordinate:
  expected x=-200, observed -400. FitV's control passes.
- A link to an equal-sized next page loses null XYZ coordinates that should retain
  the established position; zoom is retained but scroll resets to zero.
- On the same page, changing zoom while retaining null coordinates keeps/clamps the
  old pixel scroll rather than retaining the default-user-space position.

In an isolated checkout, removing FitBH/FitBV from the “already page space” branch
changes both targeted failures into passes; the source was restored afterward.
That supports the coordinate diagnosis, not certification of every rotation/UserUnit
case. ADR 0162's model/viewer boundary remains sound. Its completion claim needs
qualification; the mistaken coordinate classification is in current code, not an
explicit normative choice in the ADR. The expected-failure manifest now has 26 tests.

The eight-crate run preceding these four regressions ran 5,275 tests: 5,253 passed,
22 expected failures, 52 skipped, no unexpected failures. Formatting and warning-denied
Rust lint passed. Later targeted tests and their lint are recorded separately.

The invalid `ID4142>` draft was discarded: it is one regular-character token, not
an ID operator followed by data. A valid ASCII85 case starts data with the delimiter
`%` immediately after ID; it passes both with and without separator whitespace and
checks exact decoded bytes and resumption after EI. This tests the actual normative
exception as amended by EC3 #20. Two more public redaction tests pass for direct
Indexed palette preservation and an inline stencil retaining its unredacted bits.

Four new embedded-font tests pass for TJ sign/thousandths, text-line versus text
matrix positioning, replacement/rotation by Tm, and the double-quote operator's
spacing and line move. A separate two-show test verifies modes 5/6 paint their glyphs
before the accumulated clip takes effect at ET. The original draft's T* x=40 and
Type3 matrix scale=10 were corrected to x=30 and scale=.01 using Tlm and FontMatrix;
those were faulty test expectations, not product failures.

A valid nine-page fixture now passes through page-label queries to host PageEntry
labels, with an unlabeled-page fallback control. Existing arithmetic fixtures that
query nine indexes in a one-page document remain narrower evidence. ADR 0601's
choice under contradictory accepted alphabetic-label errata remains defensible
and explicitly ambiguous; the two PDF annotation groups were checked again.

### Empty page labels and nested outline consumers

The label model returns the empty string required by Table 161's no-S/no-prefix case, but
`viewer-core/src/viewer/answers.rs::label` filters that value into absence. The host then substitutes
its unlabelled fallback (for example, “Page 5”). A valid nine-page fixture proves the model result
and the adjacent `A-8` range before failing at the viewer query. The target has one pass and one
failure (`page-labels-empty-reviewed-target.log`); the manifest now contains 27 individual failing
tests, not 27 distinct defects. The explicit-empty-prefix branch follows the same code but is not
reached after the first failing assertion. ADR 2671 deliberately emits empty-label ranges for
appended pages, making the distinction relevant to this project's own output as well.

A new `viewer-ui` test follows a valid PDF's nested positive and negative outline Counts into the
pure sidebar. It verifies linked order through actual activation targets, initially hidden
descendants, and opening/closing. It passes (`outline-count-panel-target.log`). GTK's separate
4096-index initial expansion cap remains a source-supported qualification: some Count-positive
rows start collapsed after the threshold, but their children remain manually accessible. This is
not a claim of lost descendants. ADR 0244's small-document evidence cannot establish behavior at
the boundary; ADR 0246's Qt flattening and ADR 1906's mechanical GTK refactor do not ratify that
cap. These are selected decision-unit reviews, not reviews of every decision in those documents.

### Image byte order, text-state controls, and the next regression gate

The old test `sixteen_bit_samples_are_two_bytes_most_significant_first` uses only
`0000/5555/AAAA/FFFF`. Those values survive byte reversal, so the test name overstates its
assertion. The new image audit uses asymmetric `0123/4567/89AB/CDEF` samples and independently
expected gray levels `1/69/137/205`. In the isolated mutation checkout, both original and new tests
pass normally; reversing the two-byte read leaves the original test passing and fails the new
one (`image-byte-order-results.json`). The isolated source was restored and production was never
edited. A second new test checks four exact RGB pixels from a two-row, two-bit-per-component image,
with distinct Decode pairs and nonzero row padding. Both pass.

Three new text-state tests pass: implicit page defaults after interpreting a changed first page
from the same Document; vertical Tc/Tw/Tz/Ts with an embedded composite font; and text clipping in
hidden optional content. The last includes a visible-versus-hidden normal-paint control and a
full-rectangle no-clip control, so equality of two rasters alone cannot establish success. Default
rendering mode also asserts absence of stroke/clip commands. These close selected assertion gaps,
not every text-state combination. EC3 #366 was located on PDF page274 using annotation geometry
and text bounding boxes: the LZW/Flate sample-size condition loses its predictor-only qualifier.

ADR0025's explicit image-reduction departure remains a reasonable documented product choice,
but its claim that an informative NOTE licenses deviation from a specific SHALL needs
qualification. Keeping the behavior and establishing standards authorization are different
conclusions. No historical ADR was edited. The image dictionary's general rule also explicitly
requires an error for inconsistent entries/colour space; a prior draft's blanket dismissal of
malformed Decode pair counts was rejected and is under focused followup.

The nine-crate gate completed **5,611 tests: 5,584 passed and 27 expected audit failures**, with
no unexpected failures; see `nextest-nine-crates-wave7.log`. The pure `viewer-ui` outline test
passed separately. This run predates the next Link highlight test, which also passes on its own.
Clippy passed for `pdf-model`, `viewer-host`, and `viewer-ui` before the latest16bit/link additions;
the next lint gate must include them. The ADR index distinguishes selected decision units from
whole-document completion and leaves every ADR's whole-document completion false.

### Image error reporting and optional-content initialization

A malformed input is not automatically outside reader obligations: §8.9.5.1 explicitly requires
an error when an image's dictionary entries disagree with its colour space. DeviceRGB Decode
arrays with too few or too many component pairs currently produce no report. Both new regressions
first paint a valid three-pair control red, then fail on the required error
(`image-dictionary-consistency-target.log`). They are two witnesses of the missing shape check,
not a claim that all malformed PDFs must be refused.

Three valid optional-content initialization tests pass: the default configuration wins over an
opposite alternate; explicit default View intent ignores a Design-only group; and initial View
usage can turn an OFF group ON without affecting an absent usage category or a non-View event.
Some existing fixtures put BaseState OFF or Intent All in D, contrary to Table99's producer rules;
those remain useful robustness tests but do not prove valid-default conformance. ADR0017's original
claim that its deferred interactive entries did not affect initial state was too broad for AS/View;
later implementation addresses it. ADR1106's host-input architecture is retained, while its claim
that missing audience data lies outside the normative comparison remains an interpretation requiring
separate review. Full usage/lifecycle behavior and alternate selection are still residual work.

## Link, action and Form XObject checkpoint

Table 176 is on physical PDF page 498 (printed page 483). EC3 #17 permits the visual appearance
and active QuadPoints region to differ; it does not change the H entry's pressed-inside-active-area
condition. The new complete-PDF viewer test proves LinkAt returns false outside valid QuadPoints,
but pressing there still turns the red appearance cyan. Inside-quad activation/highlight controls
pass. An independent fixture verifies that any escaping QuadPoints coordinate restores the Rect
active area; its inside/outside query and pixel controls pass. These fixtures were split during
parent review because the original two overlapping annotations would invalidate the background
control. ADR 0177's general annotation routing remains useful, but it needs subtype active geometry.

A second Link test reproduces ADR 0123's implicit-P policy when H is absent and AP/D exists.
Explicit H/I over a blue down appearance renders yellow; the otherwise identical absent-H case
renders differently. Table 176 specifies default I without that exception. The widget-only
Table 191 override sentence invoked by the ADR does not supply a Link exception. The finding is
therefore a deliberate shared-policy departure from the Link default, with the inference about
appearance composition stated explicitly. Link O border inversion and explicit P down-artwork
selection pass; P's exact visual artwork remains producer/project choice. Main Link BS evidence
was downgraded: the cited existing border tests use Border, not BS, and cannot prove BS dispatch.

Table 196 requires Next-array actions to be performed in order. A complete valid-PDF reader test
gets FirstPage, LastPage from a root followed by the non-cyclic array [same-reference,
same-reference], where the ordered sequence contains two LastPage occurrences. The distinct-child
positive control passes. This occurrence requirement is inferred from ordered-array semantics;
the specification does not contain a separate sentence about duplicate references. Its informative
self-reference advice does not describe this acyclic case. Current global visited-object tracking
drops the second occurrence. Full viewer execution and generic catalog OpenAction behavior remain
separate residuals.

The six Link/action tests finish **3 passed, 3 failed** in
`link-actions-reviewed-target.log`; the three failures are recorded individually. Current
pdf-model/viewer-core all-target Clippy passes. A second test-quality issue emerged in ADR 1175's
archive evidence: the existing test checks that NextPage bytes survive, but does not prove they
remain reachable from OpenAction or that multiple survivors keep their order. A focused semantic
writer test is being drafted. The preservation decision remains reasonable; its proof was weaker
than the draft review claimed.

Eight Form XObject tests pass in `form-xobjects-expanded-target.log`: caller color inheritance,
color restoration after invocation, repeated translated use, direct Matrix placement, noncommuting
Matrix/caller-CTM composition, reference page-label selection, numeric page selection, and proxy
fallback without a supplied target. The referenced file has distinct green page 0 and blue page 1;
only page 1 has the requested label, while the proxy is red. Complete xrefs, resources and full
file identifiers prevent fallback repair or an always-first-page lookup from satisfying the tests.

Form inventory remains partial. EC3 #292 on physical page 287 makes independence mandatory from
PDF 2.0 and Resources sometimes required; #463 on page 288 adds the structural-role exclusivity.
These conditional producer requirements are not certified by permissive reader fixtures. Metadata,
PieceInfo, LastModified, AF, Measure, PtData and structural-role conditions now have separate
records instead of compound entries. Generic print-resolution evidence was removed from the
reference-XObject print permission, and positive imported-annotation tests do not certify all
hidden/nonprinting exclusion branches.

Selected ADR corrections matter here. The draft had reversed ADR 0113: it **adopts**, as an explicit
extension, a zero-origin Rect-sized BBox for a malformed stored appearance. The existing test
checks both its report and exact recovered extent; this is recovery policy, not a normative
stored-form default. ADR 0115 chiefly concerns font-cache dictionary identity and is cross-referenced
to the earlier dedicated audit. ADR 0057's actual tiling test distinguishes form-space and
page-space anchoring. ADR 1101's explicit host supply remains sound, while its mandatory-ID
matching is a trust policy: Table 95 makes ID optional, and matching an already approved resource
name need not grant arbitrary filesystem access. This audit records that qualification without
changing the policy. No whole ADR is marked fully reviewed.


The ADR 1175 follow-up is now executed: two archive tests pass. One follows the converted catalog's
OpenAction and asserts four surviving actions in exact recursive order, with the original Launch
and ordered subtrees checked first. The other exercises two holders that would require different
successors for one shared action and asserts the named conversion refusal. These close the
specific observation gaps described above; the latter remains a project preservation policy.
The reviewed draft originally expected a file-naming Launch to be Refused, which parent review
corrected to the actual Launch variant before execution.

Alternate selection also gained a passing Print fallback test and a stronger order test. The old
"first visible" fixture has only one visible alternate, so selecting the last visible alternate
would satisfy it too. The new fixture has two visible alternatives with distinct samples and
reverses their array order under View, Print and Export. This tests the ordering rule itself.
A separate proposed Print/image-level OC failure stays in scratch pending interpretation: EC3#79
removes the old Print recheck sentence and makes the designated image mandatory, while its explicit
image-level OC exemption appears in step d). ADR0092 had read the deleted sentence as referring
to the image XObject, which supports the current Print behavior. This is not counted as a
confirmed defect or promoted as a normative failing test.

The isolated last-visible mutation confirms that distinction: the old alternate-selection test
passes on both baseline and mutant, while the new test passes baseline and fails mutant.
`alternate-order-mutation-results.json` records all four runs; the isolated source is restored.

Three additional action-trigger tests finish **2 passed, 1 failed**. Catalog OpenAction handles a
GoTo destination and JavaScript at first presentation, but a supported Named LastPage action does
not execute there. The same Named action works through a real Link pointer gesture in the test,
so the failure isolates the catalog consumer rather than action support or presentation timing.
The other tests protect Hidden|NoView annotation lifecycle events and outer-X-before-inner-E
ordering for nested annotations. Informative Note 1 explains the flag distinction; it is not
counted as a separate normative SHALL. Table 200 was checked in the source PDF, physical pages
524–525 (printed 509–510), because its generated Markdown cells are transposed.

Reconsidering ADR 0164 exposed a changed premise: the old single-page argument cannot justify
co-raising visibility and open/close events in today's continuous and two-page layouts. ADRs 1750
and 1752 explicitly acknowledge the neighbor-visibility restriction, so this is a documented
limitation rather than an unnoticed regression. Two new public-viewer tests fail independently:
changing to a two-page spread omits the neighbor's PV, and changing the current page within that
spread raises PI for the prior page even though its geometry remains displayed. Both fixtures
first verify a working initial PV, exact xrefs, page resources, current-page identity, and placement.
The selected visibility decisions merit revision; runner ownership and recursion policy are
separate decisions. These tests use URI actions, so script policy cannot explain the omission.

The individual expected-failure inventory is now **35**. This includes three new action-trigger
failures after the last broad run; it is a count of failing assertions, not distinct root causes.

Image-coordinate coverage now includes three passing complete-PDF CPU tests: a non-square source
mapped onto a square, horizontal reflection, and a CTM with both off-axis components. Six distinct
RGB samples and outside-footprint controls distinguish row/column order, source-aspect distortion,
and placement. The published matrix is `[1/w 0 0 -1/h 0 1]` on physical pages 271–272 (printed
256–257); generated Markdown line 6321 drops the minus sign. There are no non-Link EC3 annotations
on those pages. Codec-native orientation and exhaustive GPU affine cases remain outside this pass.

Five default-color tests pass in `default-colour-expanded-target.log`: Form-local resources,
Indexed base, uncolored Pattern base, selected DeviceN alternate, and DeviceCMYK initial values
under a DeviceN default. Each compares distinct expected colors or an absent-default control.
The last also selects the same default as its own family to show that its initial vector gives a
different result. Parent review corrected missing CalRGB array delimiters in two draft fixtures
before execution. Default-family and component-count producer constraints remain inventoried
separately; permissive malformed-input reading is not evidence of a violation without a reader rule.

The default-color ADR review required a substantive correction. ADR 0069 originally excluded only
top-level named spaces from shading caching; arrays, indirect references and nested device spaces
could still reuse colors under the wrong resources. ADR 1765 supersedes that mechanism with the
ColorSpace resource entry in both cache keys, and named spaces are now cached too. Its existing
three-spelling test asserts page-red versus Form-green for the same shading object. The selected
resource-dependency principle is retained; the original mechanism is superseded, rather than
reported as unchanged. ADR 1001's shared lookup remains useful, but today's ranking has further
group/intent cases, so the historical three-source description is not certified as current.


The mixed-chain ordering regression is confirmed in `interpolation-mixed-action-target.log`.
A link's SetOCGState action followed by JavaScript invokes the runner while the layer is still
OFF; final viewer state is ON. The reversed JavaScript-then-SetOCGState chain sees OFF and ends
ON as expected. This observes state through the public runner snapshot, not through independent
logs with incomparable timing. ADR 1602's known interleaving limitation merits revision under the
supported runner scope; ADRs 1750/1752/1904's event and confinement boundaries can be retained
separately. This makes **36 individual expected failing tests**.

Ordinary-image interpolation now has a passing parser-to-pixel policy test under normal and
reflected magnification. Absent and false preserve sharp samples; true mixes seam pixels while
retaining source interiors. ISO permits ignoring this hint, so the assertion protects this
application's choice. An isolated mutation discarding the parsed flag leaves the old constructed-
Image policy test passing but fails this new complete-PDF test; both pass baseline and the isolated
source is restored. The old test remains useful for its own layer, but cannot certify parsing.

The interpolation ADR review also corrected the archive summary. ADR 2498's XObject loss/history
prerequisite remains supported, but its blanket inline-image refusal is no longer current:
ADR 2516 permits eligible whole-page photographs under `as-if-printed`; `as-shown` still refuses.
The existing tests distinguish the two outcomes. Adjacent Table 87 cells do carry EC3 marks on
physical pages 274–275, although none changes the Interpolate cell or its paragraph on page 278;
the draft's blanket assertion of no marks in that printed-page range was discarded.

## Tiling patterns and selected pattern decisions

Five new complete-PDF tests pass (`tiling-patterns-five-target.log`), and warning-denied
Clippy passes for all pdf-model targets. They cover cell CTM/stroke-color/width restoration,
image-mask painting inside an uncoloured cell, signed step combinations, pattern placement
when page CTM changes before or after selection, and genuine gaps in patterned glyph strokes.
The old glyph-stroke test required red ink and no green fallback but could accept a solid-red
stroke; the added test also forbids ink at known cell-gap coordinates. The old split-path test
did not change the page CTM, so it could not alone prove independence from those changes.

Selected decisions in ADRs 0057, 0058, 0735 and 2212 still hold with the recorded qualifications.
ADR 0058's historical unsupported-stroke limitation was superseded by ADR 0735. Backend stroke
expansion into an alpha mask remains supported by the inspected path and strengthened text
assertions. The source-retention review does not certify every colour-conversion context or
re-evaluate the separate non-RGB solid-colour issue recorded in ADR 2213.

Parent source review rejected the draft's attribution of a Pattern tint amendment to EC3 #128:
that issue concerns Type 3 resource lookup. The actual nearby pattern amendments are #428
(physical page 235, printed 220), qualifying tile order as implementation dependent, and #294
(physical 236, printed 221), clarifying the Table 74 stream-dictionary caption. The annotation
index uses one-based physical page numbers. Producer constraints remain inventory entries even
when the clause specifies no reader refusal. Zero-extent BBox behavior and sampled colour images
inside coloured cells remain explicit test residuals; no entire pattern clause is certified.


## Image-mask errata and document-event timing

The latest broad wave now runs 5,663 tests: 5,625 pass, all 38
failures match the expected-failure manifest, and 53 are skipped (`nextest-nine-crates-wave10.log`).
Warning-denied Clippy passes for pdf-model and viewer-core after these changes. The new image-mask
target has four passing tests and two failing regressions:

* EC3 #215 adds a reader requirement to ignore `Mask` and `ColorSpace` on an image whose
  `ImageMask` flag is true. A stream-valued `Mask` is instead applied and suppresses a red stencil
  mark, revealing the green background. Colour-space and array-mask controls pass. The test
  permits diagnostics and fails on changed pixels; it does not mistake a warning for a defect.
* A base stencil invoked by `Do` with `BitsPerComponent 2` is silently interpreted as one bit.
  Table 87 requires one when stated, and §8.9.5.1 requires an error for inconsistent image
  entries. EC3's ignore rule does not cover this bit-depth entry. The one-bit control paints
  correctly; the inconsistent case reports nothing. This is adjacent to the earlier Decode
  pair-count findings, not an invented blanket malformed-file rejection policy.

Passing controls prove pre-Decode colour-key comparison, coarser-mask alignment across both base
rows, `SMask` precedence over a colour-key mask, and preservation of previously painted green
background under all three masking routes. The latter also proves optional stencil bit depth can
be omitted. An existing render-raster stencil-edge test passes under a Vulkan/Radeon ICD selection,
checking non-vacuous partial coverage and preserved red colour; its log does not print adapter
identity, so it is not evidence of a particular hardware model.

The ADR review preserves the separate mask mechanisms and codestream-domain decisions, while
correcting two historical attributions. ADR 0451 §2 separates the grid preference from the
allocation ceiling; the current source comment's reference to ADR 0456 is wrong. ADR 1121 still
refused non-eight-bit non-Indexed JPX samples; ADR 1242 supplied the wider sample-preserving
pipeline used by the current 12/16-bit tests. Neither historical limit describes today's full
implementation. Raw codec validity and every grid ratio remain outside this selected review.

Two public Viewer timing tests pass. A `/WS` field edit is present in reopened saved bytes, while
an ensuing `/DS` edit is live-only. `/WC` produces its observable report before `Closed`. A `/WP`
edit reaches the actual page handed to the printer (rasterized against a blank control), `/DP`
runs on active Finish, and an idle or duplicate Finish produces no additional event. All runners
return `rc=false`, testing the project's non-veto policy separately from ISO's event timing.
ADR 1614's boundary still holds with the ADR 2763 qualification: XFA preSave can separately hold
or refuse a save after `/WS`, and XFA postSave cleanup is distinct from catalog `/DS`. Combined
cancellation/refusal tests and full host-by-host event timing remain in the continuation work.

## Presentation defaults and held-operation timing

Four new presentation tests pass. Random access to a page with two navigation nodes runs the
primary forward action and makes the next node current; a backward request then runs that
second node's backward action without leaving the page. A separate node-duration test checks
the two-second boundary with no page duration. Transition metadata reaches the host with the
selected Table 164 defaults and explicit controls; omitted style and explicit `R` both replace
immediately even with `D 99`. These metadata assertions supplement existing visual tests;
they do not certify every style or every native host's animation.

The source review corrects the draft's blanket Table 164 confirmation into separate, bounded
entry records. EC3 #36 makes `Di` an integer; parser tolerance remains a recovery policy.
Effective #304 is binding despite its `Text` annotation subtype: linked Review/Completed
annotations add the end-of-list rule. Ordinary exhaustion has tests. Its interaction with
arrival at a page containing only one node remains an interpretation question, not a reported
defect. Action-triggered redirects, backward arrival, and the complete style applicability
matrix remain open. Notes and examples are retained as guidance, without inventing new shalls.

The selected ADR decisions retain injected elapsed time and pure frame geometry. ADR 0316
reverses ADR 0135's original absence of explicit presentation state and its tick-only transition
rule; those historical premises must not be treated as current. ADR 1299's selected quantities
remain explicit product choices. ADR 1853's final-frame assertion checks incoming raster bytes;
its entire current host adoption path was not re-audited.

Two additional operation tests close the combined cancellation/refusal gap described above.
One ordered runner log distinguishes catalog `WS`/`DS` from XFA preSave/postSave. Successful
saves run both catalog events; refused or declined saves omit `DS` but still perform XFA
cleanup. An unanswered request runs neither `DS` nor cleanup yet. A complete encrypted print
fixture with permission bit 3 clear verifies that `WP`/`DP` occur only after the configured
policy admits the job. The test checks decrypted script bytes as well as callback order.
`DS` follows the successful write internally; it precedes emission of the public `Saved` event,
so no assertion claims the opposite. These six new tests pass independently of the earlier
wave10 totals. Full host-by-host operation timing remains open.

## Font-width premises and annotation-border defaults

The new width target passes four complete-PDF consumer tests, and the separate pdf-font unit
target passes all 370 tests. The embedded TrueType fixture checks actual glyph origins from
the declared width array; it observes the first three advances through four glyphs. The
standard-font cases characterize deterministic fallback and partial dictionaries under the
effective PDF 2.0 reading. They do not establish ISO 32000-1 producer conformance.

EC3 #47/#48 strikes Table 109's all-or-none/override paragraph and replaces the compatibility
mandate with a cross-reference. The availability paragraph in §9.6.2.2 is deleted and the
preceding paragraph becomes informative. ADRs 0133, 0358 and 2687 repeat retired mandatory
premises. Their useful implementation choices can stand, but those quotations no longer supply
the claimed requirement. ADR 2686's archival profile context is separate and has not been
invalidated by this reading. EC3 #453 also makes mismatched actual glyph widths and MissingWidth
implementation-dependent. The Courier 600-versus-400 test was therefore renamed and classified
as policy evidence. The corpus width test has a non-vacuity bound but tolerates some mismatches;
it is a mapping regression screen, not a comprehensive producer validator.

ADR 0358's current implementation condenses substitute outlines by a bounded median. Its
assertion meaningfully checks width and unchanged height, but an advance does not generally
bound a glyph's ink: overhang is possible. That rationale needs qualification independently
of the retired standard-font quotations. No substitution behavior is changed by this audit.

Four border tests pass. They isolate omitted BS width/style, default dash array and phase,
default no-effect BE, and Table 168's explicit requirement to use solid for an unknown style.
The unknown-style sentence was absent from the draft inventory. Default-versus-explicit dash
equality alone could pass with two equally wrong phases, so the test also compares independently
authored page strokes at phases zero and one. Every fixture checks that parsing required no
cross-reference recovery. A later residual pass below supplies selected cloud-specific BS width/dash assertions.

The border ADR review retains the inside placement, precedence, cloud and relief decisions with
their chosen geometry distinguished from ISO requirements. ADR 0674's two thresholds are not
identical: an inward border band covers the whole rectangle at half its smaller dimension,
whereas the inset stroke path degenerates at the full dimension. The existing tests cover both
and pass; the inaccurate explanation is not evidence of a current rendering defect.

Broad wave11 includes these additions: 5,677 run, 5,639 pass, exactly the same 38 manifested
failures, 53 skipped and no unexpected failures. The pdf-font 370-test run is separate from
that total. Warning-denied Clippy passes for the changed model/core test targets.


## Appearance selection and cloud-specific border controls

Table 170 requires the normal appearance for printing. New complete-PDF regressions reproduce
one defect at two layers: Print purpose with a retained pressed pointer selects the blue down
appearance instead of red normal. The public Viewer test first observes red on screen and paper,
then performs a real pointer press and observes blue on screen before querying the print page.
The print raster is blue. The model target has one passing rollover/default test and one expected
failure; the public Viewer target has one expected failure. The failure manifest now contains
40 assertions; wave11 above predates these additions.

Two cloudy-polygon tests pass. They vary dash arrays and line widths while preserving the
selected cloud path, require visible ink, check thicker coverage, and require zero width to
suppress the stroke. These verify Table 169's separate width/dash duty; the scallop radius is
still an implementation choice. Warning-denied Clippy passes for these model/core additions.

The appearance ADR review corrects stale evidence. ADR 0579's report-only group limitation was
superseded by commit 72ca5e99: stored appearances now enter transparency-group rendering, and an
existing pixel test distinguishes isolation and knockout. The CA/ca text conflict remains.
The actual opacity regression uses CA0.5 and an opaque stream and expects alpha255; it does not
assert the historical 0.8-versus-0.64 example. Name-tree tests prove lookup and shared object
identity, not rejection of an AP-name input. R/D state dictionaries and absent-Group equivalence
remain targeted follow-ups.

Physical page 485 places the EC3 #34 no-AP sentence in the amended NOTE2 before the heading
12.5.3. The existing errata narrative and ledger's 12.5.3 attribution are misplaced; it belongs
to 12.5.2. This corrects the citation, without changing the implementation-dependent meaning.


## Descriptor, flag and assertion follow-up

Three annotation-flag tests pass through public model consumers: Invisible on an unknown subtype
versus a standard Square, hit testing for ReadOnly/Hidden/NoView/ToggleNoView with the Widget
exception, and containment of an unselected appearance stream. The old undefined-AS no-mark
assertion could pass because the selected state was absent, independently of Print gating. The
new containment test has explicit rendered controls; its malformed-N construction fallback is
labeled recovery policy rather than valid-file conformance.

A separate public Viewer regression fails: with identical print scale, an 80-unit NoZoom
annotation prints 160×160 pixels after view zoom 1 and 80×80 after view zoom 2. Ordinary-annotation
controls print 160×160 in both cases. The test filters nonempty red pixels on a transparent medium,
so paper background cannot satisfy the size assertion. This is screen magnification leaking into
print geometry. NoRotate's intended paper behavior remains unresolved. ADRs 0168, 1179 and 1180
retain their screen-placement and output-purpose architecture with this interaction qualified.
LockedContents default-Off enforcement remains an explicit product policy, with its public edit
path delegated for further testing.

The selected common descriptor and CID descriptor reviews separate writer constraints from
reader behavior. Missing required descriptor entries, reserved bits, and font-file exclusivity
are not certified by reader tests. Font substitution ranking, positive-Descent recovery and
numeric FontWeight tolerance are choices. ForceBold remains unresolved: the specification requires
the flag to determine an extra-pixel decision but permits thickening without prescribing a raster.
Current substitute weight selection does not prove that low-resolution decision.

Two new font unit tests pass without system-font dependence. One gives Symbolic and Nonsymbolic
routes different glyph indices and checks that conflicting flags follow Symbolic. The previous
cited test never set both bits. The other tests a distinctive class for each of Table 123's six
collection rows, with nonmember and wrong-collection controls. All 372 pdf-font unit tests pass;
some older installed-font consumers can return early and are not thereby certified. EC3 #5
repairs the old FD/Table120 contradiction. ADR 0086's FD deferral is superseded, and ADR 1074's
claim that StemV and ascent/descent are unused is stale after later consumers. The class-selection
policy remains bounded; external collection/Supplement semantics and PDF/A output checks are open.

Peer review corrected the presentation timing claim: a terminal node action does not prove
successor selection, and Dur's maximum-time wording does not require waiting until the exact
deadline. A new middle-node test observes a distinct successor action after the timer; Fly motion
default/override and action-redirection controls also pass. The presentation target now has seven
passing tests. The ambiguous #304 terminal-arrival composition remains open. Three additional
appearance tests pass for absent-Group/default isolation, knockout overlap and R/D state selection;
the retained print-selection regression still fails.


Broad wave12 runs 5,692 tests: 5,651 pass, exactly 41 manifested failures, 53 skipped and no
unexpected failures. The separate pdf-font suite passes 372 tests. Warning-denied Clippy passes
for pdf-font, pdf-model and viewer-core after the added tests.

An isolated mutation check then exposed a narrower weakness in the new absent-Group test:
changing its default knockout flag to true still passed the single-element isolation control.
An added overlap comparison against explicit non-isolated/non-knockout now fails that mutation;
both unmutated controls pass. The production source in the isolated checkout was restored, and
live production code was never mutated. This strengthens an assertion, without alleging a real
knockout rendering defect. Wave12 predates this final assertion; its targeted run is recorded
separately.


## Annotation contents policy at the public boundary

The new LockedContents test passes through public Viewer commands and reads effective text back
from the note popup. Default Off changes the text; On refuses; Ask leaves it unchanged while
unanswered and after no, and commits after yes. Warn commits and emits Dirty before Warned.
Locked bit8 alone creates no contents-edit question. The fixture uses PDF1.7, where this Text
annotation may omit AP, and verifies exact cross-reference parsing.

These are product-policy assertions around Table167's restriction, not a claim that ISO defaults
to Off. The selected ADR0304/0814/1144 decisions remain supported. ADR0304 originally concerned
FreeText; this Text/SetNoteText test exercises their shared restriction boundary and does not
recertify appearance regeneration or saved output. The test is additional to wave12 totals.


## Common annotation dictionary evidence

A complete PDF1.7 regression now proves that the same NM string on two different pages resolves
to two distinct annotation objects, with an absent-name control. The target passes. This supports
page-local lookup; it does not certify producer uniqueness within a page or the separate rule that
one annotation dictionary belongs to only one page's Annots array.

The Table166 review keeps indirect P references, FDF exclusion, Border array structure and
producer constraints visible as inventory duties. Existing malformed-date assertions prove that
the popup model retains the exact string; they do not replay every native host. The cited date-save
tests can return early when corpus files are absent. Annotation language assertions do discriminate
annotation, structure and document values at the public accessibility boundary.

Parent review rejected a claimed combined OC/flag test: the cited configuration-audit target has
no annotation fixture. An older annotation test varies only group visibility with F4 fixed.
Combined gating is a focused follow-up. A draft's unrelated ADR0444 attribution was removed;
selected ADR0209 and ADR1160 decisions were checked against their actual text.


## Embedded font programs: corrected scope and a consumer control

The effective §9.9 inventory now includes four previously omitted OpenType glyph-selection duties
and the separate writer recommendation for Type0/Identity-H. Table124 format pairings, required
producer keys and external font-format conformance remain separate from tolerant reader behavior.
The Length3=0 assertions do check unchanged program bytes, 512 added zeros and the terminator;
other required-key and CFF single-program constraints are not certified by parsing one program.

Parent and peer review rejected a proposed failing cmap test. The shared helper explicitly answers
Table124, while the simple/CID cmap rules appear elsewhere in §9.9.1. Demanding an empty helper result
would change that contract. Simple-font archive checks exist separately, and the composite writer
strips cmap from FontFile2 output. A possible CID validation gap is being investigated with a
complete-PDF test; it is not in the failure manifest.

One new display-consumer test passes with no system-font dependence. A repository font receives
embedded vertical advances of 5 versus 7 em; the diagnostic API confirms they are loaded and the
three glyphs have outlines. Their placed quads remain identical. Changing W2 moves the second
glyph, while changing only DW2 advance leaves the second unchanged and moves the third. Parent
review corrected the font’s units-per-em, the vhea extrema and a confounded control. The vhea
metadata follows the [OpenType definition](https://learn.microsoft.com/en-us/typography/opentype/spec/vhea).
This establishes display placement, while archive inspection of those metrics remains a distinct
interpretation question. Warning-denied Clippy passes; this test is additional to wave12.

ADR0049 was also corrected in the audit: its decision concerns CID indexing into a malformed bare
Type1 embedding, not the adjacent CFF signature-dispatch test. The actual corpus regression compares
glyph shape, which distinguishes the wrong substitute; it can return early without its corpus file.
Historical archive conversions and external font-format conformance were not freshly certified.


## Watermarks and combined annotation visibility

Three new raster tests pass. The watermark test compares omitted Matrix/H/V against explicit
identity/zero defaults and an independently displaced positive control. The annotation test varies
layer state, Hidden, Print and NoView across screen and print; every positive requires the full
expected extent, and every negative requires no visible pixels. A second annotation test checks
OCMD AllOff with the opposite layer-state outcome. All fixtures have exact cross-reference tables,
no recovery, page/form resources and nonempty stored appearances. Clippy passes.

The watermark review supports media-box display, known/unknown print media, transformed rectangle
placement and page-scale cancellation at the inspected consumers. Tiling remains conditional on an
operation no current host selects. Model scale cancellation and host cell geometry are tested
separately; a composed n-up consumer test is the next step. No physical printer output is certified,
and the ADR0934 page-Rotate interpretation remains provisional. FixedPrint does not create a second
report of the existing NoZoom print defect.


## Reconsidering the half-pixel ADR premise

ADR0444’s four-corner maximum is sound: the norm of an affine displacement is convex, so its value
inside a rectangle cannot exceed the maximum at a corner. Its next inference is false. A point at
0.49 can move to0.51 across a nearest-sample cell boundary with a displacement of only0.02. A bound
below half a pixel therefore does not guarantee that every feature stays in the same pixel cell.
The measured scroll/zoom separation supports a chosen approximation threshold, not that theorem.

Current code also accepts equality at0.5, while some prose says strictly below. More significantly,
it measures inter-page disagreement before snapping the selected translation independently on each
axis. That snap can add up to0.5 per axis, so the reported `within` is not final placement error.
The temporary-frame policy can remain useful with these limits stated accurately. ADR0442’s former
exact composition comparison is superseded; exact `depicts` still answers a different question.
ADR0657’s interruption policy inherits the approximation, without becoming a pixel-identity guarantee.
No production code or historical ADR was rewritten, and no failing conformance test was invented
for a zero-error contract the product does not have.

The complete-PDF CID cmap probe confirms a documented validator boundary. With and without cmap,
the narrow filing judgement is Met; the full report explicitly leaves font-format validation
Unchecked and reports other failures. ADR2307 records that limited scope. The probe therefore does
not establish false overall conformance, and its proposed failing assertion was not promoted.
A separately named cmap check remains a possible coverage extension. Parent repaired the draft’s
descriptor brackets, Length1 location and empty-glyph choice before running the observation.


Broad wave13 runs 5,698 tests: 5,657 pass, exactly the 41 manifested failures, 53 skipped and no
unexpected failures. The parent compared the exact binary/test set, not only the count. The six
new tests since wave12 are included. The vertical fixture now uses Symbolic for its full embedded
font, and a subsequent targeted run also requires every outline path to be nonempty; that stronger
precondition passes. The failed assertion count remains distinct from the number of underlying bugs.

A new PaintType regression passes six cases across embedded Type1, Type1C and CIDFontType0C.
Each program explicitly requests stroke painting, while PDF Tr0 still emits Fill and Tr1 emits
Stroke. The test requires a loaded half-em program advance and a nonempty glyph path before
checking display commands. Parent repaired the draft Type1 dictionary construction, and FontTools
independently parsed all three fixtures with PaintType2, width500 and the same closed rectangle.
This checks the specific PDF override, without claiming complete font-format conformance. The
single test is additional to wave13; warning-denied Clippy passes.

An isolated mutation calibrates the new vertical-metrics assertion: replacing PDF vertical advance
with embedded vmtx advance makes the cross-program quad comparison fail, while the unmodified
control passes. The isolated production file was restored and compared byte-for-byte with live
source. This demonstrates sensitivity to the prohibited dependency, not a defect in current code.


The clause 6 review inventories seventeen aggregate, definitional and optional-permission records across its eight ledger rows. The ledger integrity test checks population, citations and accounting; it explicitly cannot prove implementation. qpdf acceptance and foreign-reader readback are useful bounded checks, and both cited consumers can return successfully without their external prerequisites. Existing accepted writer and annotation-printing counterexamples carry through to the aggregate claims; they are not counted as additional defects. Headings and definitions do not require invented behavior tests.

The selected decisions in ADRs 0984, 1461, 1535 and 1622 remain sound with those qualifications: include clause 6, recheck writer applicability, distinguish grounded quotations from correct interpretation, and distinguish mandatory requirements from recommendations. ADR 1225's local-disk decision to decline linearized access optimization remains reasonable on its historical measurements; those measurements were not repeated and would need reconsideration if the IO model or latency costs change. No whole ADR is certified by this bounded review.


A stronger §9.9.1 glyph-selection test exposed a Type1/OpenType CFF defect. An explicit PDF encoding maps codes 65 and 66 to `two.sups` and `two`, but a competing Macintosh cmap wins first, selecting A and B. The source routes this font to §9.6.5.2, where the explicit Type1 encoding overrides its built-in mapping, including for symbolic fonts. The same CFF bytes embedded directly as Type1C select the required glyphs. The regression fails with GIDs [3,4] instead of [2,1]; its bare-CFF control and the other three OpenType cases pass. This remains an unfixed production defect.

Parent review replaced corpus-dependent draft fonts with two tiny synthetic rectangle fonts and their offline reproducible generator, corrected flags and CID glyf cmap filing, and checked expected glyph names, indices and nonempty outlines with FontTools. The simple glyf case swaps A/B; composite CFF and glyf cases cover identity mapping. Nonidentity CID-keyed CFF remains open. The selected ADR 0015 TrueType route and ADR 0050 exact-name lookup remain justified within their scopes; neither justifies using the TrueType symbolic route for valid Type1/CFF. ADR 0029's separation of mappings remains sound, with its other branches still unverified here.

The host/core printing handoff now has a passing complete-PDF consumer test. Actual `printing::placed` and `cell` values feed public `Viewer` printing; rasterized `PrintPage` bounds match independently calculated paper coordinates in two-up and four-up layouts, retain the 20-point physical size, stay within their cells and do not overlap. This checks uniform page sizes and the public interface; native printer integrations and mixed-page geometry remain outside this test.


The markup/state/ink review adds eight consumer regressions: seven pass and the known Contents writer departure fails. Edited paragraph text serializes as LF instead of the CR required by EC3 #90; the separate single-line control passes. FDF reply names resolve to the exact existing same-page annotation after saving and reopening, while absent and other-page names are refused with IRT/RT omitted. Existing XFDF tests already covered parts of the shared resolution path; the draft's claim that no such exact assertion existed was corrected. The governing link-resolution decision is ADR 1297, not ADR 1224.

Markup review also distinguishes individual Table 172 duties, optional defaults, group attributes and Table 173 external-data constraints. The draft omitted ExData's required Type/Subtype; those are now inventoried. MarkupGeo has no additional entries or stated display operation, consistent with ADR 1212's decision to avoid a consumerless reader; its producer constraints remain unverified. The earlier no-formatting decision in ADR 0199 was superseded in part by ADRs 1634/1642 and is not a current blanket exclusion. Existing group/thread assertions cover selected values, not all ten group keys across every host.

The annotation-state regression passes through a comment-only reply to the later state update. It asserts the user, model, state and source object, plus an empty result for the leaf itself. Missing required T or StateModel in existing fixtures is tolerant recovery, not a rule supplied by the standard. The state model has no native-host consumer, and this clause does not independently demand a comments panel.

Ink tests now distinguish a bent three-point InkList and a genuinely bowed cubic Path from a shortcut or chord; both pass. The Path fixture deliberately exercises missing-AP construction policy and is not a conforming PDF 2.0 writer witness. A separate complete PDF with a stored appearance proves public activation opens the Ink note's popup with its exact text. In an isolated checkout, replacing cubic drawing with a straight chord leaves the old collinear test passing but makes the new cubic test fail; both unmodified controls pass. The mutation was restored and checked against live source. This directly demonstrates the old assertion's limit without alleging a production curve defect.


Broad checkpoint wave 14: all nine crates ran 5,713 tests, with 5,670 passing, 43 expected audit failures and 53 skipped. The exact binary/test failure set equals the manifest; there are no unexpected failures. Runtime was 60.950 seconds, with compilation making the full command 200 seconds. The new Type1/OpenType and Contents paragraph failures are included. Targeted Clippy checks passed after the latest test additions.


The FreeText review adds standalone annotation fixtures with an empty AcroForm Fields array. Earlier shared fixtures also put the FreeText object in Fields, so their behavior does not establish a valid field hierarchy. The first new draft had another weakness: its ink bounds included the border, allowing border movement to impersonate text movement. The accepted tests disable the border. Missing Q equals Q0, Q1 centers the text, and Q2 moves it right. Four independent RD comparisons verify left/right translation, top translation, and bottom clipping while preserving pixels above the clip. Both tests pass. EC3 #299 changes BS's minimum version to 1.3; #524 changes RD's type to array. Producer version duties remain separate from reader tolerance.

The Line review adds a passing public activation/popup regression and a passing diagonal-caption test. The latter compares omitted CO with explicit zero, then verifies the actual glyph transforms move in the line's axes for positive horizontal and negative perpendicular offsets. It does not certify RC-only or non-LTR captions. Existing shape comparisons distinguish different endings but do not by themselves prove every Table 179 geometric constraint.

A new regression exposes a conflict between Table 178's explicit LE endpoint wording and ADR 0192's chosen placement with nonzero LL. First/second endpoint controls without LL pass. With LL20, both square centers move from the L coordinates at y50 to the displaced line at y30. The ADR says Figure 80 draws an arrowhead on the line proper; parent inspection of the original PDF found no such arrowhead, only separate measurement arrows. That figure does not substantiate the claimed exception to the table. The regression requires the literal L-coordinate mapping and fails; this is recorded as an interpretation conflict requiring follow-up, with historical ADR and production behavior unchanged. The missing-AP fixture deliberately exercises construction policy rather than certifying a PDF 2.0 producer. The failure manifest now contains 44 assertions.

Selected FreeText ADR decisions retain on-page layout, subtype-specific construction, rich-text routing and regeneration after edits with scope qualifications. The rich-text-only whole-regeneration sentence in §12.7.4.3 does not independently justify replacing every plain FreeText stream; that broader ADR 0304 argument remains unresolved in this review. No whole ADR or whole annotation clause is certified.


Further Figure 80 review changes the earlier provisional conclusion about ADR 0075 as well. Table 178 describes leaders extending from the line proper in the signed direction; L holds their far endpoints. Thus the proper line is L minus the signed clockwise vector, whereas the implementation adds it. The original figure's positive and negative examples support subtraction. An independent agent reached the same reading. Existing tests pass because they encode the ADR's reversed convention. The new test first passes missing-LL/explicit-zero controls, then fails for both signs, with the proper line on the opposite side in each case. This is a second line-geometry finding, separate from LE endpoint placement; the manifest now has 45 failing assertions. Historical ADRs and production code remain unchanged.

The FreeText assertion correction was also mutation-tested in an isolated checkout. Removing all FreeText glyph output while leaving its border intact still passes the original RD draft and fails the accepted borderless test. Both unmodified controls pass. An earlier mutation that merely ignored RD for text failed both versions and was not used as evidence that the draft missed that narrower fault. The source was restored and compared byte-for-byte with the live checkout.


Polygon/Polyline review reproduces three construction defects with controls before each failure. PolyLine discards IC before drawing its endings, so a Square ending stays unfilled; the equivalent Line ending fills and an open-style control ignores IC as expected. The first PolyLine ending is placed at the second vertex: the call passes the first style with the reversed first segment but applies it at that segment's first point. The old first/last-leg test merely checks that adding arrows changes some pixels. A new three-collinear-vertex comparison matches plain and last-ending Line controls, then fails at the first ending. Ordinary Polygon Path also omits the closing stroke, while explicit segments and the open PolyLine control behave as expected. The draft incorrectly described a late close call; there is no close in that branch at all. These add three expected failures, bringing the manifest to 48 assertions.

Effective EC3 #444 changes both Table 181 cells. The draft saw the Vertices amendment but missed the facing-page replacement of “not be present” with “be ignored” in the Path entry. The effective text no longer prohibits Path/Vertices coexistence. New PDF 2.0 fixtures retain required Vertices as a distant decoy, verify it is ignored, and include exact cross-references and an ID; missing AP is deliberate construction-policy scope. The current source comment's non-presence rationale is stale. Path/LE still has an interpretive gap because the LE cell names coordinates in ignored Vertices; its current refusal is not certified as a mandatory standard behavior. ADR 1784's Polygon LE no-op remains supported, while its broader historical IC audit claim is contradicted by the new fill regression.


Subset-font review found a concrete violation of the same-document tag uniqueness requirement in §9.9.2. Two distinct subsets of bundled Liberation Sans Regular, with glyph closures {0,42,220,488,570} and {0,38,97,120,135}, both receive `NQYCPV+LiberationSans`. The new unit exercises production `machine_font` and `for_a_file` into one resource dictionary: distinct program streams and matching BaseFont/FontName controls pass, then uniqueness fails. Parent review traced resource promotion in `view/writing.rs`; it numbers streams without reconciling tags. This is production writer-seam evidence, not a complete public save test. ADR1425's assertion that different subset inputs differ is disproved; the case involves two newly generated subsets, beyond its separate accepted risk of collision with an existing producer tag. The manifest now contains 49 expected failing assertions.

A public reader control embeds two validly tagged subsets from the same source font. It verifies distinct nonempty outlines, repeated-glyph equality, stated advances and two cache entries; a second interpretation preserves all display commands while adding cache hits without misses. The draft incorrectly assumed intrapage repeated Tf use necessarily increments the public FontCache hit counter. This supports the clause's independent-subsets recommendation, which is a NOTE rather than a separate SHALL. Selected CFF naming/glyph-zero decisions remain qualified: the collision witness is TrueType, and full CFF dependency closure was not re-certified.

Square/Circle review adds three passing tests: asymmetric RD with unequal ellipse axes, IC component-count consumers including transparency, and one-point default border with an explicitly stated colour compared against stored AP. The proposed absent-C default-black failure was rejected because the cited clauses do not establish that colour oracle. The current C-to-outline mapping and absent-C transparency remain policy/interpretation questions. Missing AP deliberately exercises construction policy; producer version and full Circle popup duties remain uncertified. ADR0674's selected extreme-width behavior has meaningful colour/extent assertions, but its claim that frame closure and inset-path degeneracy occur at the same width is queued for separate geometric review.
