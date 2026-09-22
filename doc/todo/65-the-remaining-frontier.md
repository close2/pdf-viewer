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

- **Beside this bucket and not in it** — §12.3.5 and §12.3.5.1 are `departed`, and §12.3 with them:
  every name Table 160 defines is drawn, `/View` is obeyed for each of its four values, `/Sort`
  orders the panel and Table 158's `Direction N` gives the window to the file list. What is left is
  Table 157's `/Colors`, a suggestion a NOTE only recommends, and Table 158's `H`, `V` and
  `/Position`, whose splitter divides a pair of areas this window does not present — both named by
  `panel::unused_furniture` (ADRs 1168, 1215, 1251, 1252).
- **Beside this bucket and not in it** — §12.7.5.3 is `departed`: bit 26's `RichText` *formatting*,
  which §12.7.4.3 hands to XFA and which is therefore reported rather than drawn — the plain `/V` is
  laid out. The control's contents cross, a person's chosen pathname is read by `viewer_host::policy`
  and submitted as the field's value (ADR 1216), and the dialogue over that typing is built:
  `viewer-gtk` a `gtk4::FileDialog` and `viewer-qt` a `QFileDialog`, each inside the widget's own
  §12.5.2 rectangle, behind `viewer_host::policy::may_choose_file` (ADRs 1070, 1122, 1240).
- **Beside this bucket and not in it** — §12.7.8.3.3 is `departed`: Table 252's `/Rename` `true`,
  which asks for fields under names this document has not got. Table 253's `/F` is no longer the
  residue — a template page in another file is asked of a host under `--remote-documents=` and
  copied whole, and no second `pdf_syntax::Document` reaches the interpreter (ADR 1239).
- §10.8.3 (`partial`) — separation simulation. The control exists (ADR 1228) and the four steps are
  executed over the colourants one painting operation states (ADR 1229). What is left is the rest of
  step a): "Process the PDF as if separations were to be created" is a claim about the *page*, and it
  wants a plane per colourant. Overprint is not what is missing — §8.6.7 is implemented and
  §11.7.4.3's special blend mode draws §10.8.2's cyan-over-yellow example green on the four process
  planes — so what is missing is a plane for a **spot** ink, which reverts to the group's process
  components as it is painted (§11.7.3). That plane is now designed and priced in `doc/todo/23`:
  `ceil(S / 3)` rasters beside the chromatic and black halves, one run of the content stream per
  plane, the colourants enumerated from the page's resources before the first mark lands — and 87
  call sites across seven crates where "two rasters" is written into a type, which is why it is
  several rounds rather than one. Table 275's requirement is still answered by
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
- §12.7.6.2 — a submission's *client*. The request itself is composed — method, URL, media type,
  body — and `viewer_host::policy::may_submit` is the one place the answer is decided; what is
  absent is something to send it with, because `viewer-host` has no HTTP dependency and `xdg-open`
  cannot carry an entity body. A dependency is not a thing a round adds by itself, so
  `doc/questions/Q98` puts three options to the owner with a recommendation (ADR 1062).
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

- §11.7.5.2 — the two paints whose function is still inside their colour rather than on the mark.
  The channel is built and both CPU backends map the finished pixel through it, reading §11.6.4.2's
  shape for each kind of mark; `Unsupported::TransferFunction` is narrowed to a shading pattern's
  sampled ramp and a tiling cell interpreted once and copied to every site. The shading half is two
  hunks in `content/pattern.rs` and is designed (ADR 1255); the tiling half is a change to the
  replication rather than to the channel. `render-gpu` has no pass over a Vello scene's result and
  refuses such a list by name; a page carrying the channel crosses the confinement as pixels
  (`doc/todo/13`, ADRs 1125, 1255).
- §11.4.4, §11.4.6 — a knockout element whose one alpha is the product of shape and opacity. A bare
  constant is read as opacity at every shape, not only where the two readings agree; what is left is
  the element whose two quantities reach the compositor as one number. §11.4.4's recurrence and
  NOTE 3's backdrop removal are executed and measured; its residue is this same element, which is
  why the two rows move together (ADR 1022 section 5).
- §11.3.7.2, §11.3.7.3, §11.4.3, §11.6.4.3, §11.7.4.4 — one raster carrying the product where the clause wants
  the pair, and the two-object seam a rasteriser leaves nothing between. **The one image that had
  both is drawn** (ADR 1218): a stencil under its own `/SMask` is routed to the device-scale producer,
  so §11.6.4.2's shape and §11.6.4.3's opacity reach a command apart. Each row still names its own
  case — a shape channel every command carries and a non-isolated group used as a knockout element
  (§11.3.7.2); a soft mask behind an image codec and one carrying Table 144's `/Matte` (§11.3.7.3); a
  group whose content painted under both readings of `/AIS` (§11.6.4.3); a fill-and-stroke pair that
  is a direct element of a non-isolated knockout group (§11.7.4.4); and the same quantity in §11.4.3's
  own sentence, which asks that a group's "colour, shape, and opacity" be treated as one object's —
  `Command::Group`'s `alpha_is_shape` states the groups where the two coincide, and everywhere else
  the single object carries the product.
- §11.4.7, §11.5.3 — a page that has already spent `colour::MAX_PRESSES`: such a group has no press
  to composite in, so its elements are painted in the parent's space and `PagePress::Beyond` names
  why. The bound is no longer a round number — it is twice what the standard says one profile can
  be, Table 69's four intents against §8.6.5.9's `/UseBlackPtComp` less the pair that clause
  forbids, and the deepest page `examples/press_depth` finds names one press (ADR 1254) — so
  §11.6.6 and §11.7.2 record it as a `departed` bound rather than a debt and have left this bucket.
  §11.4.7 carries a second requirement of its own — a reference XObject's imported page is
  composited under the containing page's group attributes instead of its own — which nothing on
  this disk can witness, because no document here states a reference XObject at all. §11.5.3
  carries its own second one: a blend mode inside a subtractive group of more than one component.
  **Beside this bucket and not in it**: §11.3.4 is `departed`. Its one departure is the choice of
  route into a one-component blending space (ADR 0790), which `doc/todo/23` still prices; the
  precision that stood beside it here is closed, the cube into a parent's components being carried
  as the device's decoding, a linear grid and the space's own encoding rather than as one sampled
  grid (ADR 1267).
- §10.7.4 — the sharing half of a path whose subpaths overlap, which the two fill rules answer
  differently. `doc/todo/11` prices it. The clip region that is the union of two fills is built:
  `render-cpu` composes it and the other two backends refuse it by name, with
  `doc/QUORRA_FEEDBACK.md` section 51 the ask that would let a scene state it (ADR 1231). The row's
  four
  *departures* — anti-aliasing instead of the half-open square rule, the covered-area consequence of
  it, averaging over the pixel area, and the clipping paragraph's own product — are documented
  choices §10.7.1's NOTE licenses, each measured against a closed form rather than argued; §10.7 is
  this row's aggregate.
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
  clipping path keeps its boundary while its marks are cut, an image's `/Alternates` is dropped with
  its variants, and a JPEG 2000 image on its own grid is cleared and re-encoded. The cases still
  owed are a stroke whose outline holds an arc, a codec image whose decode is not on the grid its
  dictionary states, a `JPXDecode` image stating a non-zero `/SMaskInData` or more than eight bits
  per component, a codec image carrying transparency, and an inline image whose codec or resource
  colour space the splice cannot re-encode. The overlay is a decided departure inside the row
  (`doc/todo/64`, ADRs 1124, 1195, 1196, 1236, 1248).
- §12.7.4.3 — variable text whose `/DA` matrix states a linear part with **no inverse**: it sends
  the whole plane onto one line, so the box has no preimage that is a region and the glyph outlines
  enclose no area. Every invertible linear part is laid out, in the chord the box leaves the line
  a given baseline carries (`doc/todo/22`, ADRs 1114, 1130, 1247).
- **Beside this bucket and not in it** — §12.7.8.3.2 is `departed`: Table 249's `/APRef`, and it alone. `/AP`, `/A`, `/AA` and `/IF` are applied by one
  rule, a value that lives in the other file crossing as a *value* rather than as a reference
  (ADRs 1186, 1223); `/RV` is XFA rich text on `CLAUDE.md`'s closed exclusion list and is not a
  requirement this project answers. `/APRef`'s two branches are both built: **without** Table 253's
  `/F` the named page is one this document holds under §12.7.7's tree, and
  `named_page::page_as_form` makes it the widget's appearance (ADR 1235); **with** `/F` it names a
  second PDF, and the hop is `viewer-core`'s — a second host question raised while the first import
  is being applied, §12.6.4.4's suspended-walk shape, under the reader's `--remote-documents=` level
  (ADR 1239). §12.7.8.3.3's Table 253 `/F` is the same one and travels with it. A file nobody
  supplies is still named on `Imported::refused`.

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

- §11.6.5.2 — a `/Matte` on a parent whose raster is not on the grid its dictionary states, which is
  a `JPXDecode` codestream decoded at one of its own reduced resolution levels (§7.4.9 NOTE 3,
  ADR 0321): Table 143 pairs a `/Matte`'d mask with the parent's stated grid, and on a reduced
  raster that pairing does not exist, so the pre-blending is named through the shortfall rather than
  undone. The two residues this bucket named before are closed (ADR 1268) — a mask in a
  one-component space Table 143 does not permit supplies its samples and reports the departure, and
  the `/Matte` is undone in the image's own components before the colour conversion, in all three
  domains a route holds them in — and the codec residue left before that (ADR 1232).

Bucket 4's four rows are buildable by a normal round as well; what separates them is that each of
those closes a *case* while a row here closes the row. The seven this bucket last named — §7.5.6,
§7.7.2, §7.7.3.3, §7.7.4, §7.11.3, §8.9.5.1 and §8.10.2 — are all `implemented`, which is the bucket
doing what it is for and the reason its membership is re-derived rather than carried.

### Aggregate rows — no debt of their own; they move when a row below them does

These are `partial` only because something they carry is; each note says so and names what it carries.
They are not independently actionable — do not brief a round to *take* one. `tools/state.sh ledger`
counts them among `partial`; they flip when the last binding row flips.

§7.6, §8.9.6, §10.7, §11.3.7, §11.4, §11.6,
§11.6.4, §11.7, §11.7.4, §11.7.5, §12.1, §12.5, §12.5.6, §12.6, §12.6.4, §12.7, §12.7.4,
§12.7.5, §12.7.6, §12.8, §12.8.3, §12.8.3.4.


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
