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

## Current checkpoint (120)

Partial reviews now cover **883 of 883 ledger rows**, with 0 wholly pending. The current
failure manifest contains **112 failing test identities**: 111 behavior assertions/probes
and one stale ignored-gate expectation. These are grouped findings, not 112 independent defects. B08 and B42 remain unresolved interpretation/policy
probes; this is not a count of confirmed mandatory ISO violations. All 72 failures from the prior audit pass at this baseline.
The refreshed broad run, wave116, covered eleven crates: **7,305 tests ran, 7,205 passed,
and exactly the 100 recorded in-scope assertions failed; 58 were skipped**.
Exact binary/test identities matched the manifest at that checkpoint, with no extra or missing failures.
Wave116 includes all default-scope additions and strengthened assertions through checkpoint115.
Passing status includes successful early returns; two are reproduced under D45.
Five GPU failures and one engine-only Matrix2D availability failure under the adopted
A193 reference scope are recorded separately, as is D51’s ignored stale test expectation. The new outline-panel raster test also
passes separately; an existing LZW test was strengthened.
A separate engine-enabled run adds **247 passing script/worker tests**, including
two new string/stream execution controls; four ignored tests are separate.
The subsequently selected corpus pair also passes across90,763 PDFs. Its counter
assertions measure execution/error populations, not correct script effects.

Checkpoint50 adds two failures: enabled rendition JS is skipped in favor of OP,
and TrueType dictionary encoding omits the final StandardEncoding fill. Five new
controls pass; model/core Clippy passes. A detached-page hypothesis was withdrawn
because page templates invalidate its premise. Script corpus throws are being
classified against the admitted API surface and producer dependencies.

Checkpoint51 confirms that VisiblePages omits groups on a second visible page.
AllPages/collection-label and list-wide language-rendering controls pass. A real
incremental save preserves the signed prefix and cryptographic evidence while
reporting an unsigned tail. The tree-depth comment/completeness concern remains
a qualified design finding; informative Annex C does not impose a new UI duty.

Checkpoint52 adds the qualified Matrix2D constructor gap. Stored appearance
precedence, cropped/rotated navigation, LZW default handling, and actual outline
colour controls pass. The initial navigation draft masked coordinates with scroll
clamping; its failures were rejected. Older unavailable source claims about the
script helper were withdrawn; the pinned official example supports the recorded gap.

Checkpoint53 reconciles the broad run and indexes late-page errata annotations.
Four selected overlays add excluded runtime behavior, an explanatory namespace note,
and a bibliography entry; none establishes a new supported-reader defect. The issue
index is a discovery aid, not certification of every correction. An earlier namespace
source-page locator is corrected in the parent review.

Checkpoint54 adds two passing syntax assertions: permitted indirect object-stream
dictionary values and the documented current-entry `/Size` departure. An existing
stored-appearance blend test gains an outside-region control and still passes.
All ten departed rows were reconsidered; the current redaction limit is already
accurate, and stale scan-conversion wording is the existing D13 finding. The early
errata index covers source locations, not a verdict on every correction.

Checkpoint55 confirms one premature-termination failure in the adopted implicit
table-header search, with three passing controls. EC3 #200 uses “should” guidance
for an algorithm readers “may” adopt; the finding preserves that qualification.
The broad53 run predates this new failure. All eight partial and two writer-side
rows retain their qualified status after review. The audit's earlier Annex L
bidirectional paraphrases are corrected: P may parent Table, and WP may parent
Figure; the reverse edges are forbidden by the checked matrix cells.

Checkpoint56 finds two failures within one parameter-resolution gap: indirect
`Predictor` and `Columns` values change decoding despite identical direct controls.
Two outer dictionary/array representation tests pass. These are ordinary data
streams, so the special direct-only rule for cross-reference streams does not apply.
Syntax Clippy passes; the failure manifest now has 78 entries, 74 in the broad scope.

Checkpoint57 adds four passing representation controls: annotation pixels and
NoView, GoTo link targets, sampled-function mappings, and CalGray parameters.
The function fixture uses nondefault values that make each tested field observable.
A too-tight colour precision assertion was rejected; exact reference equivalence
and parsed values still pass. Model and colour Clippy checks pass.

Checkpoint58 adds two passing graphics-state pixel tests. Nested indirect dash
values use a nonzero phase that makes default fallback observable; indirect blend
names and alpha preserve the independently calculated composite. Model Clippy passes.

Checkpoint59 adds two passing font metric tests. The simple-font control uses the
public metrics-only consumer so a program fallback cannot mask ignored Widths.
CID controls use exact embedded-program widths and literal vertical metrics.
Font Clippy passes. The refreshed broad run matches every in-scope failure identity.

Checkpoint60 reconciles 24 selected errata issue groups with their primary targets.
It corrects draft source ownership and distinguishes informative notes, producer
constraints and reference-version rules. Existing literal assertions support selected
encoding, timezone, object-stream and metadata paths. No new failure or execution is
claimed; full soft-mask restrictions and linearization resources remain partial.

Checkpoint61 reviews 51 further selected errata issue groups and adds two passing
tests: composite-font literal escape ordering and missing attribute classes.
Model Clippy passes. The corrected Courier-BoldOblique list exposes a stale source
premise in current comments and historical ADRs; tolerant alias behavior remains valid.
The review withdraws a stale draft rich-text exclusion: current layout/save tests and
ADRs 1634/1635 supersede ADR 1122. No new behavior failure is established here.

Checkpoint62 adds two passing axial/radial pixel tests for nested references,
including nondefault domains and both extension boolean polarities. Model Clippy
passes. Twelve further errata groups were reviewed. Current FDF code already
implements the #173 distinction; #4 changes a deprecation note, not the signing
condition. No new behavior failure is established; the manifest remains 78.

Checkpoint63 adds two failing old-version linearization assertions: PDF1.0/1.1
headers remain unchanged below EC3 #331's floor1.2; the1.2 control passes.
Media base-URL and indirect font Differences controls pass. Model/transform Clippy
passes. The manifest now has80 assertions,76 in broad scope; wave59 predates these
additions. The remaining23 literal errata candidates were reviewed, including a
source cross-reference conflict at #179. Zero absent issue-number citations does
not establish complete semantic coverage; a disposition crosswalk is underway.

Checkpoint64 confirms two runnerPASS results skip their missing named fixture
inside a present corpus. Passing counts are therefore not assertion-execution
counts. The current habit distinguishes this from an absent optional corpus.
A new deterministic Open report-event test passes; core Clippy passes. Four other
producer witnesses initially called absent are present in the audit worktree and
pass selected execution; the reviewer had inspected the owner checkout. Broad64
matches all76 in-scope failures, with no unexpected or missing identities.

Checkpoint65 adds passing independent CalRGB pixel samples and a combined annotation
NoZoom/NoRotate test with a rotating appearance matrix. A temporary zero-transfer
mutation survives the old CalRGB identity oracle, but both an existing CalGray test
and the new pixel test catch it. Exact source restoration and all three passing
checks are recorded. Model Clippy passes. Second review keeps B08/B42 explicitly
unresolved, B40 an adopted SHOULD policy, and B24 narrowly about centering.

Checkpoint66 reconciles the errata inventories: the two older per-issue indexes
omit four later markers in grouped subjects, while the current 360-ID census is
correct. A reproducible crosswalk records source locations and evidence pointers,
with automated record shapes explicitly separated from semantic coverage. The
named-destination grouped correction now has an explicit bounded disposition.
Second review retains the three revocation findings with their existing consumer,
current-versus-historical-time and capture-provenance limits. No new failures.

Checkpoint67 adds three passing tests for annotation print/hover interaction,
actual raster-export visibility policy, and indirect embedded-file parameters.
Literal pixels, dates, payloads and checksums supply independent expectations.
A draft legacy EF/Unix case was excluded because current Table43 permits F/UF
keys there. Model/transform Clippy passes; the 80-assertion manifest is unchanged.
The broad64 run predates five passing additions through checkpoint67.

Checkpoint68 adds passing GTK settings and Qt bridge print-default tests. They
check literal ranges, copies, scaling and duplex values, while preserving the
native-dialog/printer evidence limit. Native all-targets Clippy passes.
The source inventory now records unnumbered editorial notes as well as numbered
issues. A deeper AnnexL review verifies six current matrix cells and the actual
merge-wrapper assertions; it does not claim a complete hierarchy validator.

Checkpoint69 adds three passing syntax tests for incremental stream-to-stream
cross references and object-zero free-list updates. Thirty weakly classified
errata receive deeper source/assertion review. Two stale source/design explanations
are recorded without changing valid writer choices or inferring parser failures.
Syntax/GTK all-targets Clippy passes; the GTK fixture now includes required PDF2 IDs.

Checkpoint70 finishes deeper dispositions for the remaining25 selected errata
candidates and adds one passing chained-inline-image assertion with an independently
checked sample oracle. Parent source review corrects draft readings of CMap and
output-profile URL restrictions. Existing choice-selection failure B26 is qualified
against the general array-value rule; it is not the specific erratum468 trigger.
Model all-targets Clippy passes; the failure manifest remains80.

Checkpoint71 adds two passing tests: hexadecimal language values at the public
accessibility consumer, and distinct predictor/image bit depths with independent
exact-pixel controls. Model/core Clippy passes. Binary transport review confirms
an existing full binary-prefix save assertion and qualifies the narrower C API
evidence. No new runtime failure is added; structure-split cardinalities remain
under review because preserving edges alone does not prove required sequences.

Checkpoint72 confirms four real-window transition sequence checks under X11:
GTK, Qt, Quorra default and Quorra CPU. These supplement static geometry tests;
captured endpoint bounds alone do not prove exact effects or hardware provenance.
The69 broad file references are reconciled: all have named siblings, and three
sampled rows contain five of those references. Silent-skip follow-up preserves
existing findings and their limits. No new runtime failure is added. Broad64 now
predates11 new broad-scope passing tests; native checks are recorded separately.

Checkpoint73 adds a split-preservation failure: Warichu's required WP/WT/WP
sequence becomes WT when the boundary elements omit optional K. The text-bearing
boundary control passes; the source-validity premise and successful-split contract
are explicit. An omitted black-point-compensation gs control also passes. Model and
transform Clippy pass. The manifest now holds81 assertions; broad64 predates this
failure and13 subsequent passing tests. The Screen ledger still quotes a sentence
deleted by #42, although current code and ADR2818 already qualify that old rule.

Checkpoint74 adds a public-key opening failure for valid distinct stream/string
default crypt filters. The shared-filter control passes; OpenSSL independently
decrypts actual retained PDF payloads. Type4 mesh continuation coordinates and
inherited colors also pass literal expectations. Model/syntax Clippy passes.
The manifest reaches82 assertions; broad64 predates two failures and15 new passes.
Erratum #218 targets Table263 UR3, not Table264 Legal; current rights consumers
exist, while malformed-cardinality recovery remains a policy boundary.

Checkpoint75 adds passing Type5 multirow topology and nonlinear Function pixel
oracles. Eleven calculator reference/device tests pass on the recorded Radeon890M.
The T.88 reference test first returned without data; after restoring verified
official data it executes all16 classified cases and passes. Eight require exact
bitmaps, seven require nonmatches, and one only agreement between isolation modes.
The explicitly run ignored fixed-document gate fails two checks on one of77present
files: expectations predate accepted font substitution. D51 records that stale
expectation without changing its baseline. Model Clippy passes. Broad64 predates
17new passing tests and two failures; the ignored stale gate is outside its scope.
AnnexE private-name and writer claims remain bounded; Table49's amended required
URL conflicts with surviving optional prose, and custom structure types are expressly allowed.

Checkpoint76 refreshes the eleven-crate run with an exact match to all78expected
in-scope failures. Four selected ignored corpus tests pass with fixtures present:
three archive witnesses,963opened disk/memory documents, three composite-font
witnesses at three targets over every two-byte code, and the accessibility census.
The transport count includes generation-zero lookups; nonzero-generation objects
can compare null on both sides. A stronger oracle is being drafted. A ratio-only
TJ-spacing assertion has a separate blind spot under review. The font fallback
control now includes required Length1 and passes; model Clippy passes. An exact
pointer index records34of69file-only references as partially assertion-reviewed,
with35remaining. These counts do not certify entire files or clause implementations.

Checkpoint77 strengthens two oracles. Disk/memory comparison now uses each actual
object generation, with a new literal generation 7 witness; nine selected syntax and
archive tests pass, including the corpus. The horizontal TJ scaling test gains no-TJ
controls and literal +10/+5 point deltas. A zero-adjustment mutation passes its old
ratio and fails the new delta; production was restored byte for byte and the test
passes again. An existing positive-TJ test already supplied an independent magnitude
oracle, so the old suite was not wholly blind. Archive witnesses now distinguish an
absent optional repository from a missing named file within one. Three-crate Clippy
passes. Five earlier behavior findings survive fixture/oracle second review with
narrower qualifications. The file-only index reaches 44 of 69 partially reviewed pointers;
Windows actual HideWindowUI visibility remains an unexecuted integration proposal.
Broad76 predates the new generation test and these stronger oracles.

Checkpoint78 replaces two weak expectations and repairs unrelated fixture faults.
The Indexed default-Decode test now checks every one of 256 indices exactly, instead
of four with a one-level tolerance. An index-minus-one mutation passes the old test
and fails the stronger one; production is restored and the stronger test passes.
The press-routing test uses an independently calculated literal color, scoped to its
synthetic profile and selected output policy. Incorrect stream lengths in Contents
and damaged-page fixtures are computed exactly. All 32 selected tests pass; model
Clippy passes. Two GPU test comments now distinguish shading-sampling policy from
the specification's image-specific pixel-centre rule. The exact file-only pointer
index reaches 56 of 69 partial reviews, with 13 under active review. No new behavior
failure or new test identity is added by this checkpoint.

Checkpoint79 completes the selected-assertion review of all69 file-only pointers;
this is partial evidence, not certification of entire files or features. Both16-mode
GPU differential tests now explicitly skip in debug instead of returning success
without assertions. Both actually execute and pass in the release-derived gates
profile on the Radeon890M. A new parsed-PDF test confirms repeated invocation of
a group uses its changed backdrop. All three affected crates pass Clippy.
Historical ADR1242's profile exception is superseded by2549; audit evidence records
that distinction. Deeper descriptor-confinement, encrypted-output and valid-ICC
reviews continue. The failure manifest remains83; broad76 predates the new test.

Checkpoint80 strengthens descriptor identity tests and adds independent color/encryption
controls. The real confined worker still renders the held file after its pathname is
removed; protocol and kernel probes pass. An ICCDEV-validated v4.4 Gray profile selects
three distinct intent transforms with independently calculated pixels. R6 writer fields
match a PyCryptodome-derived vector, whose helper is cross-checked against qpdf output.
All17 selected tests pass; confined/syntax/model Clippy passes. Tampered permissions
comments now identify malformed-input policy, followingD16. The manifest remains83;
this checkpoint adds two passing test identities. Further ADR reviews are active.

Checkpoint81 adds independent output-protection and stroke-width controls and strengthens
XFA attachment metadata assertions. All6 selected tests pass (5 new identities), and
transform/render-raster Clippy passes. Unequal-scale stroke geometry is correct in the
sampled public paths; the initial suspicion from a lower-level trace was withdrawn.
FileSelect acquisition timing and Archive+Protect API handling remain qualified decision
findings (D55/D56), with no production changes. The expected-failure manifest remains83.
Weekly usage was65%; further geometry and archive-policy reviews continue.

Checkpoint82 verifies named-page appearance geometry and saved output with independent
RGBA controls. That test and the valid XFA attachment control pass. A new malformed-input
integrity test fails: the archive attaches only part of an XFA resource while reporting
preservation (B58). This is a repository contract issue, not valid-PDF nonconformance.
The expected-failure manifest is84; model/transform Clippy passes. Grammar-only dynamic-XFA
refusal remains a policy proposal (D57), outside the manifest. No production code changed.

Checkpoint83 confirms a stroke-width defect at the near-similarity cutoff: width1000
under horizontal scale1.009 becomes1008 pixels thick where1000 is required. Identity
and perpendicular controls pass; direct display-list and parsed-PDF paths fail (B59).
Earlier strongly anisotropic cases still pass. The manifest is86; render-raster Clippy
passes. Clip/flattening review qualifies historical mathematical rationale without
asserting a source-prescribed fractional-antialiasing failure (D58).

Checkpoint84 adds three passing confined-host save tests: literal output bytes, source
preservation for ordinary/already-edited names, and failed-write preservation of a
conflicting directory and its contents (T114). The older no-form-fill/no-save premise
is stale; restriction-control UI remains a qualified policy question (D59). Viewer-ui
Clippy passes. The manifest remains86; no production behavior was changed.

Checkpoint85 adds explicit entry matrices for Tables57/224/226 and confirms two
form-name defects: coordinate names inherit an ancestor TM when they must use the
terminal field (B60), and ordinary HTML export ignores TM (B61). Local/absent/space
controls pass. A valid indirect graphics-state font test independently checks
advances, replacement and cumulative state (T115). Model Clippy passes; manifest88.
Stale pending-transfer and signature no-walk citations were corrected before promotion.

Checkpoint86 adds catalog/image entry matrices and a passing indirect catalog-action
control through public save. Exact script and trigger, plus a no-action control,
verify dispatch (T116). Catalog ledger prose still calls AA unread (D60). Image
statuses are reconciled to existing B11/B14 failures. Viewer-core Clippy passes;
manifest88 is unchanged. These matrices remain partial evidence for their cells.

Checkpoint87 confirms ancestor-only field AA is wrongly dispatched for a terminal
keystroke (B62); local and absent controls pass. Valid NeedAppearances save/reopen
controls and direct/indirect page actions pass (T117/T118). The Page dictionary
now has a 33-key partial matrix. Model/core Clippy passes; manifest89.

Checkpoint88 adds font-descriptor and linearization dictionary matrices. Four
selected font tests pass with output confirming their host guards did not skip.
The existing writer parameter test now checks format version1.0 across12shapes
(T119), and transform Clippy passes. Earlier AnnexF errata prose is superseded;
source notes337/403 are included. No new failure; manifest89 remains.

Checkpoint89 confirms a nonterminal default style leaks into a terminal rich-text
field (B63): 27 ink rows instead of9, with local/override controls passing. Separate
widget quadding precedence passes with literal edge/centre assertions (T120).
The cached inherited map is the cause; the rich-text consumer sees two dictionaries.
Model Clippy passes; manifest90. No invalid absent-required-RV failure is claimed.

Checkpoint90 refreshes the broad run:7,257run,7,174pass,83expectedfailures,58skipped.
Exact identities match the manifest with no extra or missing failures. Seven
manifest identities remain outside this scope. The original11ADR shortlist entries
now link to their current partial reviews; this does not certify the ADR catalog.

Checkpoint91 confirms ancestor TI/I leakage into terminal choice fields (B64).
Ancestor Opt also changes absent choices (B65), qualified by the table’s SHOULD.
The three assertions have passing local/default controls before their failures.
Model Clippy passes; the manifest has93 identities. These tests inspect the public
choice-control model, without claiming actual UI pixels or appearance regeneration.

Checkpoint92 adds two passing Required-field report/body/host-policy tests (T121),
including report-before-submit through the real viewer event path. This characterizes
a deliberately unmet Required condition; it is not a claim of conforming export.
The soft-mask test now observes restoration after Q (T122). Optional-content AND
is checked across16 cases; an old-test-evading mutation is caught (T123), and source
is restored. Nine more table matrices are partially mapped; model/host Clippy pass.

Checkpoint93 adds the missing rollover fallback control (T124) and a public
page-label test for absent style and empty prefix (T125); both pass. Five more
table matrices are partially mapped. The unknown-annotation N-only fixture
does not distinguish pointer-state selection, which remains under investigation.
Model Clippy passes; the failure manifest remains93.

Checkpoint94 adds seven failures grouped into three findings: empty Fields arrays
select all for submit/reset (B66); changed live charLimit leaves subsequent values
bounded by the old limit (B67); and unsupported annotations use R/D through actual
viewer pointer events despite the N fallback (B68). Nonempty/default/exclusion,
stored-limit, and known-Square controls distinguish the failures. B67 is qualified
by accepted product semantics; B68 uses a synthetic extension, not a published one.
Model/core Clippy pass. No production changes were made.

Checkpoint95 refreshes the eleven-crate run:7,272 tests,7,179passes, exactly93
expected failures and58skips. No extra or missing failure identities. The100-entry
manifest includes seven separately scoped GPU/engine/ignored identities. This run
verifies execution of the new tests and strengthened assertions through94.

Checkpoint96 adds three assertions for stale live Required/ReadOnly consumers (B69).
Stored flags and edit-permission controls pass; only metadata and Required reporting
remain stale. Strengthened B67 assertions also expose stale MaxLen descriptions.
Five new passing tests cover direct CID descriptors after EC3 #106 and narrow
shading BBoxes on CPU and three actual Vulkan coverage modes. Model/render-raster
Clippy passes. Broad95 predates these additions; current default-scope failures total96.

Checkpoint97 finds two CPU shading gaps: a coincident axial axis paints the whole
page (B70), and coincident zero-radius radial circles are refused (B71). Distinct
zero-radius centres correctly paint nothing. Positive geometry controls and all six
paired Vulkan assertions pass. Encoding CMap inheritance/override and independent
omitted crypt-filter defaults also pass. Final three-crate Clippy passes. The105-entry
manifest now includes98 default-scope failures; broad95 predates these additions.

Checkpoint98 corrects a Type3 test that claimed to omit a glyph but retained its
dictionary entry; the strengthened missing-key and two-glyph control pass. Unicode
attachment-stream preference and signed tiling steps pass. Existing tiling spacing
assertions now reject empty comparisons; all35 tiling tests pass. Model Clippy passes.
D62 qualifies ADR0914: a singular matrix still defines a transformation, so refusal
is a project policy rather than a consequence of the claimed mathematical premise.

Checkpoint99 refreshes the eleven-crate run:7,283 tests,7,185passes, exactly98
expected failures and58skips. No extra or missing failure identities. The105-entry
manifest includes seven separately scoped GPU/engine/ignored identities. This run
verifies execution of the new tests and strengthened assertions through98.

Checkpoint100 adds passing outline-style and Alpha soft-mask omission controls.
The first outline draft compared different rows and missed a deliberately cleared
style: row-position differences hid the defect. The revised same-position oracle
catches it; all temporary mutations are restored. D63 corrects an ADR source-table
citation while retaining its clock design. Final model/UI Clippy passes.

Checkpoint101 exposes Type1 tiling at10.400002 device-pixel spacing after a wide
BBox causes the selected matrix snap to be declined (B72). Ordinary Type1 and
Type2 controls pass. D64 reopens ADR1080’s unsupported exception to constant
spacing. A fresh erratum reading also narrows B24: FitR centering belongs to the
SHOULD recommendation in #536; FitB retains its unamended instruction. Earlier
claims of independently mandatory FitR centering are withdrawn. Model/core Clippy
passes. Manifest106 includes99 default-scope failures; broad99 predates B72.

Checkpoint102 adds five passing tests: standalone FreeText cloud width/dash, Link
border defaults, Type5 vertex padding, Type7 continuation records, and overlapping
patch order. The Type5 test also proves nonzero padding bits are ignored. A wrong-row
coordinate in the first overlap draft was rejected, corrected, and guarded with
single-patch controls. All eight selected tests and model Clippy pass. The failure
manifest is unchanged; 63 table matrices remain partial reviews.

Checkpoint103 adds passing checks for all48 Type5 coordinate/component width pairs
and the stream DL/Length distinction. Five existing FreeText callout tests now use
valid standalone annotations; all87 tests in that file pass. The shared filter fixture
now has a binary header marker and page resources; all14 filter tests pass. Model
and syntax Clippy pass. Function and annotation cell reviews retain existing
qualifications; 72 table matrices remain partial and the failure manifest is unchanged.

Checkpoint104 strengthens popup precedence: all four metadata entries now conflict
with their parent values, and the public query still returns the parent values. Three
consumer tests and core Clippy pass. Color and shape table reviews retain existing
B25 and D12 findings; older draft status is corrected. Ninety table matrices remain
partial reviews, with no new failure or test identity in this checkpoint.

Checkpoint105 corrects remaining standalone FreeText fixture roles while retaining
font resources and the original assertions. All 140 variable-text/rich-text tests and
model Clippy pass. Viewer preference and article cells add four partial matrices,
for 94 total; the existing OpenAction ordering failure remains open. No new failing
identity or production change is introduced.

Checkpoint106 adds passing direct-Info and valid hybrid-deletion controls. A prior
zero-width fixture is now explicitly classified as malformed-input tolerance. Binary
headers, deletion generations/free links and a 15-byte PDF2 identifier are corrected;
all 41 affected xref/Prev tests pass, as does syntax/model Clippy. The 99 table
matrices remain partial; no production behavior or failure count changes.

Checkpoint107 adds independent later-glyph assertions for TD leading and double-quote
spacing persistence. All seven positioning tests pass, including the two additions.
The source review corrects table boundaries and an ADR attribution; 103 selected
table matrices remain partial. The failure manifest remains unchanged.

Checkpoint 108 repairs collection and binary-image fixture structure. The same six
assertions still fail, with positive collection controls and two JPX tests passing.
Host/model Clippy passes. Three new table matrices and three supplements bring the selected partial
review to 106 tables; this does not certify every condition or consumer.

Checkpoint 109 repairs six further fixtures, including resources, binary headers,
initial identifiers and embedded-file descriptions. Six tests pass and four retain
the existing B32/B48/B54 failures, exactly matching their manifest identities.
Literal description assertions also catch a dangling-reference error rejected
during draft review. Final model/syntax Clippy passes.

Checkpoint 110 strengthens two tree consumers and adds two independent image Decode
tests; all four pass, with model Clippy clean. The source review corrects a stale
errata table number and the earlier image-table continuation omission. After merging
a duplicate Table 227 entry while retaining both reviews, the index has 110 unique
partial table matrices. No new failing identity is added.

Checkpoint 111 refreshes the eleven-crate run after the network interruption. Every
failing binary/test identity matches the manifest, with no extra or missing failures.
The run completed in 64 seconds with a peak of 7.02 GiB; its failing exit status is
expected for this audit branch. Font, filter and image-measurement reviews continue.

Checkpoint 112 adds B73: a GEO measure attached to an image is invisible to the
interactive measurement query. The matched page-viewport control returns the literal
expected position; the image case returns no answer. Core Clippy passes. This is a
supported measurement-feature gap, with no separate PtData display obligation inferred.
Broad111 predates these two new tests. The manifest now has 107 identities.

Checkpoint 113 adds passing literal Table 113 mapping and valid CCITT EOFB image
controls. The fax fixture distinguishes black and white source rows, including a
shorter Rows parameter. Font/model Clippy passes. Eight selected table matrices bring
the unique partial count to 118; the failure manifest remains 107.

Checkpoint 114 adds two passing public-consumer tests: exact decimal measurement
formatting and nondefault external-filter parameters after archive embedding. Clippy
passes. D66 corrects an audit error: issue 10 changes the external stream Length
obligation, not the already-mandatory embedded-byte-ignore rule. Seven more selected
matrices bring the unique partial count to 125; no new failing identity is added.

Checkpoint 115 adds a passing extraction and speech test for a named property
resource with an indirect ActualText value, with a literal inline control. Model
Clippy passes. Three signature transform tables and their actual assertion bodies
are reconciled; B18 remains observed and B42 remains interpretation-qualified.
The unique partial table count is 129; the 107-entry failure manifest is unchanged.

Checkpoint 116 refreshes the broad run with no extra or missing failure identities.
Three structure-attribute matrices bring the unique partial count to 132; D17 and
B53 keep their existing qualifications. Exact errata target checks confirm D66 and
reject a proposed issue216 mismatch after its second-page replacement is included.
The run completed in65 seconds with peak7.22 GiB.

Checkpoint 117 strengthens18 DCT assertions/fixtures and adds a passing explicit
Crypt omitted-Name/default test. All-target model/syntax/core Clippy passes. The
unique partial table count is134; navigation and FDF candidates are being reviewed.

Checkpoint 118 records B74: known embedded-parent navigation fails both across two
actions and within a single child/parent walk. Remote named destinations and the
attachment-index route pass. Core Clippy passes;137 unique table matrices remain
partial. The failure manifest now has109 identities, including these two.

Checkpoint 119 records B75: explicit FDF UTF-8 encoding fails for names, values
and choice options. Seven public tests produce four passing ASCII/BOM controls
and three failures, including live controls and saved content. Model Clippy
passes. The manifest has112 identities;140 table matrices remain partial.

Checkpoint 120 adds selected font-format and action-trigger table reviews, plus
a second primary errata-target sample. A rendered table resolves the shared
Korea1/KR row; no font behavior failure follows. D67 records obsolete whole-table
script-exclusion prose in the #618 disposition. No new execution is claimed.

The following paragraphs preserve checkpoint-specific results in chronological order.

At checkpoint 33, new failures concerned ignored minimum requirement versions (both reporting and
processing restrictions) and double compositing of overlapping portions of one
transparent stroke. New public measurement, join/fill geometry and new-file identifier
controls pass. Syntax/render-cpu/viewer-core all-targets Clippy passes. The previous
checkpoint records silent DPart truncation and false archive validation after an
incomplete transparency survey. Tests accept truthful bounded processing.

That review retains the conditional halftone and media handoff scopes, separates
viability from playback/version matching, and finds an obsolete partial-status rationale
in the implemented separation-simulation row. A filtered UTF-8 script dispatch control
passes. RichMedia Assets fixtures are now explicitly documented as partial consumer
probes. A preliminary claim that the current ledger called Configurations optional was
checked against the exact note and withdrawn; no such wording is present.

Checkpoint34 reproduces the same-stroke failure through a PDF and three hardware
lanes: direct Vello, raster GPU and raster Compute. Ordinary-line controls pass.
A sampled-CMYK-image overprint exclusion control passes. Model and both GPU
backend all-targets Clippy passes. The broad38 run predated those tests; refreshed broad53 includes the default-feature tests.

Checkpoint35 adds a passing glyph-clipping pixel control with an embedded font
and independently inspected outline. The existing FDF import test still passes
with its header corrected to the canonical form. Non-interactive forms share the
already-confirmed PrintField consumer gap; no duplicate failures are counted.
The custom-encryption-wrapper writer boundary is retained with explicit fixture limits.

Checkpoint36 adds a signed-fixture failure in usage-rights withdrawal after an
ungranted annotation edit. This is a gap in the adopted policy against a SHOULD
recommendation, not a mandatory ISO failure. A proposed paragraph-baseline
superscript test was rejected after distinguishing layout displacement from semantic
text position. Deprecated web/prepress review separates reader guidance, writer
version rules, and the product’s chosen metadata retention policy.

Checkpoint37 confirms that the signature-timestamp selector uses genTime despite a
verified keyCompromise on the TSA path. Genuine CMS/token/CRL artifacts and clean
and damaged-signature controls distinguish this from untrusted input. A separate
FieldMDP rename regression fails at the comparison API; the cross-revision identity
reading remains qualified, and its minimal fixture does not certify signature validity.

Checkpoint38 adds passing public-PDF controls for an empty Alpha soft-mask group
and its transfer function, including the outside-BBox value. All21soft-mask tests
pass. Coordinate foundations and group algebra retain explicit precision, shape,
backend, and arbitrary-stack residuals. The failure manifest remains65.

Checkpoint39 records a clean broad comparison:7,158run,7,096pass, exactly62
manifest failures within the eleven-package scope,58skipped. Three GPU failures
remain separately verified. Two subsequent lexical-context assertions pass. Scope
review retains the3D runtime exclusion while admitting normal appearances; stale
ledger/comment wording is recorded. Current-time CRL applicability remains a static
concern pending a valid public-path regression.

Checkpoint 40 adds an accessibility-order failure: an interleaved parent/child
structure whose logical text is ABC becomes ACB in the public AccessKit tree.
The flat control, model order, and fixture validity guards pass. Layout-only
attributes remain distinct from this observable reading-order obligation.

Checkpoint 41 confirms a public current-time CRL defect: a genuinely signed PDF
is reported locally valid using a CRL issued after the supplied clock. The matching
in-window control passes. Multimedia review corrects two errata summaries while
retaining the ratified playback exclusion.

Checkpoint 42 finds that Type 4 functions reject valid NUL whitespace. Five
other separator controls pass; the PDF and delegated PostScript grammars agree
on NUL. Filter/function framing is mapped without inventing heading requirements.

Checkpoint 43 adds a public archive-report failure: a257-component path becomes
256 components with a merged final component. The256 control passes; no path is
opened. Filtered attachment checksum and cross-Contents graphics-stack controls
pass. A token-boundary comment and several source locators are corrected in the
audit evidence; production and historical decision files remain unchanged.

Checkpoint 44 adds passing CPU controls for Type 3 image suppression and the
image-mask exception, including nested forms. Colour/font overview requirements
are mapped to their detailed owners; no aggregate status certifies every child.

Checkpoint 45 maps graphics, transparency and interactive overview clauses to
their detailed assertion owners. It adds no failure or execution claim. Source
locators and earlier draft statuses are qualified in each parent review.

Checkpoint 46 confirms historical OCSP applicability failure through the public
signature report. The later-response control passes; independent cryptographic
checks verify both fixtures. Rendering overview review also records stale aggregate
black-generation rationale against the current implementation and successor ADRs.

Checkpoint 47 reviews structure, interchange and linearization framing. It
qualifies shared-parser verifier evidence and archive tagging guards, preserves
confirmed child failures, and records stale interchange/prepress scope notes.
No new execution or runtime failure is claimed by this checkpoint.

Checkpoint48 confirms page-order reversal after a valid tree crosses the local
depth limit; the adjacent-depth control passes and recovery is disclosed.
File/security and text/colour framing reviews reconcile older draft statuses
with current results. No duplicate MissingWidth finding is counted.

Checkpoint49 completes a first current review of every ledger row. It records
the stale Screen/rendition scope rationale, qualifies signature aggregate evidence,
and adds passing actual script execution controls. Every row remains only partially
reviewed; feature residuals inside rows and confirmed failures remain open.

These are partial feature reviews, not whole-clause certification. The paragraphs
below preserve earlier checkpoint evidence. Weekly usage was 48% at the latest check;
the ceiling is85%, preserving the requested15% reserve.

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

The transfer/font checkpoint adds five passing controls: Type5 component overrides,
Type6 stream override, distinct RGB transfer slots, TR2 precedence, and an embedded
TrueType StandardEncoding fallback whose GID differs from WinAnsi. The first font
draft was rejected because ASCII alone could not discriminate the route. Marked-content
source/errata and existing ON/OFF/post-EMC assertions were reviewed. Synthetic ICC
intent fixtures discriminate transform selection but do not certify full profile
validity. Partial evidence reaches393rows, with490pending. No new behavior failure;
model all-targets Clippy passes. The broad38 run predated those tests; refreshed broad53 includes the default-feature tests.

The scan-conversion/AnnexO checkpoint strengthens the stroke-fit oracle from a
row minimum to minimum-and-maximum coverage across its interior. Both tests pass,
including the three Radeon890M Vulkan coverage lanes at eight placements. The
family ledger rationale is stale relative to10.7.4/.5 and A714; supported path
scope remains qualified. Encoded structure-ID and crop/rotation/UserUnit highlight
controls pass; the latter explicitly tests current origin policy. The existing
earliest-word fragment-search test is absent from its ledger references, not absent
from the test suite. Partial reviews reach403rows, with480pending. The manifest
remains36; viewer-core/render-raster all-targets Clippy passes.

The Type1/Type3/form-lock checkpoint adds a combined shading Matrix/BBox/Background
control with independent RGB interior values, corrects Type3 fixture extents and
labels named-Encoding tolerance accurately. A genuinely signed CAdES fixture now
proves Include lock selection and public edit refusal under enforcement, with
unlisted, unsigned and explicit reader-override controls. All61selected model tests
and the viewer edit test pass. All/Exclude/P still need genuine signed witnesses.
AntiAlias no-read remains an interpretation residual: an optional entry and no
fixed filter kernel do not establish permission to ignore true. Partial reviews
reach414rows, with469pending. Model/core all-targets Clippy passes; manifest remains36.

The security/structure-attribute checkpoint adds passing independent password-preparation
vectors and confirms a consumer gap: flattened-field Role, Checked and Desc changes
produce identical public accessibility answers. Three regressions fail; the title
control passes. The manifest now contains39failing identities. Perms conflict recovery,
ListNumbering classification and the Algorithm2.B256-round cap receive separate,
qualified decision findings. No high-round authentication failure is claimed.
Partial reviews reach436rows, with447pending; syntax/core all-targets Clippy passes.

The language/compositing/reference checkpoint corrects an oracle that expected empty
Lang to inherit: source says unknown. Two model regressions and two structure Alt/E
escape regressions fail; inheritance, explicit-language and resumption controls pass.
A reference label on page4097 is falsely reported absent despite successful numeric
selection; the test allows a truthful resource-limit report. The manifest reaches44.
Independent fractional-alpha Normal/Multiply/Color formulas pass after removing the
draft's white medium; that preliminary failure was a fixture error. Public reference
printing passes with supplied-target/proxy controls. Primary #330, #345 and #688 were
checked directly where the errata inventory was incomplete. Partial reviews reach460
rows, with423pending; model/core/render-cpu all-targets Clippy passes. The final
nine-crate refresh runs6,723tests:6,679pass, exactly44manifest failures and55skipped.
Exact binary/test identities match, with no additional or missing failures.


The filter/annotation/navigation checkpoint adds four passing public controls for
StateModel defaults, GoToDp leaf/interior navigation, Thread bead indices, and a
link-triggered transition event. The interior-node behavior remains an interpretation;
the event test does not certify visual sequencing. Parent source review corrected
DPart draft arrays against #612, Projection text against #42/#94, and the JPX
producer/support distinction against #29. Existing PrinterMark/TrapNet pixels,
Popup flags, Watermark placement, DCT precedence and named Crypt witnesses were
reviewed as assertions. A valid unsupported-ICC JPX fallback witness remains missing.
Partial reviews reach 477 rows, with 406 pending; the failure manifest remains 44.
Model/core all-targets Clippy passes. The broad wave27 run predates these four tests.

The output-intent/empty-text checkpoint raises the manifest to 49 assertions.
A valid page output condition without an embedded profile incorrectly borrows the
catalog's different press: both public pixels and the in-force report fail.
Three present-empty ActualText routes preserve a discretionary hyphen instead of
removing it; absent and explicit-hyphen controls distinguish empty from missing.
Balanced BX/EX pixels, current-shape DPart attachments, and a strengthened public
annotation click all pass. Effective #612 uses flat child references; Table409a is
held on appended primary page1015 despite a historical ADR's unavailable-table premise.
The current evidence records that correction without rewriting the historical ADR.
Partial reviews now reach 497 rows, with 386 pending. Final model/core all-targets
Clippy passes. The nine-crate wave27 run predates these changes.

The annex checkpoint reaches 509 partially reviewed rows, with 374 pending, and
50 expected failing assertions. The new failure is annotation CA counted as
transparency despite an explicit opaque appearance; archive validation consumes
that detector. Ruby role/order/text and nested legacy resource controls pass.
A state-only soft-mask probe also reports transparency, but its source interpretation
remains unresolved and is not in the failure manifest. Archive/accessibility
all-targets Clippy passes. The latest broad run remains wave27 and predates this batch.
Annex I's approved first-error reporting choice is retained; its ledger warning timing
is stale. Dynamic XFA is accepted and scheduled under ADR2876, not excluded. Annex L
review covers generated Document wrappers and updated matrix provenance, not all cells.

## Budget and continuation

The account's weekly allowance meter read 29% used at the start. The owner asks
to leave 15%; stop new dispatch near 81% used and reserve the interval through
85% for validation, evidence and handoff. Percentages are account quota readings,
not a known fixed token conversion. Preserve unfinished work explicitly rather
than reduce review quality.

No production behavior, ledger statuses, or historical audit records are changed
by this audit. A test-only module hook exposes no new production API. Tests and findings will be committed and pushed in reviewable batches.
