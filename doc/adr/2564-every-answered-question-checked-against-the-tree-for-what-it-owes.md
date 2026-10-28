# 2564 — Every answered question checked against the tree for what it owes

Status: accepted. Session 1865, slot 5 of batch 127, a reading round; no code. ADR 2565 is unused.
Context: `doc/questions/README.md` (the `Owes:` line), the 121 `A` files in `doc/questions/` at base
2d54febf, the answers A01, A02 and A23 that sat unbuilt for weeks (ADR 2552 built A23), ADR 0281 (a
record keeps its chronology). Questions asked: Q731, Q737, Q738. I worked in `scratchpad/r1865/`.

## 1. Method

Five fresh Opus subagents, one brief, about 25 answers each, read every `A` file whole — header and
body, since an `Owes: none` answer may command work in its body (A26, A27, A28, A31, A39 do) — and
looked for each owed item at the commit, through `git grep` and `git log` at 2d54febf alone, so that a
sibling's uncommitted edit counted for nothing. One row an answer: what it owes, `built`, `partly`,
`unbuilt`, `nothing` or `deferred` (the answer's own trigger not met), the shortest evidence, and for a
gap its owner (a slot of batch 127, a `doc/todo/` entry, an ADR's trigger, or nobody). I re-checked
every `partly` and `unbuilt` row against the answer's text and the tree, and five `built` rows (A01,
A02, A03, A130, A691: the `mozjpeg-rs` pin and its encoder, the confined transform worker and its
test, `serialize_linearized` and the Annex F rows, `check_url`, the two refusal names absent).

**Plant.** The same brief, given A23 at 0c34fee4 (before round 1859) and A714 at 2d54febf, answered
`unbuilt` (the clause's Y still taken, `CONTRADICTED_LUMINOSITY_OF_A_CIE_BASED_MASK` held, no ADR naming
A23) and `built` (`raster/grid_fit.rs`, §10.7.5 `implemented` under ADR 2553): both right.

## 2. What the audit found

Of 121 answers: 68 built, 25 owe nothing, 5 deferred on their own trigger, 15 partly built, 8 unbuilt.
The brief's hypothesis — a handful unbuilt, one or two unowned — did not hold: twelve answers carry
work nobody owns. Three answers are ambiguous about what they owe and are asked: A662's hardware lane
after its first item was taken back (Q731), A47's download shape and scope against A569 (Q737), A35's
host option "if it is wanted at all" (Q738).

**Owed and unowned, named to the orchestrator as batch-128 slots** (sizes in tool calls):

| answer | the work | lane | size |
|---|---|---|---|
| A31, A39 | one §7.3.3-tolerant integer accessor wherever a table types an integer, `/Rotate` included, warning where easy | robustness | 150, two slots |
| A706 items 1, 5 | `derive:<tool>:append` as a step of a `try` list (`config/steps.rs` passes the step over; the remedy itself is built, `archive/derived.rs` `import_pages`, ADR 2539) | archive | 30 |
| A694 item 3 | R03a for a CFF or Type 1 program — a charstring writer for the `.notdef` copy | archive | 150 |
| A288 | the `doc/third-party-data.md` row for the held Adobe Supplement EL3 | docs | 10 |
| A27 | Q27's four counted cost properties (the census count, the `Document::open` counter, the font-cache count in place of the 40 pages/s floor, the standing default) | gates | 100 |
| A28 | the distinct-stream cost counter, writes gated off by an environment variable | gates | 60 |
| A11 | the vacuum mode: a new vacuumed file, in `pdf:/` only by copying it out | transform, faces | 80 |
| A08 | an older version (an append) copied out of the filesystem, as undo | faces | 100 |
| A09 | integer keys of stride 100 from 00100 for page renames, refusals by name | faces | 120 |
| A54 | the executor's loop in the KIO worker and the FUSE filesystem, through `pdf-vfs` | faces | 100 |
| A254 item 4 | Route A, the PDF handed to CUPS through the transform suite as a named option | print | 100 |

Item 2 of A254 (the winit print panel, `ipp`, the confined spool writer, the docked preview, tiling)
is owned only by `doc/todo/65`'s bucket 1 ("RFC 0004's print path") and is several slots. Every lane
past `archive` and `robustness` waits for the owner's word of 2026-10-27 to be lifted.

**Owed in `CLAUDE.md`, the owner's file** (named in the report, never edited): A26 takes Q26's repair,
"No port changes the renderer's system-call set", where principle 3 still says the set "does not
change" (A706 item 2's sentence is in the first authoring-line amendment, added by round 1851).
**The owner's own acts**: A227's DC-overflow
report upstream (`doc/patches/` says "for the owner to file"); A635's tool-list comparison is the
orchestrator's.

## 3. The table (at 2d54febf, as the subagents wrote it; section 2 is my reading of its gaps)

| A | owes | built? | evidence | owner |
|---|---|---|---|---|
| A01 | mozjpeg-rs DCT encoder; re-encode only when a format is explicitly asked | built | Cargo.toml:354 `mozjpeg-rs =0.9.2`; crates/pdf-sandbox/src/encode.rs:96; doc/stack.md:20; ADRs 2138, 2139; commit 127b5dd3 | - |
| A02 | pdf-transform split into broker + confined worker now | built | crates/pdf-transform/src/confined.rs, src/bin/quorra-transform-worker.rs, tests/confined.rs; ADR 2140; commit 127b5dd3 | - |
| A03 | Annex F output behind `optimize --linearize`; qpdf as input; CLAUDE.md exclusion sentence gone | built | crates/pdf-syntax/src/linearize.rs; quorra-transform.rs:1747; ledger F.1–F.3.2 `implemented` (ledger.toml:6650+); CLAUDE.md lists Annex F in scope, "stays excluded" sentence absent | - |
| A04 | nothing (confirms the shipped restriction policy) | nothing | - | - |
| A05 | binaries quorra*/quorrafs, raster/ crates, C prefix quorra_/quorra_vfs_ | built | crates/viewer-ui/src/bin/quorra.rs; pdf-fuse/Cargo.toml:11 `quorrafs`; tools/pdf-retrieve/Cargo.toml:17; raster/crates/{raster,raster-scene,raster-gpu,raster-pages}; crates/viewer-ffi/include/quorra.h, pdf-vfs-ffi/include/quorra_vfs.h; no `pdfv_` left | - |
| A06 | nothing (confirms deterministic default) | nothing | - | - |
| A07 | nothing (confirms the `pdf:/` scheme) | nothing | scheme in crates/pdf-vfs-ffi/include/quorra_vfs.h | - |
| A08 | a route copying an older version (an append) out of the filesystem, as undo | unbuilt | pdf-vfs Generator enum (layout.rs:67-112) has no revisions/versions directory | nobody |
| A09 | integer keys stride 100 from 00100, gap insert, explicit renormalise, rename refusals by name | unbuilt | crates/pdf-vfs/src/writing.rs:515 `rename` always refuses `ReorderIsAmbiguous` | nobody (doc/todo/58:431 still calls it an open question) |
| A10 | nothing (confirms meta/info.json writes) | nothing | - | - |
| A11 | vacuum mode: a new vacuumed file, in `pdf:/` only by copying out | partly | no `vacuum` anywhere; only optimize.rs:21 reachability pass (new file, unreachable objects dropped) does the work unnamed; no pdf:/ copy-out route | nobody |
| A12 | nothing (page-label aliases left out) | nothing | - | - |
| A13 | nothing (keeps 150/300 dpi as shipped) | nothing | crates/pdf-vfs/src/lib.rs:130 `RESOLUTIONS = &[150, 300]` | - |
| A14 | nothing (ratifies per-page images as built) | nothing | crates/pdf-vfs/src/layout.rs:21 | - |
| A15 | nothing (superseded by Q46) | nothing | - | - |
| A16 | nothing (parts bought, placed in doc/pdfa; extraction permitted, not commanded) | nothing | - | - |
| A17 | nothing (superseded by Q46) | nothing | - | - |
| A18 | ship sRGB with override flag; report output intent reinterpreting existing marks | built | quorra-transform.rs:266 `--output-intent-profile`/`--profile`; archive/report.rs:803 `profile`, :1411 `Decision::Stated{reinterprets}` printed, :1436 profile line | - |
| A19 | nothing (void) | nothing | - | - |
| A20 | nothing (confirms not-checked-by-name as built) | nothing | - | - |
| A21 | report every constructed appearance (per appearance) | built | archive/report.rs:871 `appearances`, :1812-1826 one line per annotation and page; archive/mod.rs:1449 | - |
| A22 | verbs named `quorra-transform archive`, `quorra-retrieve archive-check` | built | quorra-transform.rs:993 `"archive"`; tools/pdf-retrieve/src/main.rs:126 `"archive-check"` | - |
| A23 | ADR for switching §11.5.3 luminosity to the common behaviour, and the switch | built | ADR 2552; ledger 11.5.3 `departed` (ledger.toml:3255); pdf-colour/src/colour/press.rs:273, to_rgb.rs:109 | - |
| A24 | the proposed principle 3 wording into CLAUDE.md | built | CLAUDE.md:107 "It may be *given* things…" | - |
| A25 | CLAUDE.md principle 2 adapted (gates, stack) and startup prohibitions softened | built | CLAUDE.md:46 "Perf gates run at every merge", :58 "no full page-tree walk unless page one needs it", :99 gates with numbers in doc/checks/launch-path.toml | - |
| A26 | (1) allow-list moves only by owner's word, in todo/61; (2) Q26's repair of CLAUDE.md's "system-call set does not change" | partly | (1) doc/todo/61-what-a-library-asks-the-machine.md:5; (2) absent: CLAUDE.md:110 still reads "The renderer's system-call set does not change" | nobody |
| A27 | Q27 recs: accessibility_census count floor; `Document::open` counter; transform gate's pages/s floor → font-cache count; standing walk obligation | unbuilt | crates/pdf-transform/tests/gate.rs:59 `PAGES_PER_SECOND_FLOOR = 40.0` still; no open counter in pdf-syntax; ADR 0899 §4 table lists all as left | nobody |
| A28 | distinct-stream cost counter in pdf-syntax's decoded cache, writes gated off by an environment variable | unbuilt | crates/pdf-syntax/src/document/cache.rs has no such counter; A28 used only as env-var house style (crates/viewer-ui/tests/launch_path.rs:201) | nobody |
| A29 | options 1+2: clock figures under an opt-in quiet run, busy-machine decline | built | crates/viewer-ui/tests/launch_path.rs:202 `PDFVIEWER_LAUNCH_CLOCKS`, :408 run_delay decline; doc/verify.md:526; ADRs 0916, 0917 | - |
| A31 | one §7.3.3 tolerant integer accessor applied wherever a table types an integer, warning where easy | unbuilt | crates/pdf-model/src/integer_entry.rs:52-57 answers /Width,/Height only and still calls Q31 "open"; Object has only as_integer (crates/pdf-syntax/src/object.rs:302) | nobody |
| A32 | take floor off whole-process peak_mib; keep a two-sided claim on the program's own memory | built | peak_mib gone, `peak_anon_mib` banded both sides (doc/checks/launch-path.toml:91, :442); delivered via Q37/A37's shape | - |
| A33 | Movie /Poster stream drawn as image XObject in /Rect; boolean form refused and reported | built | crates/pdf-model/src/appearance.rs:1875 `movie_poster`; ADR 1561 | - |
| A35 | keep refusal for a null /Font as default; host-supplied option to substitute a face, still reported | partly | refusal kept: crates/pdf-model/src/content/font.rs:614; no host option to guess found | nobody |
| A37 | gate bands the anonymous high-water; whole-process figure printed, unbanded | built | doc/checks/launch-path.toml:91; crates/viewer-ui/tests/launch_path.rs:3211 "MiB resident of which … allocated"; ADR 0910 | - |
| A39 | A31's accessor covers /Rotate too; no stricter visible-change test | unbuilt | crates/pdf-model/src/page.rs:229 and crates/pdf-syntax/src/document.rs:584 read /Rotate by `as_integer` (a real → None) | nobody |
| A45 | panel row for an external associated file, decided for the whole panel; note kept meanwhile | deferred | trigger: the attachments panel's design pass; note kept at crates/viewer-core/src/notes.rs:234; panel lists embedded files only (viewer/answers.rs:409) | - |
| A46 | PDF/A on; RFC 0006 ratified; validator over parts 2+4; finish and certify PDF/A-4 first | partly | RFC 0006 ratified (doc/rfc/0006:3), validator crates/pdf-archive, ADR 0926; 26 `Check::Unchecked` rows in crates/pdf-archive/src/table and doc/pdfa-blocking-issues.md remain | archive lane, slots 1, 2, 4, 7 of batch 127 |
| A47 | ship wide-coverage OFL family incl. CJK, licence discipline, refuse where uncovered; two downloads (default full) | partly | CJK pack (doc/stack.md:21, doc/third-party-data.md:28, ADRs 2302/2303); refuse: crates/pdf-transform/src/archive/fonts.rs:729; no non-CJK universal family in data/ | nobody |
| A48 | DeviceN /DefaultCMYK and empty glyph for .notdef: allowed, reported, in xmpMM:History naming the clause | built | crates/pdf-transform/src/archive/prepare.rs:76 `DEFAULT_CMYK_PARAMETERS`, :6275 empty glyphs in history; ADR 2458 | - |
| A49 | nothing (void) | nothing | Status: void | - |
| A50 | nothing (a narrow permission: F→f only where a deprecation rule requires it) | nothing | doc/pdf-a-conversion-limits.md:1115 records it; crates/pdf-transform/src/archive/decision.rs:3526 | - |
| A51 | nothing (part 2 not bought; row's fate is A169's) | nothing | answer body; doc/T-REC-T.801-200208.pdf held | - |
| A52 | apply TechNote 0010 A021: part 2 §6.6.6 outside PDF/A-2 validation; part 4 §6.7.5 stands | built | crates/pdf-archive/src/clarification.rs:88 (A021, 6.6.6) | - |
| A53 | widen `Lexical::Date` to ISO/WD 8601-1 forms, citing the WD; keep GPSCoordinate check | built | crates/pdf-archive/src/iso_8601.rs:6; table/metadata.rs:1305, :1313; ADR 1013 | - |
| A54 | executor loop in the KIO worker and FUSE filesystem via pdf-vfs (CLI loop and executor built) | partly | built: crates/pdf-transform/src/executor.rs:121, bin/quorra-transform/here.rs:73, viewer-host/src/archiving.rs:591; none in crates/pdf-vfs or crates/pdf-fuse | nobody |
| A55 | derive mode: never default, needs site+tool, reported "this is derived, not original", in xmpMM:History | built | crates/pdf-transform/src/archive/remedies.rs:24-35, :61 `DERIVED_NOT_ORIGINAL`; config `DeriveWithoutTool` | - |
| A56 | no sandbox offer in v1; warning at the tool configuration site | built | crates/pdf-transform/src/archive/config.rs:3243; bin/quorra-transform.rs:2738; doc/profiles/derive-attachments.toml:71 | - |
| A57 | on-failure takes one alternative, default stop; chain only on evidence | built | crates/pdf-transform/src/archive/config.rs:204-206 refuses a list; ordered `try` added later by A706 (config.rs:2826) | - |
| A58 | one amendment: appended pages + Level A structure entries for them | built | CLAUDE.md near-side amendment "A page composed solely of content the document already holds"; doc/scope-ratifications.md:104; crates/pdf-transform/src/archive/tagged.rs:23 | - |
| A59 | departure omits PDF/A identification by default; separate switch keeps the claim | built | crates/pdf-transform/src/archive/mod.rs:333-340 `claim_conformance`; mod.rs:535 | - |
| A60 | nothing (don't buy 19005-3; keep part 3 cheap to adopt later) | nothing | - | - |
| A61 | cross-check test, scoped to what the survey walks | built | crates/pdf-archive/tests/cross_check.rs; ADR 1055 | - |
| A62 | nothing (void) | nothing | - | - |
| A63 | `departed` status: enum, aggregate rule, state.sh count, ledger header, rows | built | tools/conformance/src/bin/ledger.rs:60; counts.rs:128; tools/state.sh:139; ledger.toml:17; 9 rows `departed` | - |
| A64 | redaction application, written as new file; row reason; region choice documented | built | crates/pdf-transform/src/redact.rs:6,13,238; ledger §12.5.6.23 `departed` | - |
| A65 | CLAUDE.md relocation amendment; two named refusals (unbalanced q/Q, overlap) | built | CLAUDE.md:251; pdf-transform/src/archive/prepare.rs:5771-5776 (`relocate_marks` fallbacks); ADR 1123 | - |
| A66 | shared crypto crate + §7.6.5 build, only on a real trigger | deferred | trigger: a document encrypted to a key a real reader holds, or a host supplying a private key; ledger §7.6.5 `reported` | - |
| A67 | nothing (keep `ask`; already shipped) | nothing | crates/viewer-host/src/policy.rs:1307 `#[default] Ask` on Links; ADR 1155 | - |
| A72 | four transition styles and /TP 2-5 at stated numbers, reported as ours | built | crates/viewer-core/src/transition.rs:418-481,652; pdf-model/src/appearance.rs:3509; ADR 1299 | - |
| A76 | 17th mode in raster/ (DestOver + per-channel), QUORRA_FEEDBACK §49 closed | built | raster/crates/raster-gpu/src/compose/draw.rs:288; raster-scene/src/blend.rs:271; ADR 1295; QUORRA_FEEDBACK.md §49 "Closed by ADR 1295" | - |
| A97 | XFDF spec held + extracted, `<annots>` read implemented | built | crates/pdf-model/src/xfdf/annotations.rs:1; doc/third-party-data.md:663; ledger §12.7.8.3.4/§12.7.6.2 implemented | - |
| A98 | ureq client in viewer-host behind four levels, ask default, scheme gate, stack/deny | built | viewer-host/Cargo.toml:38; policy.rs:708 `may_submit`; submit.rs:314; doc/stack.md:25; deny.toml:159 | - |
| A100 | §8.6.6.5 `implemented`, note with --separations default as precedent | built | ledger §8.6.6.5 `implemented`, note cites `--separations` and A100 | - |
| A121 | nothing (No to early present) | nothing | - | - |
| A130 | CLAUDE.md TLS exception; hostname length/charset check before TLS | built | CLAUDE.md:120-123; viewer-host/src/submit.rs:82-91,144-150 (`check_url`); ADR 1327 | - |
| A131 | hint tables pad each item run to a byte; F.4.1 row + ADR 1293 | built | crates/pdf-syntax/src/linearize.rs:96,3222; ledger F.4.1 note cites A131; ADR 1293 | - |
| A168 | §7.6.5 note and A66 corrected: four witnesses decryptable; build untriggered | built | ledger §7.6.5 note "witness set, ruled by A168"; A66 appended paragraph; crates/pdf-syntax/tests/public_key_witnesses.rs | - |
| A169 | T.801 held/entered, M.9.2 JPX-baseline rule implemented, row reason amended | built | doc/third-party-data.md:287; pdf-archive/src/table/graphics.rs:4395; survey.rs:1858 | - |
| A170 | brainpoolP512r1 + Ed448 private modules, RFC vectors, swap condition | built | crates/pdf-signature/src/brainpool_p512.rs:10; ed448.rs:10; ed448/vectors.rs; ADR 1386 item 3 | - |
| A171 | geospatial census first, WKT inverse methods per GN 7-2, accuracy budget | built | crates/pdf-model/examples/geospatial_census.rs; src/geospatial/mod.rs, datum.rs; ADRs 1586, 1587 | - |
| A192 | "reviewed = stable + trusted supplier" written beside swap condition; trigger | built | doc/todo/65-the-remaining-frontier.md:119-125 (definition, judged packages, trigger); doc/stack.md:113-117 | - |
| A193 | RFC 0008 Tier 0 + Tier 1, Scripts level off, pinned ref, scope rule | built | pdf-model/src/aform/; crates/pdf-script/src/lib.rs:48 (`ab3b42a7`); pdf-script-worker; policy.rs:2063-2071,2390; client.rs:139; ADR 1689 | - |
| A209 | nothing (T.88 data stays outside the repo) | nothing | - | - |
| A227 | zune-jpeg fork + both patches, regression tests, both reported upstream | partly | Cargo.toml:329-341; doc/patches/zune-jpeg-*.patch; crates/pdf-model/tests/banded_decodes.rs:226,258; ADRs 1589, 1730 | the owner (doc/patches/zune-jpeg-dc-prediction-overflow.md:3 "for the owner to file") |
| A254 | RFC 0009 §12 items 1-4 (items 5-6 on demand/never) | partly | built: §13.4 poster (ADR 1561), XFA sentence (ADR 1932), §14.11.5, media + viability + hand-off (ADRs 1946,1960,1961,2382), guidelines (ADR 2440) | print items: doc/todo/65:41 (bucket 1, "RFC 0004's print path"); Media level: ADR 1961 §1 (owed the day a window starts a player); Route A: nobody |
| A270 | `VK_LOADER_DRIVERS_DISABLE=*lvp*` launcher line, GPU-less fallback measured first | deferred | trigger: the tree ships a launcher (none at REV: no `.desktop`/launcher file); ADR 1585 | - |
| A271 | `/GPTS` degree-shaped projected registration read via base system; `departed` row; ADR | built | crates/pdf-model/src/geospatial/mod.rs:373,532; ledger §12.10.2 `departed` (ledger.toml:4730); ADR 1918 | - |
| A286 | CLAUDE.md script exclusion amended; §12.6.4.17 `implemented`, `off` default, A193 condition in note | built | CLAUDE.md:236; ledger.toml:4086-4091 (`implemented`, A100/A193/A286 in note) | - |
| A287 | ADR superseding 0001; .NET host on viewer-core boundary; GPU path in gtk, qt, .NET host | partly | ADR 2048 (supersedes 0001); hosts/windows/Quorra.App/; ADRs 2052, 2070 (gtk/qt device). Missing: .NET host's device path (SwapChainPanel) | RFC 0010 step N3c (doc/rfc/0010:180); no slot of batch 1861 |
| A288 | `doc/third-party-data.md` row for the held Adobe Supplement EL3 | unbuilt | no `Supplement`/`EL3`/`A288` hit in doc/third-party-data.md (only XFDF line 664 mentions EL3) | nobody |
| A308 | Core 14 AFMs held + NOTICE + PROVENANCE; pair table; TrueType/OpenType kern; rich text applies | built | data/standard-fonts/afm/; NOTICE:75; PROVENANCE.md:62; crates/pdf-font/src/pairs.rs:480,745; ledger §12.7.4.3 `implemented` | - |
| A348 | PIMA 7667 held + third-party row; EnumCS 20/24 drawn; §7.4.9 note narrowed to CIEJab | built | doc/third-party-data.md:316; crates/pdf-colour/src/colour/extended_srgb.rs; ledger §7.4.9 note (ledger.toml:~383) | - |
| A393 | CLAUDE.md principle 2 admits page one on the faster lane; ADR 1831 ratified | built | CLAUDE.md:90-91; ADR 1831:3-4 (`ratified` and `built`) | - |
| A430 | nothing (close built by ADR 1945; GLib issue 4079 filed) | nothing | crates/pdf-sandbox/src/bin/pdf-sandbox-worker.rs:50; tests/inherited_descriptors.rs:76 | - |
| A436 | Helmert datum transformation from TOWGS84; stated refusal otherwise; no registry | built | crates/pdf-model/src/geospatial/datum.rs:79 (`Helmert`), :264 (`Refusal::DatumTransformation`) | - |
| A448 | nothing (keep as-is) | nothing | - | - |
| A473 | line lane's endpoints in page space mapped by `line.wgsl` view uniform; own ADR + instrument | built | raster/crates/raster-gpu/src/shaders/line.wgsl:11-40; doc/todo/75 §2 "Built" (ADRs 2022, 2036, 2056); `zoom_frame` `ZOOM_FRAME_DUMP` | - |
| A477 | priced per-page route in confined window, crossing counted, lane on the trace | built | crates/viewer-confined/src/lib.rs:470 (`Route`), :913; crates/viewer-ui/src/bin/quorra-confined/pricing.rs:46; ADR 2017 | - |
| A478 | nothing (ratifies ADR 1989) | nothing | ADR 1989; doc/state-of-play.md:341 | - |
| A501 | nothing (WinUI 3 chosen; rides A287) | nothing | hosts/windows/Quorra.App/*.xaml | - |
| A514 | manifests on close2/hayro@7e39af41, deny allow-git, patches deleted, refusal out, §7.4.7 implemented, digests | built | Cargo.toml:387,401,589; deny.toml:188; no hayro patch file; ledger §7.4.7 `implemented` (:365); ADR 2102:67 (six arms by digest); aa74b695 | - |
| A538 | cheaper exact folding-stroke measure under a fifth, routed, sixteenths out, §10.7.4 says so | built | delivered via A559: ADR 2252 ("Routed", §10.7.4 note), ADR 2253; folding.rs:1082 | - |
| A550 | Foxit notice in program's XMP `UsageTerms`; restated Liberation renamed | built | crates/pdf-transform/src/archive/fonts/notices.rs:22,132; archive/fonts.rs:1404 (`renamed_modified`) | - |
| A559 | `FOLD_FLATNESS` = 1/20, gate as flattened set, one boundary per mark shared by strips | built | crates/render-cpu/src/stroke/folding.rs:1082; ADR 2252 §2 (gate); ADR 2253 (no strip finds boundary twice) | - |
| A562 | launched-documents level defaults to `ask` once every window has its menu | built | crates/viewer-host/src/policy/launched.rs:71-72 (`#[default] Ask`, ADR 2185) | - |
| A566 | `substitute-stretch` loss; one decision carries every loss; drawn pairs with pixel diffs | built | crates/pdf-transform/src/archive/decision.rs:550; ADR 2262 ("The eight, drawn" §, levels stated) | - |
| A569 | CJK pack as second download; composite CID route; notice; third-party row; stack.md | built | archive/fonts/pack.rs (`CjkPack`); ADRs 2302, 2303, 2386; doc/third-party-data.md:28; doc/stack.md:21 | - |
| A570 | re-keyed Adobe-Identity text (two losses, drawn pairs); `text-as-image` page last in order | built | ADR 2378 (`archive::fonts::rekeyed`); ADR 2394; decision.rs:555; doc/history/1772:24 (drawn pairs) | - |
| A585 | nothing (ratifies standing behaviour) | nothing | - | - |
| A595 | `NODEVICE_SELECT=1` launcher line, guarded by DRI_PRIME/MESA_VK_DEVICE_SELECT, two-GPU trial | deferred | trigger: the tree ships a launcher (none at REV); ADR 2299 | - |
| A596 | CJK pack built by CI as a release asset with SHA256SUMS; test checks built pack against pack.rs digests | built | .github/workflows/ci.yml:980 (`cjk-pack` job), :1018 test step; pack.rs:939; doc/cjk-pack.md:63 PACK_VERSION; ADR 2386 | - |
| A631 | movie, sound-/F and rich-media assets handed over; RIFF wrapper for embedded samples | built | viewer-core/src/interact/deprecated_media.rs:20; pdf-model/src/media.rs:126 (`wave`, ADR 2398); attachment/rich_media.rs:42; ledger 13.3, 13.4, 13.7.2.1 implemented; ADR 2382 | - |
| A632 | RFC auditing Tier C decoders and sizing surface; Tier D trigger restated | built | doc/rfc/0012-…; doc/scope-ratifications.md:31 ("fourth edition whole"); asked again as Q649 | - |
| A633 | XFA 3.3 into doc/md, RFC splitting static/dynamic, then one XFA round a batch | built | doc/md/XFA-3_3.md (gitignored, on disk); RFC 0011; ADR 2380; ADRs 2414/2450/2474/2475; batch 127 brief:214 slot 6 | - |
| A634 | Table 380 decoration, baseline shift (sub/sup), ruby into accessibility node | built | viewer-core/src/accessibility.rs:620, :638; viewer-accessibility/src/role.rs:261; ADR 2384; ledger 14.8.5.4.4 implemented | - |
| A635 | orchestrator cleared after merge+handover; round-type allowlist measured then adopted | partly | clear rule: ADR 2372 §3, ADR 2462 §2; allowlist unmeasured per ADR 2479:43 | batch 127 brief:86 (launched as `round`); comparison: nobody |
| A636 | sign zip executables after a person ran WinUI window and `window` job passed | deferred | trigger unmet: WinUI window run on a Windows desktop; unsigned-zip note ci.yml:946; ADR 2374 | - |
| A639 | CLAUDE.md XFA entry: static half in scope, dynamic excluded pending measurement | built | CLAUDE.md:235; dynamic amendment deferred (static half in + `/NeedsRendering` page-one priced) | - |
| A649 | records only: Tier C trigger restated, two decoders and five-step order | built | ADR 2428; doc/scope-ratifications.md:28; doc/stack.md:42-47; RFC 0012:34 | - |
| A657 | stand_in holds old picture one refresh; zoom_stand_in adds a lands-inside document | built | viewer-host/src/stand_in.rs:212 (`Hold`), tests :1036, :1070; tools/drive-windows.sh:3583, :3652; ADR 2510 | - |
| A662 | (1) resident image texture; (2) hardware lane for moving view | partly | (1) built, measured, taken back: ADR 2512 (rejected, no code remains); (2) no code | nobody |
| A668 | end §7.4.8 departure; keep §12.6.4.6; JPX writer for >16-bit redaction | partly | (1) ADR 2468, ledger 7.4.8 implemented; (2) 12.6.4.6 departed; (3) pdf-jpx-write 17–24 bit, ADRs 2470/2480/2481 | doc/todo/65-the-remaining-frontier.md:217 (recorded, unslotted) |
| A680 | ratification detail moved to own doc named in CLAUDE.md table; default effort | built | CLAUDE.md:18 → doc/scope-ratifications.md; CLAUDE.md:232 | - |
| A681 | blocking-issues doc + reopen blocks; slot policy (+A657/A662/ADR 2365 fill); ci/batch push | partly | doc/pdfa-blocking-issues.md; ADR 2490; batch 127 brief:4-5, :38; A657 built | nobody (A662 lane, ADR 2365 levers) |
| A687 | lift no-widths refusal behind `layout-from-this-viewer`; supply /BaseFont | built | archive/fonts/described.rs:644; tests/archive/layout_from_this_viewer.rs:115; doc/profiles/as-if-printed.toml:315, as-shown.toml:353; ADR 2504 | - |
| A691 | remove two deep-JPX refusals; archive keeps colour-channel wording; veraPDF issue | built | refusal names absent from crates; ADR 2521; doc/pdfa-blocking-issues/verapdf-jpeg2000-issue.md; pdf-archive/src/coverage.rs:2450 | - |
| A694 | photo gate R08/R03; R05 a+b; "printed and scanned" note; R11; R03a | partly | ADRs 2516, 2517, 2518, 2520, 2543; as-if-printed.toml:24; R05(b) ADR 2519 deferred | slot 1 of batch 127 (R03a CIDFontType2); ADR 2519 / doc/todo/72:380 (ICC writer) |
| A706 | try lists; append-statement + sentence; info/presentation; two profiles; derive attach/append + --config; neutral tool name | partly | no-data-loss.toml:68; state-every-loss.toml; ADRs 2536, 2538, 2539; CLAUDE.md:250's sentence; archive/derived.rs `import_pages` (derive append); no `office-to-pdf` in tree | slot 4 of batch 127 (append-statement, graphics); derive append as a `try` step (steps.rs:207): nobody |
| A707 | photographed page of tagged doc prunes its leaves under `tagging-of-photographed-pages` | built | archive/decision.rs:557; fonts/photographed.rs:926; tests/archive/photographed.rs:772, :856; as-if-printed.toml:323 | - |
| A713 | read ETSI TS 119 172 / TR 102 272; enforce §12.8.3.4.4 policy constraints | unbuilt | ledger 12.8.3.4.4 `partial`; no policy parser or signer-rule code in pdf-signature/src; ETSI PDFs placed in doc/ (gitignored) | slot 3 of batch 127 (session 1863) |
| A714 | /SA true grid fit in all backends, on by default, §10.7.5 implemented | built | ADR 2553; raster-gpu/src/raster/grid_fit.rs; render-raster/tests/stroke_adjustment.rs; ledger.toml:2985 10.7.5 implemented | - |
