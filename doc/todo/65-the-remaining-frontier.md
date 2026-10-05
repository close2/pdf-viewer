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
decay is the shape — the six reasons a requirement stays open, plus the four shapes after them that
the six do not cover: aggregate rows, not-owed rows, expired premises, and `implemented` rows held
only by the robustness instrument.

**The population is the ledger's `partial` and `reported` rows, and only those.** A `departed` row
is a decision already taken and priced — `doc/HANDOVER.md` says what the word means — so it is not
open work and is not mapped here, even where the departure itself waits on something; what such a
row waits on is in its own note. Those worth knowing about are named beside the bucket their
residue would otherwise belong to, in that bucket's prose rather than as bullets. ADR 1237 is the
rule and its cost.

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
presentations. No row is here at present: what the bucket held was a gap in the surface rather than
in the reading, each row's core already in place and waiting only on the operation it drives, and
each row left as its surface was built. **What builds a row that arrives here:** the host work of
`doc/todo/30`–`38` and RFC 0004's print path. Table 147's print half is no longer among them: it is
read, answered and applied entry by entry, and §12.2 is `departed` for `/HideMenubar` over the
reader's own menu and for `/CenterWindow` in the one window whose toolkit cannot place it
(ADRs 1203, 1204, 1227, 1145, 1429).

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

### 2. External-dependency-blocked — a crate release or an unheld specification

The reading is done; what is missing is a reviewed package on this tree's line, or a text the
project does not hold. **What would unblock them:** an upstream release (re-measurable, not
permanent) or an owner decision to acquire a specification.

- §7.4.7 — a generic region on ISO/IEC 14492's extended template is refused out loud: the pinned
  `hayro-jbig2` reads EXTTEMPLATE and ignores it, and `doc/patches/hayro-jbig2-extended-template.patch`
  waits on the owner's fork (ADR 1459); `crates/pdf-sandbox/tests/t88_conformance.rs` says the day the
  codec takes it. No corpus stream uses it.
- §12.10, §12.10.2 — a geospatial viewport's **projection**. Everything the file states is read and a
  person can trace a path in one: which system the map is in, how many `/GPTS`–`/LPTS` pairs register
  it, whether §12.10.2's `/Bounds` neatline covers the point (ADR 1191). Turning a projected
  coordinate into a latitude needs the EPSG registry or an ISO 19162 string, and §12.10.3 names both
  as texts outside this standard. **Whether the projection may be built of this tree's own is the
  owner's open question `doc/questions/Q171`** — a WKT grammar and the inverse series from IOGP's
  free guidance note, or the registry's tables beside them — and neither row is built on it until
  the answer lands; this map records the wait and does not decide it.
- §7.4.9 — thirteen corpus JPEG 2000 codestreams decode one level off the reference software, held by
  name so an upstream release closing it fails the build. The one sentence of this clause addressed
  to a processor asks for *support* of the JPX baseline enumerated colour spaces, which ITU-T T.801
  M.9.2.4 lists — the held identical text of ISO/IEC 15444-2 (`doc/questions/A169`, ADR 1383).
  CMYK, sRGB, its grey, sYCC, ROMM-RGB and CIE Lab under D50 are drawn as defined; e-sRGB and
  e-sYCC (PIMA 7667), CIE Jab (CIE Publication 131) and CIE Lab under another illuminant take
  §7.4.9's device fallback, because those texts are not held — PIMA 7667 is sold (IS&T, ANSI), CIE
  131 is sold and superseded there by CIE 159, and a non-D50 Lab wants the illuminant's white point,
  which T.801 codes after ITU-T T.4 Annex E. That text is held and read: it gives the white point in
  XYZ for D50 alone and names every other illuminant by a code, its data left for further study, so
  no held text states the value this case needs. The row's note carries the date each text's
  availability was read on, which is the date to re-check it against. Checking the restriction on a file is not a reader's job and is not
  counted as debt (ADR 1184); `pdf-archive` checks all of M.9.2 for ISO 19005 (ADRs 1383, 1399).
- §12.8.3.4.4 — enforcing a signature policy's constraints. Everything the held texts define is read:
  ETSI EN 319 122-1 clause 5.2.9's attribute whole — which policy, its digest with the all-zero *not
  known* kept apart, the URL, the notice meant to be shown, the specification identifier — and clause
  5.2.10's stored copy checked against that digest. What is missing is the specification the policy's
  own syntax is written in, and the signature *names* it, so the block is per file and named at
  runtime rather than one text to acquire (ADR 1219).

ISO/TS 32002's brainpoolP512r1 and Ed448 are not in this bucket: they are the tree's own under the
owner's answer A170 (ADRs 1385 and 1386), so an upstream release is their *swap* condition rather
than a blocker, and `doc/stack.md`'s curve paragraph names it. **Re-checked on 2026-10-02** with
`cargo search bp512`, `cargo search brainpool` and `cargo search ed448`: RustCrypto has published no
`bp512`, and `ed448-goldilocks` is still on `0.14.0-pre.15`; RustCrypto's `ed448` 0.5.0 is the
signature and key-encoding types over it, not the arithmetic. Two packages that paragraph does not
name are candidates to be judged on `doc/stack.md`'s terms. `bp512-nestler` 0.2.1 (from `Basty-devel/bp512-nestler`)
is a brainpoolP512r1 over RustCrypto's `primeorder`, and its licence is PolyForm Noncommercial 1.0.0, which
this tree's Apache-2.0 cannot take whatever its review; the same author's `aegis-crypto` 0.1.5
names brainpoolP512r1 among its primitives under the same licence, and `krypteia-arcana` 0.2.0,
which `brainpool` also finds, states no Brainpool curve and no Ed448. `ed448-goldilocks-plus` 0.18.1
(BSD-3-Clause, from `mikelodder7/Ed448-Goldilocks`, on a stable version line) waits on the owner's
open question `doc/questions/Q192` — whether it counts as reviewed, which its own README says it has
not been — and this map does not decide it.
**Judged on 2026-10-01**, beside Q192 and on `doc/stack.md`'s terms, the two Ed448 packages
`cargo search ed448` also finds; neither is a candidate. `cx448` 0.1.1 (`BSD-3-Clause` in its
metadata, no licence file in its repository; one owner, `dignifiedquire/cx448`, five commits on
2025-04-10 and none since) carries RFC 8032's verification with context and Ed448ph and the section
7.4 vectors, but its README says it has not been audited or reviewed, it calls itself a temporary
port to be retired once RustCrypto's stable releases land, and it is built on the previous RustCrypto
generation (`elliptic-curve` 0.13, `crypto-bigint` 0.5, `digest` 0.10, `signature` 2), a second
stack beside the tree's. `tiny_ed448_goldilocks` 0.2.0 (`MIT`, one owner, `Dustin-Ray/tiny-ed448-goldilocks`)
has no signature scheme at all — no RFC 8032 verification and no 57-byte point encoding — and says
it is unaudited. Both licences are on `deny.toml`'s list; neither is reviewed, and neither is
RustCrypto's. Neither moves Q192's recommendation, and Q192 says so.
The same search on 2026-10-02 found every version above unchanged and lists five Ed448 names neither
judgement read, each still to be judged on `doc/stack.md`'s terms before it is called a candidate or
not: `ed448-rust` 0.1.1 (MIT/Apache-2.0, `lolo32/ed448-rust`), `minimal-ed448` 0.4.2 (MIT, inside
`serai-dex/serai`, whose description says unaudited), `frost-ed448` 3.0.0 (a FROST threshold
Schnorr scheme by its description, which is not RFC 8032's verification), `oxicrypto-sig` 0.3.0
(Apache-2.0, whose description names no Ed448) and `rs_ed448` 0.1.2 (GPL-2.0-only, a placeholder by
its description, which this tree's licence cannot take).

### 3. Hard rendering / architecture — a real build across several rounds

No row is here at present. The bucket is for a genuine model or rasteriser gap that only a build
across several rounds closes, and the last one, §10.7.4's mark for a stroke its matrix collapses,
was built (ADR 1360). *One raster carries the product of shape and opacity where the clause wants the pair* is not
among them: §11.4.6 is the one reader of the pair the standard has, every knockout element states its
shape, and §11.3.7.3's NOTE 2 licenses the product everywhere else (§11.4.3's row, ADR 1340).

**Beside this bucket and not in it**: §11.5.3, §11.6.6 and §11.7.2 are `departed` on one bound,
`colour::MAX_PRESSES`: a page that has spent it has no press left for its group's space, so its
elements are painted in the parent's and `PagePress::Beyond` names why. It is twice what the
standard says one profile can be, Table 69's four intents against §8.6.5.9's `/UseBlackPtComp` less
the pair that clause forbids, and the deepest page `examples/press_depth` finds names one press (ADR
1254). §11.5.3's blend mode inside a subtractive group of more than one component is composited in
the group's four components (ADR 1342). §11.3.4 is `departed` too. Its one departure is the choice of
route into a one-component blending space (ADR 0790), which `doc/todo/23` still prices; the
precision that stood beside it here is closed, the cube into a parent's components being carried as
the device's decoding, a linear grid and the space's own encoding rather than as one sampled grid
(ADR 1267).

**Beside this bucket and not in it**: §10.7.4 is `departed` on one pixel — the one a shape with no
extent along either axis lies in, ADR 1060's decision for §8.5.3.3.1's point and ADR 1360's for a mark
a matrix of rank zero collapses — and everything else it states is executed: a fill or a stroke its
matrix carries onto a line is that line (ADRs 1348, 1360), a path whose subpaths overlap is measured
as the set its rule declares inside (ADRs 1341, 1347), and the exact set's cost has a measured floor
(ADR 1359). Its four scan-conversion departures are choices §10.7.1's NOTE licenses; §10.7 is
`implemented` with it.

**Beside this bucket and not in it**: §8.7.4.5.7 and §8.7.4.5.8 are `departed`. The patch travels
to the backend and the fineness is derived there in device pixels (ADR 1217); the one branch left
is a patch whose colours §8.7.4.4 requires be converted between its corners, which no corpus
document takes.

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code.

No row is here at present. §14.3.1 and Annex L were, and are `implemented`: `update` and `merge`
state §14.3.3's deprecated entries in §14.3.2's packet alone in a PDF 2.0 file (ADR 1473), and a
merged root holding PDF 2.0 `Document`s from several sources holds them inside one (ADR 1474).

- **Beside this bucket and not in it** — §12.5.6.23 is `departed`: every content class the clause
  reaches is removed, a codec's output carried as the image's own samples in every colour space
  (ADR 1371), and Table 195's overlay is the one decided departure (ADR 1124). Two refusals stay as
  decisions with their cost recorded: a calculator function serving region-only colours (ADR 1363)
  and a `JPXDecode` component deeper than sixteen bits (ADR 1371).
- **Beside this bucket and not in it** — §12.7.4.3 is `departed`: a rich text field's formatting,
  which the clause hands to XFA 3.3, is reported rather than applied (ADRs 1122, 1197). A `/DA`
  whose `Tm` has no inverse is the clause carried out — the translation is the processor's to
  choose, and the producer's matrix draws no area — and says so (`doc/todo/22`).
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
to supply a private key — not the clause's own sake. **Four documents are the build's witness set,
not its trigger** (`doc/questions/A168`): `doc/corpora/pdfbox`'s `AESkeylength128.pdf` and
`AESkeylength256.pdf` decrypt with `PDFBOX-4421-keystore.pfx` (`CN=testnutzer`, serial `5F609C62`)
and its `AES128ExposedMeta.pdf` and `AES256ExposedMeta.pdf` with `PDFBOX-5249.p12` (`CN=test`,
serial `60FFD550`), both published beside them; the census's fifth, `3006236.pdf`, has no key
anywhere. `crates/pdf-syntax/tests/public_key_witnesses.rs` holds each recipient to its keystore.
No document is known to be encrypted to a key a real reader holds, so the calibrated refusal is the
state and the build stays untriggered.

- §7.6.5, §7.6.5.1, §7.6.5.2, §7.6.5.3 — all four `reported`: the handler itself and its dictionary
  and algorithms, refused by name before Table 23 is read.
- §7.6.6 — Table 27 and nothing else: its entries are the public-key handler's and reach nothing
  while §7.6.5 refuses the handler. Table 25's `/AuthEvent` is read and load-bearing.

**`doc/questions/`, and what each answer holds in this map.** `tools/state.sh questions` prints
the parity and names the open ones. The owner's answers of 2026-09-22 are built: A98's client sends
§12.7.6.2's request, so that row and §12.7.6 are `implemented` (ADR 1291); A72's bound — a quantity
may be chosen where the clause names the kind of mark and withholds only the number — moved
§12.4.4, §12.4.4.1, §12.5.6.19 and §12.6.4.15 to `implemented` (ADR 1299), puts §12.5.6.11's and
§12.5.6.12's artwork on its far side (both `departed` on it, ADR 1367), and moved §12.7.5.4 to `implemented` (ADR 1323); A76's
mode is drawn in `raster/` (ADR 1295); A97's text is held and §12.7.8.3.4 reads it (ADR 1297);
A100 made §8.6.6.5 `implemented`; A03 ratified Annex F, whose rows are all `implemented` (ADRs 1293,
1309); A67 names no ledger row. A130 wrote TLS into principle 3 as its one named exception, with
the host checked first (ADR 1327), and A131 read F.4.1 as padding each item's run of a hint table
(ADR 1328); neither moves a row.

### 6. Genuinely buildable now — the campaign's next targets

A normal round can advance or close each of these today; there is no missing surface, no unheld
package, no cross-round architecture. Membership is re-derived from the ledger rather than carried: a
row is in this bucket when its note names none of those three.

No row is here at present. §12.6.4.6 was, and is `departed`: a launch whose Table 207 `/F` is a
PDF opens under the reader's remote-documents level, and one that names an application is the one
requirement withheld, by principle 3's sandbox (ADRs 1368, 1358).

Bucket 4's rows are buildable by a normal round as well; what separates them is that each of
those closes a *case* while a row here closes the row. The seven this bucket last named — §7.5.6,
§7.7.2, §7.7.3.3, §7.7.4, §7.11.3, §8.9.5.1 and §8.10.2 — are all `implemented`, which is the bucket
doing what it is for and the reason its membership is re-derived rather than carried.

### Aggregate rows — no debt of their own; they move when a row below them does

These are `partial` only because something they carry is; each note says so and names what it carries.
They are not independently actionable — do not brief a round to *take* one. `tools/state.sh ledger`
counts them among `partial`; they flip when the last binding row flips.

§7.6, §12.1, §12.8, §12.8.3, §12.8.3.4.


### Not owed — a documented choice, an exclusion, a deprecation, or a standard-gap

The residue here is not fresh implementation work: the clause hands the feature to a project
exclusion, deprecates it, states no artwork, or the case is one the standard leaves undefined and this
tree reports rather than guesses. **This is the bucket principle 5 says decays** — a *not owed* claim
is a claim about the specification, so a revisiting round re-reads the titles around the clause before
trusting the word (the DeviceCMYK and transfer-function precedents in `CLAUDE.md`). A row that
arrives here is read against its clause and against Errata Collection 3, and its note records the
reading.

No row is here at present, and the bucket was emptied by re-reading it rather than by building.
§12.6.4.9's and §12.6.4.10's own opening sentence hands the playing to 13.2, so both are
`out-of-scope` on principle 5's clause 13 entry, the position §12.6.4.14's rendition action is in.
§12.5.6.11's caret and §12.5.6.12's stamp legends are `departed`: `doc/questions/A72` put their
artwork on the far side of its bound, which made it a decision rather than a debt (ADR 1367).
§12.6.4.6 did not hold: Table 207's `/F` names a document to open as well as an application to run,
and the half that names a PDF is built (ADRs 1368, 1358).

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

### Held only by the robustness instrument — rows that name no fixture

An eighth shape, and it holds no row: every `implemented` and `partial` row names a test whose input
a fixture builds and whose expected value its clause derives. A row held only by an ignored walk, a
census, or a corpus witness that returns having read nothing where `doc/pdf.js`, `doc/corpora/` or
`doc/veraPDF-corpus` is absent still counts as executed under the owner's A100, the corpus being a
control; what it owes is that fixture, added to its `test` list (principle 5).

The membership is printed, never written here. `cargo run -p conformance --bin ledger` prints, per
status, the rows held by a fixture, held only by walks or witnesses, and naming no test; the ledger
gate (`cargo test -p conformance --test conformance the_ledger_agrees_with_the_standard_and_with_the_tree
-- --nocapture`) names each such `implemented` row and holds their number to `ONLY_WALKS_CEILING` with
an `==` assertion, so a row arriving with walks alone fails the build (ADRs 1497, 1509). The
classifier reads a corpus root anywhere in a function's body, comments included, so a helper whose
comment names `doc/pdf.js` makes every test calling it a witness: `viewer_core::notes::about` is one,
which is why §14.8.6's notes test was read as a witness although it builds its own document.
