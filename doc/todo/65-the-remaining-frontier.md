# The remaining frontier — what is left, grouped by why it is not done

Status: **standing** — a map for steering the campaign, not a plan for one round. It answers one
question the ledger alone does not: of the rows that are `partial` or `reported`, *why* is each not
finished, so that the ones a normal round can take are legible apart from the ones that wait on
something a round cannot supply.

**It carries no tally.** `tools/state.sh ledger` prints how many rows hold each status, and
`grep 'status = "partial"'` / `'status = "reported"'` over `doc/conformance/ledger.toml` is the live
membership; the categories and the reasons below are the content, and each row's own note is where
the reason is argued in full. This is a sweep like `doc/todo/01`'s: membership is re-derived by
reading the notes, and a row leaves a bucket the moment its note's residue changes. What does **not**
decay is the shape — the six reasons a requirement stays open, plus two structural findings the six
do not cover.

**The population is the ledger's `partial` and `reported` rows, and only those.** A `departed` row
is a decision already taken and priced — `doc/HANDOVER.md` says what the word means — so it is not
open work and is not mapped here, even where the departure itself waits on something; what such a
row waits on is in its own note. Three of them are worth knowing about beside buckets 2 and 3 and
are named in those buckets' prose rather than as bullets. ADR 1237 is the rule and its cost.

**Read it against `CLAUDE.md`'s two denominators.** Coverage is the specification and the ledger is
its instrument; this map is a reading of that instrument. A bucket that says *blocked* is a claim
about coverage, and — principle 5 — a claim about the specification decays, so the not-owed bucket
below is the one a revisiting round re-reads first.

## The six reasons, and where measurement placed each row

The membership is taken by reading every `partial` and `reported` note once (trap 8: the reason is
the note's, never the corpus's). Two shapes fall outside the six and are named after them, because
forcing them into a bucket would mislead: **aggregate** rows, which state no debt of their own and
move only when a row below them does, and **not-owed** rows, whose residue is a documented choice,
an exclusion, a deprecation, or a case the standard leaves undefined.

### 1. Host-UI — a surface now in scope to build

`CLAUDE.md` principle 3 gives a document's restrictions four levels (`off`/`on`/ask/warn) whose
interface is in scope to build; the same surface covers printing and a collection's alternate
presentations. These rows are not a gap in the reading — each one's core is already in place,
waiting only on the surface and the operation it drives. **What builds them:** the host work of
`doc/todo/30`–`38` and RFC 0004's print path. Table 147's print half is no longer among them: it is
read, answered and applied entry by entry, and §12.2 is `departed` for `/HideMenubar` alone
(ADRs 1203, 1204, 1227).

- §12.3.5 — Table 153's `/Colors` and `/Split`, and §12.3.6's `FilmStrip`, `FreeForm` and `Linear`:
  surfaces one panel does not offer as alternatives, each named by `unsupported_presentation` rather
  than dropped in silence. `/View` is obeyed for every one of its four values, tile mode included,
  and `/Sort` orders the panel (ADRs 1168, 1215).
- §12.7.5.3 — a file-select control's dialogue. The control's *contents* already cross: a person's
  chosen pathname is read by `viewer_host::policy` and submitted as the field's value (ADR 1216), so
  what is left is a `FileChooserNative` or a `QFileDialog` over it, a convenience for a round that
  can drive a dialogue. The row's second residue is bit 26's `RichText` *formatting*, which
  §12.7.4.3 hands to XFA and which is therefore reported rather than drawn — the plain `/V` is laid
  out (ADRs 1070, 1122).
- §12.7.8.3.3 — an FDF template page whose Table 253 `/F` puts it in another file. A host question
  since `viewer_host::read_import` answers a file a document named from a directory a person
  supplied (ADRs 1155, 1186); what stops it inside `pdf-model` is deliberate — that crate has no
  filesystem and must not acquire one, and a template page from another file is a second
  `pdf_syntax::Document` reaching the interpreter.
- §10.8.3 (`partial`) — separation simulation. The control exists (ADR 1228) and the four steps are
  executed over the colourants one painting operation states (ADR 1229). What is left is the rest of
  step a): "Process the PDF as if separations were to be created" is a claim about the *page*, and it
  wants a plane per colourant. Overprint is not what is missing — §8.6.7 is implemented and
  §11.7.4.3's special blend mode draws §10.8.2's cyan-over-yellow example green on the four process
  planes — so what is missing is a plane for a **spot** ink, which reverts to the group's process
  components as it is painted (§11.7.3). Table 275's requirement is still answered by
  `requirements::unmet` by name. §10.8.3 itself still requires nothing: its verb is a permission and
  its four steps are a `should` conditional on performing one.

### 2. External-dependency-blocked — a crate release or an unheld specification

The reading is done; what is missing is a reviewed package on this tree's line, or a text the
project does not hold. **What would unblock them:** an upstream release (re-measurable, not
permanent) or an owner decision to acquire a specification.

- §12.10, §12.10.2 — a geospatial viewport's **projection**. Everything the file states is read and a
  person can trace a path in one: which system the map is in, how many `/GPTS`–`/LPTS` pairs register
  it, whether §12.10.2's `/Bounds` neatline covers the point (ADR 1191). Turning a projected
  coordinate into a latitude needs the EPSG registry or an ISO 19162 string, and §12.10.3 names both
  as texts outside this standard.
- §12.8.1, §12.8.3.1, §12.8.3.3, §12.8.3.3.1 — brainpoolP512r1 and Ed448 (ISO/TS 32002), named at
  runtime by the certificate's own identifier, with no reviewed arithmetic package on the `digest`
  line out of pre-release (ADR 1063). §12.8.3.1's row is where that measurement lives, with its date,
  the packages a search turns up that are refused for reasons other than a version number, and
  `cargo search bp512` as the whole of the re-check; the other three cite it rather than carrying a
  copy of the date. **Trust is not among these rows' debts**: a host supplies RFC 5280 section
  6.1.1's input (d) and `pdf_signature::verdict::Verdict` is where the third question's answer joins
  the other two (ADR 1076).
- §7.4.6 — CCITTFaxDecode's `/DamagedRowsBeforeError` concealment. `hayro-ccitt` exposes neither a
  failure's bit position nor a resume — `DecodeError`'s variants carry no position and
  `BitReader::byte_pos` is crate-private — so the search-and-substitute is a change to the shared
  decoder. Re-checked at the crate's head, which is also its latest release; no corpus image states
  the entry above zero.
- §7.4.9 — thirteen corpus JPEG 2000 codestreams decode one level off the reference software, held by
  name so an upstream release closing it fails the build. The one sentence of this clause addressed
  to a processor asks for *support* of the JPX baseline enumerated colour spaces, and "JPX baseline"
  is defined by ISO/IEC 15444-2, which the owner decided not to buy (`doc/questions/A51`). Checking
  the restriction on a file is not a reader's job and is not counted as debt (ADR 1184).
- §8.9.6.2 — smoothing a low-resolution stencil's edges. The shipped rasteriser filters after
  premultiplying, so a filtered tap is `mean(rgb) * mean(a)` where the clause asks for
  `mean(rgb * a)` — 131 of 255 on the painted channel at a magnified stencil's partly covered pixel,
  and it is what a reader sees. The premultiplication has to move into quorra's upload or sampler
  (`doc/QUORRA_FEEDBACK.md` section 39).
- §8.6.5.9 — black point compensation's ON half defers to ISO 18619, and **the blocker is the
  construction rather than the document**: no ICC text held states the algorithm, the content is
  obtainable free from the ICC, and ADR 1208 lists the eight things it adds — of which four move
  pixels, the LUT-destination estimation being the eleven levels ADR 0510 measured. A build across a
  round or two rather than a missing text (`doc/third-party-data.md`).
- §12.8.3.4.4 — enforcing a signature policy's constraints. Everything the held texts define is read:
  ETSI EN 319 122-1 clause 5.2.9's attribute whole — which policy, its digest with the all-zero *not
  known* kept apart, the URL, the notice meant to be shown, the specification identifier — and clause
  5.2.10's stored copy checked against that digest. What is missing is the specification the policy's
  own syntax is written in, and the signature *names* it, so the block is per file and named at
  runtime rather than one text to acquire (ADR 1219).

  **Beside this bucket and not in it**: §12.7.8.3.4 is `departed`, and the **XFDF** spelling of an
  FDF file's annotations needs ISO 19444-1 sections 6.4 and 6.6, where the preview held stops at
  5.7.1. `doc/questions/Q97` asks the owner for the text; the PDF-side import is built and needed
  none of it (ADRs 1223, 1224).

### 3. Hard rendering / architecture — a real build across several rounds

These are genuine model or rasteriser gaps, most of them priced and most unwitnessed on any first
page — the transparency residues reduce to *one raster carries the product of shape and opacity where
the clause wants the pair*, costed at a second raster per command (ADR 1022 section 5). **What would
unblock them:** focused multi-round work on a shape channel, a per-pixel second-rasterisation pass, a
colour route that is not affine, or the tessellation tolerance `pdf-model` cannot state in device
pixels.

- §11.7.5.2 — the transfer-function channel: a fully opaque mark's function seen through a later
  translucent one, a whole second rasterisation pass. The channel itself lands in both CPU backends;
  `Unsupported::TransferFunction` is narrowed to a shading's simplified ramp and a tiling cell
  interpreted once and copied. `render-gpu` has no pass over a Vello scene's result and refuses such
  a list by name; a page carrying the channel crosses the confinement as pixels (`doc/todo/13`,
  ADR 1125).
- §11.4.6 — a knockout element whose one alpha is the product of shape and opacity. A bare constant
  is read as opacity at every shape, not only where the two readings agree; what is left is the
  element whose two quantities reach the compositor as one number.
- §11.3.7.2, §11.3.7.3, §11.6.4.3, §11.7.4.4 — one raster carrying the product where the clause wants
  the pair, and the two-object seam a rasteriser leaves nothing between. **The one image that had
  both is drawn** (ADR 1218): a stencil under its own `/SMask` is routed to the device-scale producer,
  so §11.6.4.2's shape and §11.6.4.3's opacity reach a command apart. Each row still names its own
  case — a shape channel every command carries and a non-isolated group used as a knockout element
  (§11.3.7.2); a soft mask behind an image codec and one carrying Table 144's `/Matte` (§11.3.7.3); a
  group whose content painted under both readings of `/AIS` (§11.6.4.3); a fill-and-stroke pair that
  is a direct element of a non-isolated knockout group (§11.7.4.4).
- §11.3.6 — the clause's formula is executed and `scan::intersect_group` makes §11.3.7.2's group
  composition a change of weight rather than of colour; the row moves with the family above it.
- §11.4.7, §11.5.3, §11.6.6, §11.7.2 — a page that has already spent `colour::MAX_PRESSES`: such a
  group has no press to composite in, so its elements are painted in the parent's space and
  `PagePress::Beyond` names why. A four-component `/CS` that is neither `DeviceCMYK` nor a
  four-channel profile is refused on the same line and is *not* a debt beside it — §11.6.6 excludes
  such a space from being a group colour space at all, and a plain `/DeviceCMYK` with no profile
  behind it composites in ADR 0263's assumed inks (ADR 1230). §11.4.7 carries a
  second requirement of its own — a reference XObject's imported page is composited under the
  containing page's group attributes instead of its own — which nothing on this disk can witness,
  because no document here states a reference XObject at all.
- §11.7.5.3 — its first bullet is carried out and measured at the pixel against §10.4.2.4's own
  EXAMPLE (ADR 1207); the second is not, because a group's result reaches its parent's space as a
  cube resolved per pixel in a backend, where no colour space and no graphics state exist. The stated
  black generation in force at the `Do` would have to reach that conversion, which is the same
  per-pixel machinery the rows above want.
- §11.3.4 — the choice of route into a one-component blending space (ADR 0790), which `doc/todo/23`
  prices. That is the whole of what keeps this row open; the three-component spaces left it when
  §10.4.2.1's ranking was read against §10.3's own subject.
- §10.7.4 — the sharing half of a path whose subpaths overlap, which the two fill rules answer
  differently. `doc/todo/11` prices it. The clip region that is the union of two fills is built:
  `render-cpu` composes it and the other two backends refuse it by name, with
  `doc/QUORRA_FEEDBACK.md` section 51 the ask that would let a scene state it (ADR 1231). The row's
  four
  *departures* — anti-aliasing instead of the half-open square rule, the covered-area consequence of
  it, averaging over the pixel area, and the clipping paragraph's own product — are documented
  choices §10.7.1's NOTE licenses, each measured against a closed form rather than argued; §10.7 is
  this row's aggregate.
- §11.6.7 — a shading pattern's implicit *knockout* group, which follows §11.4.6. A **tiling**
  pattern's cell is evaluated once and its commands replicated, so each site keeps its own
  compositing and NOTE 1 is satisfied in geometry at every blend mode rather than only at Normal.

  **Beside this bucket and not in it**: §8.7.4.5.7 and §8.7.4.5.8 are `departed`. The patch travels
  to the backend and the fineness is derived there in device pixels (ADR 1217); the one branch left
  is a patch whose colours §8.7.4.4 requires be converted between its corners, which no corpus
  document takes.

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code.

- §12.5.6.23 — applying a redaction is built (`crates/pdf-transform/src/redact.rs`, the `redact`
  verb, `doc/questions/A64`): content within the region is removed and the bytes are gone, a painted
  path is cut to the region's complement, a §8.5.2.2 Bézier is split at the root where it crosses
  the region's edge, a §8.5.3.2 stroke is cut as the outline it marks, and a form is entered. The
  cases still owed are a clipping path, a stroke whose outline holds an arc, a `JPXDecode` image, an
  image stating `/Alternates`, a codec image carrying transparency, and an inline image whose codec
  or resource colour space the splice cannot re-encode. The overlay is a decided departure inside
  the row (`doc/todo/64`, ADRs 1124, 1195, 1196, 1236).
- §12.7.4.3 — variable text whose `/DA` matrix sends the line off *both* of the box's axes — a turn
  by something that is not a multiple of 90°, for which no length the box states is the room that
  line has — or whose linear part has no inverse and leaves no box at all. A scale, a mirror, a half
  turn, a quarter turn and a shear are all laid out (`doc/todo/22`, ADRs 1114, 1130).
- §8.9.6.4 — colour key masking on a `JPXDecode` image alone. Everywhere `unpack` sees the samples
  the range test runs in the domain the file wrote its integers in, measured on a sixteen-bit ramp
  whose two middle samples are one unit apart in sixteen bits and one byte in eight (ADRs 1121,
  1193). **The scaling is not the decoder's**, read against the pinned crate rather than against the
  sentence that said so: `hayro_jpeg2000::ComponentData` hands out `samples()` unscaled with
  `bit_depth()` beside it, and `pdf-sandbox`'s own palette-indices arm already reads them that way.
  The narrowing is `decode.rs`'s `jpx` calling `data_u8()` on the colour path and
  `pdf_sandbox::protocol::Raster` carrying no depth beside its bytes. Three edits, all on this side,
  and the twelve-bit fixture is in place to be inverted.
- §12.7.8.3.2 — Table 249's `/APRef`, and it alone. `/AP`, `/A`, `/AA` and `/IF` are applied by one
  rule, a value that lives in the other file crossing as a *value* rather than as a reference
  (ADRs 1186, 1223); `/RV` is XFA rich text on `CLAUDE.md`'s closed exclusion list and is not a
  requirement this project answers. `/APRef` has two branches and the second is built: **without**
  Table 253's `/F` the named page is one this document holds under §12.7.7's tree, and
  `named_page::page_as_form` makes it the widget's appearance (ADR 1235). **With** `/F` it names a
  second PDF file, which is §12.7.6.4's hazard, and what is missing is the *hop* rather than the
  copy or the conversion: a second host question raised while the first import is being applied,
  which is §12.6.4.4's suspended-walk shape `viewer_core`'s `Purpose::TargetRoot` already has. That
  is a `viewer-core` build, and §12.7.8.3.3's Table 253 `/F` is the same one. The unresolved
  reference is named on `Imported::refused`.

### 5. Answered, awaiting a real trigger — the public-key security handler

**One cluster, decided by the owner (`doc/questions/A66`) and waiting for a trigger, not an answer.**
§7.6.5's decryption stack lives in **a shared crypto crate below both `pdf-syntax` and
`pdf-signature`** — the `der`/`cms`/`x509`/`bigint` seam extracted once, the second extraction of
that seam after ADR 1020, with `EnvelopedData` and RSADP added there, its fuzz targets carried from
the first commit, and the private key a host input (ADR 1134). **What starts the build:** a real
trigger — a document whose recipient list could match a certificate the user holds, or a host asking
to supply a private key — not the clause's own sake. The robustness gain is nil (the five corpus
documents carry no recipient certificate this reader could match), so the calibrated refusal is the
honest state until then.

- §7.6.5, §7.6.5.1 (`reported`), §7.6.5.2 (`reported`), §7.6.5.3 (`reported`) — the handler itself and
  its dictionary and algorithms, refused by name before Table 23 is read.
- §7.6.6 — Table 27 and nothing else: its entries are the public-key handler's and reach nothing
  while §7.6.5 refuses the handler. Table 25's `/AuthEvent` is read and load-bearing.

**Five of `doc/questions/`'s Q files have no A file, and one of them holds a row in this map.**
`tools/state.sh questions` prints the parity; the open ones are Q67, Q72, Q76, Q97 and Q98.
§12.7.6.2 is the row that waits on one: submitting a form composes the request and hands it over —
method, URL, media type, body, with `viewer_host::policy::may_submit` the one place the answer is
decided — and what is absent is a *client*, because `viewer-host` has no HTTP dependency and
`xdg-open` cannot carry an entity body. A dependency is not a thing a round adds by itself, so
`doc/questions/Q98` puts three options to the owner with a recommendation (ADR 1062). Q72 could move
§12.7.5.4 out of the not-owed bucket below, Q97 could unblock the `departed` §12.7.8.3.4, and Q67 and
Q76 name no ledger row.

### 6. Genuinely buildable now — the campaign's next targets

A normal round can advance or close each of these today; there is no missing surface, no unheld
package, no cross-round architecture. Membership is re-derived from the ledger rather than carried: a
row is in this bucket when its note names none of those three.

- §11.6.5.2 — a mask in a one-component space that is not the `DeviceGray` Table 143 requires, and a
  `/Matte` whose parent space is neither `DeviceGray` nor `DeviceRGB`. The codec residue left this
  bucket: a codec-carrying mask is decoded once per document into an eight-bit grey plane behind
  `MaskCache`'s existing `ObjectId` key, bounded by `PREFER_DEVICE_SCALE_ABOVE` — the constant that
  already sent the pair down that route — and refused above it as it was before (ADR 1232).

Bucket 4's four rows are buildable by a normal round as well; what separates them is that each of
those closes a *case* while a row here closes the row. The seven this bucket last named — §7.5.6,
§7.7.2, §7.7.3.3, §7.7.4, §7.11.3, §8.9.5.1 and §8.10.2 — are all `implemented`, which is the bucket
doing what it is for and the reason its membership is re-derived rather than carried.

### Aggregate rows — no debt of their own; they move when a row below them does

These are `partial` only because something they carry is; each note says so and names what it carries.
They are not independently actionable — do not brief a round to *take* one. `tools/state.sh ledger`
counts them among `partial`; they flip when the last binding row flips.

§7.6, §8.9.6, §8.11, §8.11.1, §8.11.4, §8.11.4.1, §10.7, §11.3.7, §11.4, §11.4.3, §11.4.8, §11.6,
§11.6.4, §11.7, §11.7.4, §11.7.5, §12.1, §12.3, §12.5, §12.5.6, §12.6, §12.6.4, §12.7, §12.7.4,
§12.7.5, §12.7.6, §12.8, §12.8.3, §12.8.3.4.

Two of them carry no *child*: §11.4.3 and §11.4.8 defer to §11.4.4 and §11.4.6 in their own notes,
which is the same shape one level sideways.

### Not owed — a documented choice, an exclusion, a deprecation, or a standard-gap

The residue here is not fresh implementation work: the clause hands the feature to a project
exclusion, deprecates it, states no artwork, or the case is one the standard leaves undefined and this
tree reports rather than guesses. **This is the bucket principle 5 says decays** — a *not owed* claim
is a claim about the specification, so a revisiting round re-reads the titles around the clause before
trusting the word (the DeviceCMYK and transfer-function precedents in `CLAUDE.md`). Every row below
has been read against its clause and against Errata Collection 3, and each one's note records the
reading.

- §12.6.4.9, §12.6.4.10 (`reported`) — Sound and Movie. Each clause hands the playing to 13.2 in its
  own opening sentence, and clause 13 is on `CLAUDE.md`'s closed exclusion list; `spec-errata emit`
  files no annotation under either heading.
- §12.6.4.6 (`reported`) — Launch. Two different facts wear one shape and `action::launch` says which:
  a launch action with no `/F` is declined by Table 207's own sentence, because nothing here
  understands the three alternatives it names, and one *with* an `/F` is withheld by the sandbox,
  which is principle 3 rather than a gap. One document of 978 states a `/S /Launch` and it states an
  `/F`.
- §12.5.6.11, §12.5.6.12 (`reported`) — a caret's `/Sy` symbol and a rubber stamp's `/IT`, whose
  artwork the standard states nowhere (`doc/todo/26`); every corpus instance carries an appearance,
  and Table 184 is the only place any of the fourteen stamp legends is printed at all.
- §12.3.5.1 — Table 158's `Direction N`, which says the window "is not split" and then gives the whole
  region to the file navigation view: a `shall` conditional on a splitter this clause elsewhere makes
  a `may`, where this program gives the region to the page and the files to a side panel (ADR 0202's
  decision). `/Colors` is "a suggested set of colours" and carries no `shall` at a processor at all.
  Both are named by `unsupported_presentation`.
- §12.7.5.4 — a choice field's selection *mark*: the clause names it and states no quantity for it, so
  the page draws the list, auto-sized to what is visible and in `/Opt`'s own order, and reports which
  item `/V` names. This is the class `doc/questions/Q72` asks the owner about, and the row's status is
  the same under either reading of it.

### Expired premises — a decision whose factual ground the tree has since removed

A seventh shape, and it is not a bucket of ledger rows: these are *decisions* whose stated premise was
a fact about this tree, and the fact has changed. Each is re-tested against the code it names rather
than against its own words (ADR 1201's method), and what stands here is the build the expired premise
no longer blocks. A round takes one of these the way it takes a ledger row; the ADR that recorded the
premise is a record and stays as written.

- **ADR 1012 — the converter's inert verbs.** `executor::execute`, `archive/preserve.rs` and
  `archive/remedies.rs` carry out `derive`, `supply` and `preserve`, and `Qualifier::Shape` selects
  against `decision::SHAPES` (ADR 1211). What is left is the listing: `print_remedy_sites` prints one
  remedy sentence per site and no shape column, so an operator meets the distinction only in the error
  that names it.
- **ADR 0660 — Errata Collection 3 Issue #307's `shall not` as a writer's.** Discharged for four
  writers by ADR 1211: `filing::tree_root` is the only place this tree writes a `/Names` node, its key
  type is the prohibition, and three end-to-end tests hold that a source's null key does not cross a
  merge, a split or an attach. `pdf-transform`'s `structure.rs` writes §14.7.5's `/IDTree` with the
  same key type and is under the same guarantee without calling that function; a round that touches it
  routes it through.
