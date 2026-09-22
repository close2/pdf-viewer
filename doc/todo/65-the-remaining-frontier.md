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
presentations, and dragging a measurement. These rows
are not a gap in the reading — each one's core is already in place, waiting only on the surface and
the operation it drives. **What builds them:** the host work of `doc/todo/30`–`38` and RFC 0004's
print path.

- §6.3.2.1, §7.6.4.1 — Table 22's printing, assembling and copying gated by an operation this
  program does not have; copying needs a host that says *this is a copy* (`doc/todo/38`).
- §8.11.4.5 — the `Print` event, built and waiting on a caller: `Purpose::Print` is what RFC 0004's
  section 4 calls print intent, and nothing but a print path is owed. The `Export` event has its
  caller already (`quorra-transform render`, the one operation meeting both of the clause's
  conditions), so only half of this row's entry is still frontier (ADR 1173, ADR 1174).
- §12.2, §12.5.6.22 — the print half of the viewer preferences and the watermark's tiling / n-up,
  each conditioned on a print dialogue; RFC 0004.
- §12.3.5, §12.3.5.1 — a collection's `/View` tile mode, `/Sort`, `/Colors`, `/Split`: surfaces one
  panel does not offer as alternatives.
- §12.5.3 — the Print flag and Locked, whose operating condition is a verb that moves or deletes an
  annotation (`doc/todo/33`), and a document restriction that must stay an *ask* (`doc/todo/38`).
- §12.6.4.15 — animating a transition outside a presentation.
- §12.6.4.3 (`reported`) — a remote go-to that needs a host filesystem to reach another file.
- §12.6.4.6 (`reported`) — a launch action the sandbox withholds by design; deliberate, kept named.
- §12.7.5.3, §12.7.6.2 — a file-select control's file *contents*, and a submit that needs a
  network.
- §12.9, §12.9.1, §12.10, §12.10.2 — measurement and geospatial: nothing takes the two points a
  person would drag between (projection also needs an external registry — see bucket 2).
- §10.8.3 (`reported`) — separation simulation, whose condition is a user's request this viewer has
  no control for.

### 2. External-dependency-blocked — a crate release or an unheld specification

The reading is done; what is missing is a reviewed package on this tree's line, or a text the
project does not hold. **What would unblock them:** an upstream release (re-measurable, not
permanent) or an owner decision to acquire a specification.

- §12.8.1, §12.8.3.1, §12.8.3.3, §12.8.3.3.1 — brainpoolP512r1 and Ed448 (ISO/TS 32002): named at
  runtime by the certificate's own identifier; no reviewed arithmetic package on the `digest` line
  is out of pre-release (ADR 1063).
- §7.4.6 — CCITTFaxDecode's `/DamagedRowsBeforeError` concealment: `hayro-ccitt` exposes neither a
  failure's bit position nor a resume, so the search-and-substitute is a change to the shared
  decoder.
- §7.4.9 — thirteen JPEG 2000 codestreams decode one level off the reference software, held by name
  so an upstream release fails the build; the baseline-feature check needs ISO/IEC 15444-2, which the
  owner decided not to buy (`doc/questions/A51`).
- §8.9.6.2 — smoothing a low-resolution stencil's edges needs premultiplication moved into quorra's
  upload or sampler (`doc/QUORRA_FEEDBACK.md` section 39).
- §8.6.5.9 — black point compensation's ON half defers to ISO 18619, which this tree does not hold
  (ADR 0510).
- §12.7.8.3.4 — FDF annotation dictionaries need ISO 19444-1 sections 6.4 and 6.6; the preview held
  stops at 5.7.1.
- §12.8.3.4.4 — a signature policy: ETSI EN 319 122-1 clause 5.2.10 defines the attribute that
  carries the policy document inside the signature; the specification that defines the policy's
  syntax is not held.

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
- §11.3.6, §11.3.7.2, §11.3.7.3, §11.4.4, §11.6.4.3, §11.7.4.4 — the missing shape channel every
  command would carry, and the two-object seam a rasteriser leaves nothing between.
- §11.4.7, §11.6.6, §11.7.2 — compositing a painted group in a group colour space, and a four-component
  space with no ICC profile behind it.
- §11.3.4, §11.5.3 — the non-affine route into a one-component or three-curve blending space
  (`doc/todo/23`).
- §11.6.5.2 — a soft mask behind an image codec, which would decode per raster request.
- §11.6.7 — a shading pattern's implicit knockout group (follows §11.4.6).
- §8.7.4.5.7, §8.7.4.5.8 — a patch mesh's tessellation fineness, fixed rather than derived from
  §10.7.3's smoothness because the silhouette tolerance is in device pixels `pdf-model` does not
  carry (ADR 0919).
- §10.7.4 — a region that is the union of two fills under two rules, which no backend's clip
  vocabulary states (`doc/todo/11`).

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code. Membership is re-derived from the live ledger: the three
rows the section below records as having moved here are named here now, which they were not.

- §12.5.6.23 — redaction of a painted path or form, and codec-encoded or shared images
  (`doc/todo/64`, ADR 1124).
- §8.6.6.5 — a DeviceN `/NChannel` space's per-component reversion through each colourant's own
  Separation, and its `/Colorants` dictionary — the spot case the display cannot combine without a
  colourant it lacks.
- §12.7.4.3 — variable text whose `/DA` matrix rotates, skews or mirrors, off the one axis this
  layout lays text along (`doc/todo/22`).
- §8.9.6.4 — colour key masking for samples that are not eight unsigned bits per component: the range
  test is applied wherever the samples the comparison sees are still in the domain the file wrote its
  integers in, and the residue is this tree's eight-bit image pipeline rather than a gap in the
  reading (ADR 1121).
- §12.7.8.3.1 — Table 246's `/EmbeddedFDFs`, an ordinary PDF 1.4 array of FDF files inside an FDF
  file. Only its *encrypted* form is deprecated in PDF 2.0, which Errata Collection 3's Issue #173
  makes plain, so the import is owed rather than excused (ADR 0907).
- §12.7.8.3.2 — Table 249's `/AP`, `/APRef`, `/IF`, `/A` and `/AA`, named by the importer and not
  applied. The clause's replacing sentence is stated indicatively and covers every entry of the
  table, so each is a requirement unmet; `/RV` alone is the XFA exclusion (ADR 0907).

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

**And one small thing a normal round can take.** §7.4.8 is `partial` for one entry: Table 13's
`/ColorTransform` is not read, so the clause's second case — no Adobe APP14 marker, the entry written
in `/DecodeParms` — gets the clause's own default in its place. `colour_transform_census` prices it
at nothing: 158 images over 17 crawled documents are in that case and every one states the value the
codestream already gives, and the 974 have none at all (ADR 1177). What is owed is `decode_jpeg`
reading the entry, and the census is the test that it changed no page.

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
- §10.7, §10.4.2.3 — scan-conversion departures §10.7.1's NOTE licenses, and a grey-to-CMYK
  conversion §10.4.2.1 ranks below the ICC route this tree takes. (The second is a *departure*
  rather than a formula nothing calls — `colour::rgb_to_cmyk` evaluates it as the nominal
  separation — and is the one candidate for `departed` a round with an ADR to write could take.)
- §9.8.3.3 — an FD class descriptor: the reader's half is done; enforcement is a validator's job the
  reader does not own.
- §12.7.4.1 — a field-inheritance bound the clause forbids, kept at 32 because principle 3's resource
  budgets outrank a depth no legitimate form approaches; reaching it is reported.
- §12.5.6.2, §12.7.8.3.3 — XFA-formatted `/RC`, and the `/Rename` branch whose alternative would
  write fields onto an immutable document. (`/ExData` is disposed of by Table 173 stating no entries
  for `MarkupGeo`, not by a scope claim: geospatial is §12.10's and is in scope.)
- §12.7.5.4 — a choice field's selection: the clause states no appearance for it, so the page shows
  the list and reports which item `/V` names.
- §12.11, §12.11.3 — the weighting of one document's requirements "against other documents in the
  choosing process", which needs a second document rather than a reading. (§12.11.3's *other*
  weighting is over one document's own requirements array and is performed.) **§12.11.6 is no longer
  beside them and is `implemented`**: its threshold is `restriction::Operation::Process`, so a reader
  who asks for the clause's "shall not continue" gets it and one who asks for nothing gets the
  document drawn with each requirement named (ADR 1167).

### What left this bucket, and where it went

Four claims decayed when the rows were re-read against their clauses. Each row's note carries the
reading; this is only where they went.

- **§12.7.8.3.1** — `/EmbeddedFDFs` was carried here as *deprecated in PDF 2.0*. Table 246's cell
  states no deprecation, only Table 247's `/EncryptionRevision` does, and Errata Collection 3's
  Issue #173 rewrites the ambiguous prose so that the deprecation is FDF *encryption*'s. The entry
  is an ordinary PDF 1.4 array and its import is owed → **bucket 4, feature depth**.
- **§12.8.3.4.4** — carried here as needing a signature policy no file carries. ETSI EN 319 122-1
  clause 5.2.10 defines the attribute that carries the policy document inside the signature; what
  is missing is the specification that defines the policy's syntax → **bucket 2,
  external-dependency-blocked**.
- **§12.7.8.3.2** — carried here as an FDF branch that would write onto an immutable document. The
  row's own note has said since ADR 0907 that the unapplied `/AP`, `/APRef`, `/IF`, `/A` and `/AA`
  are requirements of the clause's replacing sentence unmet; only `/RV` is the XFA exclusion →
  **bucket 4, feature depth**.
- **§8.9.6.4** — carried here as a bit depth Table 87 leaves undefined. The same table says the
  depth *is* determined by the processor while decoding, and ADR 1121 settled that the residue is
  this tree's eight-bit raster → **bucket 4, feature depth**.

**Three of those four moves were recorded here and never carried out**: §8.9.6.4, §12.7.8.3.1 and
§12.7.8.3.2 named no bucket at all until bucket 4 above named them, and §12.8.3.4.4 none until
bucket 2 did.

Two rows kept their place and lost their stated reason, which is the same decay one step short of a
move: §12.5.6.2's `/ExData` (a scope claim `CLAUDE.md` does not support, replaced by Table 173's own
sentence) and §12.11.3 (a residue quoted across a join, half of which needs no second document and
is performed). **§10.4.2.3 is the one candidate for a status this sweep could not take**: the
clause defines the grey-to-CMYK conversion outright, `colour::rgb_to_cmyk` evaluates exactly it for
a grey, and the residue is a departure of §10.4.2.5's shape on §10.4.2.1's ranking — which is a
decision, and a decision needs a round that can write the ADR.
