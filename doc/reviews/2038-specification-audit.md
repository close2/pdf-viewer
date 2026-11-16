# Specification audit — round 2038

Baseline: `b901326c230faa5d2cb288016339b9d5c6288527`, pulled by the owner on
2026-11-16. Branch: `audit/specification-evidence-2026-11-16`. This is a new
audit round carrying forward the [previous inventory](1983-specification-audit.md),
not a restart and not a certification of the updated implementation.

The current ledger has 883 rows: 780 implemented, 10 departed, 8 partial,
2 writer-side, 32 inapplicable and 51 out-of-scope. Prior partial reviews are
retained as provenance; they are not silently promoted to current verification.
The [evidence directory](2038-specification-audit/) records the baseline, per-clause
continuation inventory, and work still pending.

## Method and current work

Recheck all 72 previously failing assertions against the updated code and the
effective ISO specification, including EC3. These are assertion counts, not
distinct-defect counts. Review actual assertion bodies, valid fixture controls
and public consumers independently of execution. Then expand into previously
pending features and selected ADR decisions. The first new shard covers logical
structure and attributes, including effective clause numbering.

All **72 prior failing assertions now pass** at this baseline, including the three
GTK cases run under Xvfb. The broad 11-crate run selected 6,518 tests: 6,508 passed
and ten failed while opening one missing corpus witness. After linking the existing
corpus into this worktree, those exact ten passed. The separate GTK library run
passed all 20 tests. There were 55 skipped tests in the broad run. Execution and
source/assertion reviews remain separate in the evidence files; none of these
counts certifies complete feature coverage.

The 72 scoped reviews revisit parts of 51 ledger clauses. Parent review corrected
historical-log confusion and obsolete ADR residuals before recording the current
assessment. In particular, Qt choices now retain an uncommitted selection; archive
and imported-FDF subset naming have successor implementations (ADRs 2838 and 2862).
Their old residual notes are not evidence of current defects.

New assertion work has already found a remaining speech gap: `/Alt` and `/E`
Unicode language escapes leave the spoken language at the prevailing `/Lang`.
Both controlled tests fail with `en-US` instead of `es`. A three-run `/ActualText`
control passes, including multibyte offsets; ADR 2812's mid-string residual is
obsolete for that path. These are two failing assertions on related handling,
not a count of independent defects.

Date regressions now classify their known valid/invalid fixtures independently
of the parser also used by the writer. FDF callback tests explicitly compare
execution positions, with the `After`/`AfterPermsReady` ordering identified as
ADR 2816's choice rather than a separately stated ISO order. These strengthened
assertions pass. The next batch found six failing assertions: wide/deep structure traversal silently
omits content without its promised truncation flag, and details-view collection
cells omit CI text/date/numeric values and compressed size. The structure
attribute-stream control and PDF-level JBIG2 filter-chain assertions pass.
An encryption control also fails: a `CF/Identity` entry is parsed despite Table20
requiring it to be ignored. There are now nine expected failing assertions; related
failures are grouped rather than counted as independent defects. The combined
28-test run has 19 passes and exactly these nine failures. Artifact reporting and
the two added colour arithmetic/transfer controls pass. A shared-date-parser
mutation is killed by both strengthened invalid-input tests, while valid controls
pass; the source was restored and all date targets then passed.

The next controls expose two additional failures: same-key string destinations prefer
the catalog dictionary over the name tree, and stitching uses signed-zero ordering
where the degenerate endpoint needs numeric equality. The destination finding states
the competing interpretation explicitly. There are eleven expected failing assertions.
Cross-reference fixture repairs and additional ordering/default-width controls pass.
Three public structure-element `ActualText` ligature tests pass; a preliminary
missing-consumer claim was rejected.

Review of ADR0601 supports the existing repeated-letter page labels but revises its
conflict premise: official issue #432 was withdrawn in favor of #593 before the ADR
was written. The static PDF retains stale accepted annotations. Source links and
chronology are recorded in finding D03; the added public AA/BB label control passes.

Further controlled tests bring the current total to fifteen failing assertions:
sampled functions collapse a tiny nonzero domain, a base image accepts a nonnumeric
Decode endpoint without error, and structure-level `ActualText` is duplicated over
two direct content items or omitted when those items belong to descendants. These
last two assertions share one substitution-design finding. An exact-xref paired
font fallback test passes, including independent glyph positions and refusal reporting.
The PLRM3 review also qualifies 32-bit shifts as local policy, while reaffirming
mandatory zero-fill. Deferred Type4 faults still lack host warning integration, a
limit already acknowledged by the ledger and ADR2848.

The form reset works in memory but loses distinct sibling defaults after saving.
JPX opacity codes 0 and 1 pass independent pixel controls; code 2 refuses a valid
premultiplied-opacity channel before rendering. These bring the total to seventeen
failing assertions. Independently regenerated JP2 fixture hashes are recorded.
The [shard index](2038-specification-audit/shard-index.json) links bounded feature
reviews to ledger owners, with published/effective numbering translations.
Neither these links nor the ledger’s existing test references certify whole rows.
The six-crate checkpoint ran 5,358 tests: 5,341 passed, exactly the 17 recorded
assertions failed, and 50 were skipped. Clippy, formatting, rustdoc with warnings
denied, and the full conformance-tool package pass.

Five further assertions expose named-glyph mapping gaps: multi-scalar AGL rows,
multi-group uni names, and known components beside an unknown component are lost.
The sequence defect reaches public extraction and search, with real Type3 raster
and single-character controls. These bring the manifest to 22 failing assertions.
Four source-derived mesh pixel tests pass, including patch-function interpolation.
The [table coverage index](2038-specification-audit/table-coverage-index.json)
records 411 structural owners and 63 missing literal table citations; those are
triage locations, not 63 missing features. Per-entry review remains necessary.

A separate static finding concerns PAdES verification: the initial Open path
defers checks required by the clause’s open-or-request trigger. Genuine signed
and same-range-tampered public Report controls pass, with an explicit host anchor
and unknown-revocation policy. They establish later verification, not Open-time
execution. No missing UI note is presented as proof of an unperformed check.

Further controls pass for all four named actions, packed and 16-bit PNG byte
prediction, Type6 inherited edges, Type7 asymmetric control ordering, and non-unit
axial/radial domains. Independent Bernstein inversion supports the tensor pixel.
A fresh PAdES generator run passes syntax and signature/tamper controls. The full
font suite has 397 passes and exactly the three recorded font failures.

The owner-authorized redaction-overlay change is recorded in A842 and ADR 2868;
the MaxLen change is recorded in A834 and ADR 2840. Their changed assertions must
be assessed against those current decisions rather than treated as unauthorized
weakening. Review of their behavior and evidence remains scoped and ongoing.

Additional pixel controls pass for clip-rule differences, clip-after-stroke timing,
graphics-state restoration, nested optional-content visibility and null membership.
A public first-viable media handoff control passes. The MU/NU overlap remains an
interpretation question. EC3#549 rejects a preliminary path-error hypothesis; stale
test comments are corrected. Parent reproduction confirms the sampled Annex D
full encoding tables, including note-defined assignments. The index now links
partial evidence to 217 ledger rows; 666 remain pending in this round.

The timestamp-only DocMDP comparison now adds one failing assertion, bringing
the manifest to 23. Both signatures in each fixture pass integrity and key checks;
the independently signed page-change companion is correctly refused. Parent
verification extracts the actual PDF ranges and checks OpenSSL CMS/RFC3161 tokens.
Page-resource inheritance, inline-image byte/name controls, repeated transparency
group invocation, and public-key permission controls pass. A stale inline-image
rounding description and transparency-test header are recorded separately from
behavioral defects. Partial evidence now reaches 247 rows; 636 remain pending.
The eight-crate broad run has 6,358 passes, exactly all 23 expected failing
identities, and 55 skips. Current touched-crate Clippy and formatting pass.

The next checkpoint adds a failing empty-intersection boundary control for all four
non-media boxes. It challenges ADR 0414's inference that an empty intersection may
use an absent-value default; a separate Media viewport avoids zero-size output.
Type 3 state restoration, all four text-markup popups, inherited CropBox rendering,
FDF template pixels/file requests and separately labeled metadata sources pass.
Table 397 defaults now use literal source values instead of implementation constants.
The manifest has 24 expected failures; partial evidence reaches 260 rows, with 623
still pending. Current touched-crate Clippy passes. Source-locator and draft-premise
corrections are explicit in the shard parent reviews and execution log index.

The latest checkpoint confirms that a degenerate tiling BBox loses its clip,
challenging ADR0056 with a distant-pixel failure and valid ordinary/boundary controls.
The manifest now contains 25 expected failing assertions. Deterministic CID mapping,
closed parser-limit boundaries and three nested associated-file controls pass.
A misleading hex-string source attribution is recorded separately. Full reading of
ADR0746 rejects a preliminary stale-rationale concern; its erratum handling is current.
Partial evidence reaches 303 ledger rows, with 580 pending. Current syntax/model
Clippy passes; initial draft errors and their corrections remain in the execution log.

A long header prefix exposes a further incremental-update defect: recovery resurrects
an object deleted by the newest xref section. Original-object and shorter-prefix
controls pass; recovery is reported, but its object semantics are wrong. The manifest
has 26 expected failing assertions. Vertical word spacing/TJ, omitted nonfinal-page
duration, and strengthened ASCII85 marker controls pass. A stale vertical-text ledger
note is recorded without changing its status. Partial evidence reaches 328 rows, with
555 pending. Current syntax/model/core all-targets Clippy passes.

The initial-open ordering check finds page and annotation actions before the catalog
OpenAction action, contrary to Tables 198 and 197. The catalog-only and no-catalog
controls pass, and presentation does not repeat the actions. The manifest has 27
expected failures. Valid multi-level tree consumers, literal preference defaults,
whole-entry border precedence and stored-appearance blend pixels pass. A preliminary
claim that the existing byte-key fixture was unsorted is rejected: its stored keys
are sorted, and query order is separate. Partial evidence reaches 338 rows, with 545
pending. Current model/core Clippy passes; broad execution retains its earlier scope.

Remote and embedded go-to tests now confirm that target page selection drops the
requested view. Separate centering checks expose FitR/FitB corner placement and an
existing wrong test oracle; matched-aspect and whole-page Fit controls pass. A
validated later-version ICC profile exposes missing source-range clipping before
Alternate conversion. The manifest contains 33 expected failures. DeviceRGB reset
passes, and the MissingWidth erratum omission is recorded as a ledger-description
gap, not a rendering defect. Partial evidence reaches 356 rows; 527 remain pending.
Current model/core all-targets Clippy passes. The broad run will be refreshed.

The refreshed eight-crate run has 6,382 passes, exactly the 33 manifest failures,
and 55 skips. Identity reconciliation found no additional or missing failures.
Four new public action/URI assertions pass: repeated siblings, cycle termination,
click offsets and rounding, default/false and catalog-open IsMap behavior. Partial
reviews now reach 360 rows, with 523 pending. Viewer-core Clippy passes. The next
text-extraction drafts are withheld pending source-valid embedded-font fixtures;
a custom encoding with a nonembedded Type2 descendant violates Table 119.

The next checkpoint confirms three further failures: duplicate choice labels lose one
selected index, ToUnicode conflates source-code lengths, and a sequential multi-character
destination maps only its first code. Valid embedded-font controls and decoder/GID
checks isolate the latter failures. Valid UseCMap inheritance and three special-colour
pixel controls pass. The manifest now has 36 expected failures; the broad run above
predates these three additions. The #309 errata description confuses Colorants with
None; its physical-page locator is correct. Partial reviews reach 369 rows, with 514
pending. No source or ledger status is changed.

A valid composed-transform integration control passes, including CropBox offset,
rotation, UserUnit and Q restoration. The earlier mid-path q/Q assertion remains
as explicitly labelled robustness coverage. Image sample/coordinate tests have
literal oracles; interpolation mixed-gray assertions check policy, not a required
filter. Primary #79 annotations confirm the existing alternate-image branches.
Partial evidence reaches 379 rows, with 504 pending; the failure manifest remains36.
Current model all-targets Clippy and the12targeted path/state tests pass.

The annotation checkpoint adds passing Stamp and Caret stored-appearance controls,
each discriminated by red/blue stream content. Primary #444 on the Ink table
contradicts the current documentation that Path precedence is merely a choice;
the code already prefers Path. That source/ledger rationale gap is D12. The table
citation triage separately records two Annex-prefix search misses among63candidates;
all63 have structural owners, which does not prove feature-entry completeness.
Partial reviews reach383rows, with500pending. The refreshed eight-crate run has
6,399passes, exactly all36expected failing identities, and55skips. No extra or
missing failure was found. Current model all-targets Clippy passes.

## Budget and continuation

The account's weekly allowance meter read 29% used at the start. The owner asks
to leave 15%; stop new dispatch near 81% used and reserve the interval through
85% for validation, evidence and handoff. Percentages are account quota readings,
not a known fixed token conversion. Preserve unfinished work explicitly rather
than reduce review quality.

No production behavior, ledger statuses, or historical audit records are changed
by this audit. A test-only module hook exposes no new production API. Tests and findings will be committed and pushed in reviewable batches.
