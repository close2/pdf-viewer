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

**Read it against `CLAUDE.md`'s two denominators.** Coverage is the specification and the ledger is
its instrument; this map is a reading of that instrument. A bucket that says *blocked* is a claim
about coverage, and — principle 5 — a claim about the specification decays, so the not-owed bucket
below is the one a revisiting round re-reads first.

## The six reasons, and where measurement placed each row

The membership was taken by reading every `partial` and `reported` note once (trap 8: the reason is
the note's, never the corpus's). Two shapes fell outside the six and are named after them, because
forcing them into a bucket would mislead: **aggregate** rows, which state no debt of their own and
move only when a child does, and **not-owed** rows, whose residue is a documented choice, an
exclusion, a deprecation, or a case the standard leaves undefined.

### 1. Host-UI — a surface now in scope to build

`CLAUDE.md` principle 3 gives a document's restrictions four levels (`off`/`on`/ask/warn) whose
interface is now in scope to build (the earlier "none is to be built now" deferral was lifted by
the owner on 2026-09-16); the same surface covers printing, a collection's alternate
presentations. These rows
are not a gap in the reading — each one's core is already in place, waiting only on the surface and
the operation it drives. **What builds them:** the host work of `doc/todo/30`–`38` and RFC 0004's
print path.

- §12.2 — `/PrintArea` and `/PrintClip`, which need the `Page::print_box` that row names, and which
  are work rather than a capability nothing would use now that a page is rendered for paper.
  `/PrintScaling` has something to honour since session 1183 (ADR 1204).
- §12.3.5, §12.3.5.1 — a collection's `/Colors` and `/Split`, and §12.3.6's `FilmStrip`,
  `FreeForm` and `Linear`: surfaces one panel does not offer as alternatives. Table 153's `/View`
  is obeyed for every value since session 1189 (ADR 1215) and `/Sort` since 1165 (ADR 1168).
- §12.6.4.3 (`reported`) — a remote go-to that needs a host filesystem to reach another file.
- §12.6.4.6 (`reported`) — a launch action the sandbox withholds by design; deliberate, kept named.
- §12.7.5.3, §12.7.6.2 — a submit that needs a network, and a file chooser widget. A file-select
  control's *contents* cross since session 1189 (ADR 1216): a person's typed pathname is read by
  `viewer_host::policy::read_chosen` and submitted as the field's value.
- §12.7.8.3.3 — an FDF template page whose Table 253 `/F` puts it in another file. A host question
  since ADR 1186 rather than an impossibility: `viewer_host::read_import` answers a file a document
  named from a directory a person supplied (ADR 1155), and `pdf-model` has no filesystem by design.
- §10.8.3 (`reported`) — separation simulation, whose condition is a user's request this viewer has
  no control for.

### 2. External-dependency-blocked — a crate release or an unheld specification

The reading is done; what is missing is a reviewed package on this tree's line, or a text the
project does not hold. **What would unblock them:** an upstream release (re-measurable, not
permanent) or an owner decision to acquire a specification.

- §12.10, §12.10.2 — a geospatial viewport's **projection**. Everything the file states is read and
  a person can now trace a path in one: which system the map is in, how many `/GPTS`–`/LPTS` pairs
  register it, whether §12.10.2's `/Bounds` neatline covers the point. Turning a projected
  coordinate into a latitude needs the EPSG registry or an ISO 19162 string, and §12.10.3 names both
  as texts outside this standard.
- §12.8.1, §12.8.3.1, §12.8.3.3, §12.8.3.3.1 — brainpoolP512r1 and Ed448 (ISO/TS 32002): named at
  runtime by the certificate's own identifier; no reviewed arithmetic package on the `digest` line
  is out of pre-release (ADR 1063). §12.8.3.1's row carries the measurement with its date, the
  packages a search turns up that are refused for reasons other than a version number, and the
  command that re-measures it.
- §7.4.6 — CCITTFaxDecode's `/DamagedRowsBeforeError` concealment: `hayro-ccitt` exposes neither a
  failure's bit position nor a resume, so the search-and-substitute is a change to the shared
  decoder. Re-checked at the crate's head, which is also its latest release; the ledger row says what
  was looked at.
- §7.4.9 — thirteen JPEG 2000 codestreams decode one level off the reference software, held by name
  so an upstream release fails the build; and whether every baseline enumerated colour space is
  *supported* — the clause's one sentence here addressed to a processor — cannot be measured against
  a set ISO/IEC 15444-2 defines and the owner decided not to buy (`doc/questions/A51`). Checking the
  restriction on a file is not a reader's job and is no longer counted as debt (ADR 1184).
- §8.9.6.2 — smoothing a low-resolution stencil's edges needs premultiplication moved into quorra's
  upload or sampler (`doc/QUORRA_FEEDBACK.md` section 39).
- §8.6.5.9 — black point compensation's ON half defers to ISO 18619, and **the blocker is the
  construction rather than the document**: no ICC text held states the algorithm, the content is
  obtainable free from the ICC, and ADR 1208 lists the eight things it adds — of which four move
  pixels, the LUT-destination estimation being the eleven levels ADR 0510 measured. This is a build
  across a round or two rather than a missing text (ADRs 0510, 1208; `doc/third-party-data.md`).
- §12.7.8.3.4 (`departed`) — the **XFDF** spelling of an FDF file's annotations needs ISO 19444-1
  sections 6.4 and 6.6; the preview held stops at 5.7.1. The PDF-side import is built and needed
  none of it: the clause is one sentence about Table 254's `/Page` and everything else in such a
  dictionary is §12.5's (ADRs 1223, 1224). `doc/questions/Q97` asks for the text.
- §12.8.3.4.4 — enforcing a signature policy's constraints. Everything the held texts define is
  read: ETSI EN 319 122-1 clause 5.2.9's attribute whole — which policy, its digest with the
  all-zero *not known* kept apart, the URL, the notice meant to be shown, the specification
  identifier — and clause 5.2.10's stored copy checked against that digest. What is missing is the
  specification the policy's own syntax is written in, and the signature *names* it, so the block is
  per file and named at runtime rather than one text to acquire (ADR 1219).

### 3. Hard rendering / architecture — a real build across several rounds

These are genuine model or rasteriser gaps, most of them priced and most unwitnessed on any first
page — the transparency residues reduce to *one raster carries the product of shape and opacity where
the clause wants the pair*, costed at a second raster per command (ADR 1022 section 5). **What would
unblock them:** focused multi-round work on a shape channel, a per-pixel second-rasterisation pass, a
colour route that is not affine, or the tessellation tolerance `pdf-model` cannot state in device
pixels.

- §11.7.5.2 — the transfer-function channel: a fully opaque mark's function seen through a later
  translucent one, a whole second rasterisation pass (`doc/todo/13`, ADR 1125).
- §11.4.6 — a knockout element whose one alpha is the product of shape and opacity, and the
  own-backdrop construction's remainder on the oracle alone.
- §11.3.6, §11.3.7.2, §11.3.7.3, §11.4.4, §11.6.4.3, §11.7.4.4 — one raster carrying the product
  where the clause wants the pair, and the two-object seam a rasteriser leaves nothing between.
  **Not a shape channel**: §11.4.6's shape is an input to each element's composite rather than a
  pass over the finished page, so §11.7.5.2's construction does not transfer to it, and what the
  remaining case needs is `pdf_model::image::Picture` handing a stencil back beside its `/SMask`
  (ADR 1205).
- §11.4.7, §11.6.6, §11.7.2 — compositing a painted group in a group colour space, and a four-component
  space with no ICC profile behind it.
- §11.7.5.3 — its first bullet is carried out (ADR 1207); the second is not, because a group's result
  reaches its parent's space as a cube resolved per pixel in a backend, where no colour space and no
  graphics state exist. The stated black generation in force at the `Do` would have to reach that
  conversion, which is the same per-pixel machinery the rows above want.
- §11.3.4, §11.5.3 — the non-affine route into a one-component or three-curve blending space
  (`doc/todo/23`).
- §11.6.5.2 — a soft mask behind an image codec. `doc/todo/41`'s decoded-stream cache cannot take
  it: that cache holds the §7.4 chain's output keyed by the encoded allocation, and
  `Document::image_stream` runs only the chain *in front of* the codec. `MaskCache`'s existing
  `ObjectId` key would take it with no new key; what is owed is a bound on the decoded grey plane,
  and the corpus says the whole of it is 16 masks and 27.2 MB (ADR 1218's row, §11.6.5.2).
- §11.6.7 — a shading pattern's implicit knockout group (follows §11.4.6).
- §8.7.4.5.7, §8.7.4.5.8 — the patch travels to the backend and the fineness is derived there
  (ADR 1217); both rows are `departed` for the one branch left, a patch whose colours §8.7.4.4
  requires be converted between its corners, which no corpus document takes.
- §10.7.4 — a region that is the union of two fills under two rules, which no backend's clip
  vocabulary states (`doc/todo/11`).

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code. Membership is re-derived from the live ledger: the three
rows the section below records as having moved here are named here now, which they were not.

- §12.5.6.23 — redaction of a stroked, curved or clipping path, a `JPXDecode` image, an image
  stating `/Alternates`, a codec image carrying transparency, and an inline image behind a codec
  (`doc/todo/64`, ADRs 1124, 1195, 1196).
- §12.7.4.3 — variable text whose `/DA` matrix turns the line off *both* of the box's axes, or
  whose linear part has no inverse. A scale, a mirror, a half turn, a quarter turn and a shear are
  all laid out (ADRs 1114, 1130); what is left is the turn by something that is not a multiple of
  90°, for which no length the box states is the room the line has (`doc/todo/22`).
- §8.9.6.4 — colour key masking on a `JPXDecode` image whose components are not eight unsigned bits.
  The range test runs in the domain the file wrote its integers in wherever `unpack` sees the
  samples — measured on a sixteen-bit ramp whose two middle samples are one unit apart in sixteen
  bits and one byte in eight — so the residue is the decoder's eight-bit hand-off alone, not the
  raster downstream of it (ADR 1121, ADR 1193's sibling measurement in `image_masks.rs`).
- §12.7.8.3.2 — Table 249's `/AP`, `/APRef`, `/A` and `/AA`, named by the importer and not applied.
  The clause's replacing sentence is stated indicatively and covers every entry of the table, so
  each is a requirement unmet, `/RV` among them since ADR 1197 took the exclusion off it — what it
  would carry is formatting §12.7.4.3 does not apply, so importing it changes nothing a reader sees.
  `/AP`, `/A` and `/AA` left this list when `forms_data::carry` made a value that lives in the
  other file cross as a value rather than as a reference (ADR 1223), as Table 249's `/IF` left it
  before them. What is left is `/APRef`, and it has two branches: with Table 253's `/F` it names an
  external PDF file, which is §12.7.6.4's hazard and a host question first, and **without** `/F`
  the named page is one this document holds under §12.7.7's tree, which `named_page::NamedPages`
  already reads — so that branch is a build rather than a blocker.

### 5. Answered, awaiting a real trigger — the public-key security handler

**One cluster, decided by the owner (`doc/questions/A66`, 2026-09-16) and waiting for a trigger,
not an answer.** §7.6.5's decryption stack lives in **a shared crypto crate below both `pdf-syntax`
and `pdf-signature`** — the `der`/`cms`/`x509`/`bigint` seam extracted once (the second extraction
of that seam after ADR 1020), with `EnvelopedData` and RSADP added there, its fuzz targets carried
from the first commit, and the private key a host input (ADR 1134). **What starts the build:** a real
trigger — a document whose recipient list could match a certificate the user holds, or a host asking
to supply a private key — not the clause's own sake. The robustness gain is nil (the five corpus
documents carry no recipient certificate this reader could match), so the calibrated refusal
(ADR 1134) is the honest state until then.

- §7.6.5, §7.6.5.1 (`reported`), §7.6.5.2 (`reported`), §7.6.5.3 (`reported`) — the handler itself and
  its dictionary and algorithms, refused by name before Table 23 is read.
- §7.6.6 — the crypt filters' Table 27, which is the public-key handler's and reaches nothing while
  §7.6.5 refuses the handler.

Every other `doc/questions/Q*` has its `A*` (`tools/state.sh questions` prints the parity); rows that
once waited on the owner — the `departed` word (`A63`), redaction (`A64`), the JPEG 2000
specifications (`A51`) — are answered, their residues now in the buckets above by their real blocker.

### 6. Genuinely buildable now — the campaign's next targets

A normal round can advance or close each of these today; there is no missing surface, no unheld
package, no cross-round architecture. **All seven rows this bucket named have gone `implemented`** —
§7.5.6, §7.7.2, §7.7.3.3, §7.7.4, §7.11.3, §8.9.5.1 and §8.10.2 — which is the bucket doing what it
is for, and the reason its membership is re-derived from the ledger rather than carried. The two
that closed last were a reading and a disposition rather than a build: §7.5.6's version across
updates (ADR 1171) and §7.7.3.3's Table 31 entries, each of which hands its meaning to a clause
whose own row had already disposed of it (ADR 1172). A round looking for the next of these
re-derives the bucket: `tools/state.sh ledger` prints the `partial` rows, and one is in this bucket
when its note names no missing surface, no unheld package and no cross-round architecture.

### Aggregate rows — no debt of their own; they move when a child does

These are `partial` only because a child is; each note says so and names the children it carries.
They are not independently actionable — do not brief a round to *take* one. `tools/state.sh ledger`
counts them among `partial`; they flip when the last binding child flips.

§7.6, §7.6.4, §8.6.6, §8.9.6, §8.11, §8.11.1, §8.11.4, §8.11.4.1, §10.4, §10.4.2,
§11.3.7, §11.4, §11.4.3, §11.4.8, §11.6, §11.6.4, §11.7, §11.7.4, §11.7.5, §12.1, §12.3, §12.5,
§12.5.6, §12.6, §12.6.4, §12.7, §12.7.4, §12.7.5, §12.7.6, §12.8, §12.8.3, §12.8.3.4.

### Not owed — a documented choice, an exclusion, a deprecation, or a standard-gap

The residue here is not fresh implementation work: the clause hands the feature to a project
exclusion, deprecates it, states no artwork, or the case is one the standard leaves undefined and
this tree reports rather than guesses. **This is the bucket principle 5 says decays** — a *not owed*
claim is a claim about the specification, so a revisiting round re-reads the titles around the clause
before trusting the word (the DeviceCMYK and transfer-function precedents in `CLAUDE.md`).

**Every row below has been read against its clause and against Errata Collection 3, and each one's
note now records the reading.** Four claims decayed and left the bucket by the door at the bottom of
this section; two more kept their disposition and lost the reason they gave for it, which is written
into their notes rather than here. The membership below is what survived.

- §12.6.4.9, §12.6.4.10 (`reported`) — Sound and Movie: clause 13 multimedia, excluded by principle 5.
- §12.5.6.11, §12.5.6.12 (`reported`) — a caret's `/Sy` symbol and a rubber stamp's `/IT`, whose
  artwork the standard states nowhere (`doc/todo/26`); every corpus instance carries an appearance.
- §10.7 — scan-conversion departures §10.7.1's NOTE licenses. (§10.4.2.3 stood beside it until its
  grey-to-CMYK direction was argued and priced: it is `departed` on §10.4.2.1's ranking, ADR 1194.)
- §12.7.4.1 (`departed`) — a field-inheritance bound the clause forbids, kept because principle 3's
  resource budgets answer a `/Parent` cycle; reaching it is reported. The number was chosen from a
  measurement rather than asserted, after 32 was found to be refusing real fields (ADR 1198).
- §12.7.8.3.3 — the `/Rename` branch whose alternative would write fields onto an immutable
  document, and Table 253's `/F` beside it, which is in bucket 1. (§12.5.6.2 left this entry when
  `/Subj` and `/CreationDate` reached `popup::Popup` and `viewer_core::PopupWindow`, and `/DS`
  turned out not to be an entry of Table 172 at all — ADR 1224.)
- §12.7.5.4 — a choice field's selection: the clause states no appearance for it, so the page shows
  the list and reports which item `/V` names.
- §12.11.6's threshold is `restriction::Operation::Process`, so a reader who asks for the clause's
  "shall not continue" gets it and one who asks for nothing gets the document drawn with each
  requirement named (ADR 1167). **§12.11 and §12.11.3 are no longer beside it and are
  `implemented`**: the residue they carried — the weighting "against other documents in the choosing
  process" — is a sentence with no modal verb in it, conditioned on alternatives a reader opening one
  document does not have (ADR 1220).

### What left this bucket, and where it went

Five claims decayed when the rows were re-read against their clauses. Each row's note carries the
reading; this is only where they went.

- **§12.9, §12.9.1, §12.10, §12.10.2** — carried here as *nothing takes the two points a person
  would drag between*. Something does: `viewer_core::Query::Measure` takes a traced path in viewport
  pixels and answers with the strings the document's own number format arrays produced, the mode and
  the wording are `viewer_host::Measuring`, and the key `m` reaches all three windows. §12.9 and
  §12.9.1 are `implemented`; §12.10 and §12.10.2 keep the projection alone → **bucket 2,
  external-dependency-blocked** (ADR 1191).

- **§12.7.8.3.1** — `/EmbeddedFDFs` was carried here as *deprecated in PDF 2.0*. Table 246's cell
  states no deprecation, only Table 247's `/EncryptionRevision` does, and Errata Collection 3's
  Issue #173 rewrites the ambiguous prose so that the deprecation is FDF *encryption*'s. The entry
  is an ordinary PDF 1.4 array, its import is built, and the row is `departed` on the one thing
  that stays refused — Table 247's 40-bit RC4 key derivation (ADR 1185).
- **§12.8.3.4.4** — carried here as needing a signature policy no file carries. ETSI EN 319 122-1
  clause 5.2.10 defines the attribute that carries the policy document inside the signature; what
  is missing is the specification that defines the policy's syntax → **bucket 2,
  external-dependency-blocked**.
- **§12.7.8.3.2** — carried here as an FDF branch that would write onto an immutable document. The
  row's own note has said since ADR 0907 that the unapplied `/AP`, `/APRef`, `/IF`, `/A` and `/AA`
  are requirements of the clause's replacing sentence unmet; `/RV` was called the XFA exclusion and
  is not one either (ADR 1197) → **bucket 4, feature depth**.
- **§8.9.6.4** — carried here as a bit depth Table 87 leaves undefined. The same table says the
  depth *is* determined by the processor while decoding, and ADR 1121 settled that the residue is
  this tree's eight-bit raster → **bucket 4, feature depth**.

**Three of those four moves were recorded here and never carried out**: §8.9.6.4, §12.7.8.3.1 and
§12.7.8.3.2 named no bucket at all until bucket 4 above named them, and §12.8.3.4.4 none until
bucket 2 did.

Two rows lost their stated reason without leaving the bucket in the same pass, which is the same
decay one step short of a move: §12.5.6.2's `/ExData` (a scope claim `CLAUDE.md` does not support,
replaced by Table 173's own sentence) and §12.11.3 (a residue quoted across a join, half of which
needs no second document and is performed — and the other half is a sentence with no modal verb, so
that row and §12.11 with it are `implemented`, ADR 1220). **§10.4.2.3 was the one candidate for a
status this sweep could not take**, and ADR 1194 took it:
the clause defines the grey-to-CMYK conversion outright, `colour::rgb_to_cmyk` evaluates exactly it
for a grey, and the residue is a departure of §10.4.2.5's shape on §10.4.2.1's ranking — a decision,
which is why it needed a round that could write the ADR rather than a sweep.

### Expired premises — a decision whose factual ground the tree has since removed

A seventh shape, and it is not a bucket of ledger rows: these are *decisions* whose stated premise
was a fact about this tree, and the fact has changed. Each was re-tested against the code it names
rather than against its own words (ADR 1201's method), and what stands here is the build the expired
premise no longer blocks. A round takes one of these the way it takes a ledger row; the ADR that
recorded the premise is a record and stays as written.

- **ADR 1012 — the converter's inert verbs.** `executor::execute`, `archive/preserve.rs` and
  `archive/remedies.rs` carry out `derive`, `supply` and `preserve`; the attachment writer the ADR
  waited on predates it, and `Qualifier::Shape` selects against `decision::SHAPES` since ADR 1211.
  What is left is the listing: `--remedy-sites` does not enumerate the shapes, so an operator meets
  the distinction only in the error that names it.
- **ADR 1107 — the second element run's population.** The re-count is done and §11.4.4's note
  carries it: on the interpreter's own condition — `CpuRasterizer::group_buffer`'s conjunction over
  the display list's group commands, `crates/pdf-model/examples/non_isolated_group_census` — one
  curated first page reaches the construction where the file-stated count was zero, and 205 crawled
  ones do. What is left of this item is the pricing: a round measures the second run on
  `doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf` page 1, which is the first corpus page that
  reaches it and holds both of its groups under `/Multiply` (ADR 1214).
- **ADR 1113 — the per-pixel shape channel's cost. Taken, and the conclusion upheld on another
  reason** (ADR 1205): §11.7.5.3's NOTE puts the transfer function after all compositing, so
  `transfer_channel` can be a pass over a finished raster, where §11.4.6's weighted average
  multiplies the accumulation *as it stood under each element* — which a finished page no longer
  holds. That last arm is built (ADR 1218): a stencil under its own `/SMask` is routed to the
  device-scale producer so §11.6.4.2's shape and §11.6.4.3's opacity reach a command apart.
- **ADR 0660 — #307's `shall not` as a writer's.** Discharged by ADR 1211: the four writers call
  one function, `filing::tree_root`, whose key type is the prohibition, and three end-to-end tests
  hold that a source's null key does not cross. `structure.rs`'s §14.7.5 `/IDTree` is the fifth
  writer of the same shape and is under the same guarantee without calling it yet; a round that
  touches it routes it through the same function.
