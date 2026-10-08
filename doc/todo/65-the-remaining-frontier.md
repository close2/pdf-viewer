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

- **Beside this bucket and not in it** — §12.3.5 and §12.3.5.1 are `implemented`, and §12.3 with them:
  every name Table 160 defines is drawn, `/View` is obeyed for each of its four values, `/Sort`
  orders the panel and Table 158's `Direction N` gives the window to the file list. Unused, as a choice
  the clause's `may` leaves, are Table 157's `/Colors`, a suggestion a NOTE only recommends, and
  Table 158's `H`, `V` and `/Position`, whose splitter divides a pair of areas this window does not
  present — both named by `panel::unused_furniture` (ADRs 1168, 1215, 1251, 1252, 1622).
- **Beside this bucket and not in it** — §12.7.5.3's file-select control: its contents cross, a person's chosen pathname is read by `viewer_host::policy`
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
  codec takes it. No corpus stream uses it. Read on 2026-10-08: the manifest takes the codec from
  upstream's repository at `ea9c81dc`, the commit 0.3.1 was released from, and neither it nor any
  branch of the owner's fork `close2/hayro` carries the patch; applying it there is the owner's step
  (ADR 1714).
- §7.4.9 — the one sentence of this clause addressed to a processor asks for *support* of the JPX
  baseline enumerated colour spaces, which ITU-T T.801 M.9.2.4 lists — the held identical text of
  ISO/IEC 15444-2 (`doc/questions/A169`, ADR 1383). CMYK, sRGB, its grey, sYCC, ROMM-RGB and CIE Lab
  under every illuminant its `IL` field names — the white point §8.6.5.4's EXAMPLE, T.4 or the CIE's
  free data states, and a colour temperature's Planckian radiator (ADRs 1712, 1713) — are drawn as
  defined; e-sRGB and e-sYCC (PIMA 7667) and CIE Jab (CIE Publication 131) take §7.4.9's device
  fallback, because their texts are not held (read on 2026-10-08, the row's note says where): PIMA
  7667 is sold by IS&T, and CIE 131 is listed by the CIE as superseded by CIE 159, itself withdrawn
  in 2022 for CIE 248; whether to buy them is `doc/questions/Q348`. Checking the restriction on a file is not a reader's job and is not counted as debt
  (ADR 1184); `pdf-archive` checks all of M.9.2 for ISO 19005 (ADRs 1383, 1399). The thirteen
  corpus codestreams a level off the reference software on the irreversible path are not debt:
  ISO/IEC 15444-1 leaves that path's reconstruction and precision to the decoder (ADR 1574).
- §12.8.3.4.4 — enforcing a signature policy's constraints. Everything the held texts define is read:
  ETSI EN 319 122-1 clause 5.2.9's attribute whole — which policy, its digest with the all-zero *not
  known* kept apart, the URL, the notice meant to be shown, the specification identifier — and clause
  5.2.10's stored copy checked against that digest. What is missing is the specification the policy's
  own syntax is written in, and the signature *names* it, so the block is per file and named at
  runtime rather than one text to acquire (ADR 1219). The signatures on this disk name none, and the
  policy most of them sign under is a PDF bound by its own digest — human-readable, which a validator can
  bind and show but not enforce (ADR 1709). That half is built: the copy at the qualifier's URL is
  fetched under the reader's network level, bound by the signed digest and opened beside the document,
  and the sentence says the constraints were not enforced (ADR 1728); no window calls it until
  `viewer_core` hands a host the policy's URL.

ISO/TS 32002's brainpoolP512r1 and Ed448 are not in this bucket: they are the tree's own under the
owner's answer A170 (ADRs 1385 and 1386), so an upstream release is their *swap* condition rather
than a blocker, and `doc/stack.md`'s curve paragraph names it. **What reopens the swap is one of two
things and nothing else (ADR 1538)**: a release on RustCrypto's own line — `ed448-goldilocks`
leaving `0.14.0-pre` for a stable version, or a `bp512` published from `RustCrypto/elliptic-curves`
— or a package with an audit on record covering the curve's arithmetic and, for Ed448, RFC 8032
section 5.2.7's cofactored verification. A package a search lists below that bar is not read again.
The re-check is a round's command, not a `tools/state.sh` section, because it needs the network and
a section may not wait on one (ADR 1538); it prints the two versions and any package whose crates.io
description names an audit without denying one, and nothing else:

```sh
cargo info ed448-goldilocks | grep -m1 '^version:'
cargo info bp512 2>&1 | grep -E '^(version|repository):|could not find'
for q in ed448 brainpool bp512; do curl -s -A 'pdf-viewer curve re-check' \
  "https://crates.io/api/v1/crates?q=$q&per_page=100" | PYTHONDONTWRITEBYTECODE=1 python3 -c '
import json, re, sys
for c in json.load(sys.stdin)["crates"]:
    d = c.get("description") or ""
    if re.search(r"audit", d, re.I) and not re.search(r"unaudit|not\W+(\w+\W+)?audit", d, re.I):
        print(c["name"], c["max_version"], d)'; done
```

**Last run on 2026-10-05**: `ed448-goldilocks` `0.14.0-pre.15` (0.9.0 its newest stable), no `bp512`
in the registry, and no description naming an audit — `minimal-ed448` and its mirror say
*unaudited* and `aegis-crypto` says it is not independently audited. `ed448-goldilocks-plus` 0.18.1
(`BSD-3-Clause`, `mikelodder7/Ed448-Goldilocks`, a stable line) does not count: its README says it
has not been reviewed, and the owner's answer `doc/questions/A192` defines reviewed as stable and
from the supplier the tree already trusts, under which every package judged below is no and the
trigger is RustCrypto's `ed448-goldilocks` reaching a stable line that carries the signature scheme.

**The packages judged, each once, on `doc/stack.md`'s terms from its crates.io record, its
repository and its source** — none a candidate, and none re-read:

- **2026-10-01.** `cx448` 0.1.1 (`BSD-3-Clause` in its metadata, no licence file; one owner, five
  commits on 2025-04-10): RFC 8032's verification with context and Ed448ph and the section 7.4
  vectors, but unaudited by its README, a self-described temporary port, and on the previous
  RustCrypto generation (`elliptic-curve` 0.13, `crypto-bigint` 0.5). `tiny_ed448_goldilocks` 0.2.0
  (`MIT`, one owner): no signature scheme and no 57-byte encoding, unaudited.
- **2026-10-02.** `ed448` 0.5.0: RustCrypto's signature and key-encoding types, no arithmetic.
  `bp512-nestler` 0.2.1 (a brainpoolP512r1 over `primeorder`) and `aegis-crypto` 0.1.5: PolyForm
  Noncommercial 1.0.0, which this tree's Apache-2.0 cannot take whatever their review.
  `krypteia-arcana` 0.2.0: states no Brainpool curve and no Ed448.
- **2026-10-05.** `ed448-rust` 0.1.1 (`MIT/Apache-2.0`, one owner, last pushed 2023): a port of the
  RFC's Python with its not-for-production warning, over `num-bigint` and `sha3` 0.9.
  `minimal-ed448` 0.4.2 (`MIT`, inside `serai-dex/serai`): unaudited by its description, a group with
  no signature scheme that rejects torsion, which is not section 5.2.3's decoding. `frost-ed448`
  3.0.0 (the Zcash Foundation): the one package with a review on record — NCC Group's 2023
  assessment of 0.6.0 and the `ed448-goldilocks` 0.9.0 operations it uses — but RFC 9591's threshold
  scheme, whose single-signer path rejects the identity and every torsion component that section
  5.2.7's cofactored equation admits, on `sha3` 0.10. `oxicrypto-sig` 0.3.0 (*Alpha*, pre-1.0): a
  wrapper over the `0.14.0-pre.15` this tree declines. `rs_ed448` 0.1.2 (`GPL-2.0-only`): no
  implementation, and a licence not on `deny.toml`'s list.
- **Listed by `--limit 20` searches and never judged, because the rule puts them below the bar
  without a reading**: `minimal-ed448-mirror`, `lit-frost-ed448`, `sodot-ed448`, `crrl`,
  `capycrypt`, `ciphersuite`, `purecrypto` and `static-dh-ecdh`. None is RustCrypto's and none
  names an audit; the re-check above is what would bring one back.

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
the group's four components (ADR 1342). §11.3.4 is `implemented`, its route into a one-component
blending space a choice §10.4.2.1's ranking leaves to the processor (ADRs 0790, 1622), which
`doc/todo/23` still prices; the precision that stood beside it here is closed, the cube into a parent's components being carried as
the device's decoding, a linear grid and the space's own encoding rather than as one sampled grid
(ADR 1267).

**Beside this bucket and not in it**: §10.7.4 is `departed` on one pixel — the one a shape with no
extent along either axis lies in, ADR 1060's decision for §8.5.3.3.1's point and ADR 1360's for a mark
a matrix of rank zero collapses — and everything else it states is executed: a fill or a stroke its
matrix carries onto a line is that line (ADRs 1348, 1360), a path whose subpaths overlap is measured
as the set its rule declares inside (ADRs 1341, 1347), and the exact set's cost has a measured floor
(ADR 1359). Its three other departures — anti-aliasing, the area it costs and a reduced image's
average — are departures §10.7.1's NOTE describes and does not license (ADR 1560), and no
contradicted verdict the oracle holds rests on any of the four (ADR 1622); §10.7 is `implemented`
with it.

**Beside this bucket and not in it**: §8.7.4.5.7 and §8.7.4.5.8 are `departed`. The patch travels
to the backend and the fineness is derived there in device pixels (ADR 1217); the one branch left
is a patch whose colours §8.7.4.4 requires be converted between its corners, which no corpus
document takes.

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code.

**One build, the rows below it: rich text formatting** (ADRs 1623, 1634, 1635, 1648, 1649, 1660, 1661, 1682). A rich text
string is laid out in the formatting it states: `pdf_model::rich_text` reads the XHTML subset and the
CSS2 and XFA properties XFA 3.3's chapter 27 names, beneath a `/DS`, and sets each run in its own face,
size, width, colour, alignment and spacing, list tags and tab stops included, in comb cells and in
UAX #9's order, with a host's caret, point and range answered from the runs; a changed rich value
regenerates the whole appearance, a save writes `/RV` beside `/V`, and an import carries Table 249's
`/RV` and XFDF's `<value-richtext>`, which a save writes (ADR 1661). XFA 3.3 is held at `/home/AI/specs/XFA-3_3.pdf`
(`doc/third-party-data.md`), cited by section, never quoted. What is left, row by row:

- §12.7.4.3 — `kerning-mode:pair` in §9.6.2.2's fourteen, reported as `Owed::RichTextUnapplied`,
  which needs pair data no file of this tree holds (`doc/questions/Q308`); a face the document
  embeds is kerned by its own `GPOS` or `kern` pairs (ADR 1682), `GPOS`'s from the script table the run's
  characters select and the language system the widget's or the document's `/Lang` selects (ADR 1708),
  and a contextual lookup is not pair kerning (ADR 1696). Leaders, right-to-left tabs, every list type,
  the nearest width and a machine face per character are built, and following a link and resolving
  `xfa:embed` are choices the clause's delegation of formatting leaves (ADR 1660).
- §12.7.5.3 — Table 231 bit 26, the same residue seen from the field's own table.
- §12.5.6.6 — Table 177's `/RC` and `/DS`, drawn; §12.7.4.3's residue is this note's too.

- **Beside this bucket and not in it** — §12.5.6.23 is `departed` on two refusals with their cost
  recorded, a calculator function serving region-only colours (ADR 1363) and a `JPXDecode` component
  deeper than sixteen bits (ADR 1371); every other content class the clause reaches is removed, and
  Table 195's overlay is a choice its own `should` leaves (ADRs 1124, 1622).

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

- §7.6.5.1, §7.6.5.2, §7.6.5.3 — all three `reported`: the handler, its dictionary and its
  algorithms, refused by name before Table 23 is read; §7.6.5 above them is their aggregate.
- §7.6.6 — Table 27 and nothing else: its entries are the public-key handler's and reach nothing
  while §7.6.5 refuses the handler. Table 25's `/AuthEvent` is read and load-bearing.

**`doc/questions/`, and what each answer holds in this map.** `tools/state.sh questions` prints
the parity and names the open ones. The owner's answers of 2026-09-22 are built: A98's client sends
§12.7.6.2's request, so that row and §12.7.6 are `implemented` (ADR 1291); A72's bound — a quantity
may be chosen where the clause names the kind of mark and withholds only the number — moved
§12.4.4, §12.4.4.1, §12.5.6.19 and §12.6.4.15 to `implemented` (ADR 1299), puts §12.5.6.11's and
§12.5.6.12's artwork on its far side (both `implemented` with that choice named, ADRs 1367, 1622), and moved §12.7.5.4 to `implemented` (ADR 1323); A76's
mode is drawn in `raster/` (ADR 1295); A97's text is held and §12.7.8.3.4 reads it (ADR 1297);
A100 made §8.6.6.5 `implemented`; A03 ratified Annex F, whose rows are all `implemented` (ADRs 1293,
1309); A67 names no ledger row. A130 wrote TLS into principle 3 as its one named exception, with
the host checked first (ADR 1327), and A131 read F.4.1 as padding each item's run of a hint table
(ADR 1328); neither moves a row. Of the owner's answers of 2026-10-05, A171 is §12.10's build (the
bullet in bucket 6), A192 closes the Ed448 search (bucket 2's curve paragraph), A193 makes
JavaScript a build stream whose Tier 0 runs the one-call `AF*` scripts while §12.6.4.17 stays
`out-of-scope` (ADRs 1578, 1579) and whose Tier 1 engine runs in every window at the reader's
`Scripts` level, `off` by default, with the row's move waiting on the owner's amendment that
`doc/questions/Q286` proposes (ADRs 1590, 1591, 1616), A227's `zune-jpeg` fork is the one the manifest pins, carrying
both patches (ADRs 1589, 1730), and A209 owes nothing; none of the last four moves a row this map holds.

### 6. Genuinely buildable now — the campaign's next targets

A normal round can advance or close each of these today; there is no missing surface, no unheld
package, no cross-round architecture. Membership is re-derived from the ledger rather than carried: a
row is in this bucket when its note names none of those three.

- §12.10.2 — a geospatial viewport's **registration in degrees**. Everything the file states
  is read, a person can trace a path in one (ADR 1191) and reads a position in decimal degrees
  through the registration's affine map (ADR 1593), and the inverse projection is built on
  `doc/questions/A171`: no EPSG registry, because the census found no system named by code alone; the
  older WKT form the files carry and ISO 19162's own; eight methods to Guidance Note 7-2's worked
  examples in both directions (ADRs 1586, 1587, 1672), so a projected `/DCS` on the file's datum is
  displayed and a position given to a viewport answers its page point. What is left: every one of
  the census's 158 projected maps writes its `/GPTS` as degrees where Table 269 says eastings and
  northings, refused by name until `doc/questions/Q271` is answered — the forward leg those maps
  would be fitted through is built and waits on that answer alone. A `/DCS` on another datum stays
  outside A171's scope.

§7.4.9 is not here: its buildable case, a CIE Lab image under an illuminant other than D50, is drawn
under that illuminant's white point (ADR 1713), and the row stays in bucket 2 for e-sRGB, e-sYCC and
CIE Jab, whose texts are not held.

§7.10.2 is `implemented`: Table 39's `/Order 3` is the not-a-knot cubic spline, the choice ADR 1636
writes down against the clause's own four-sample threshold, and §7.10 settles with it.

§13.4 was here, and is `departed`: Table 306's stream `/Poster` is drawn in `/Rect` at a placement
this program chose, and the playing — the boolean form with it — is the one withholding, on the
clause 13 exclusion `doc/questions/A33` bounded (ADR 1561).

§12.6.4.6 was here, and is `departed`: a launch whose Table 207 `/F` is a
PDF opens under the reader's remote-documents level, and one that names an application is the one
requirement withheld, by principle 3's sandbox (ADRs 1368, 1358).

Bucket 4's rows are buildable by a normal round as well; what separates them is that each of
those closes a *case* while a row here closes the row. The seven this bucket last named — §7.5.6,
§7.7.2, §7.7.3.3, §7.7.4, §7.11.3, §8.9.5.1 and §8.10.2 — are all `implemented`, which is the bucket
doing what it is for and the reason its membership is re-derived rather than carried.

### Aggregate rows — no debt of their own; they move when a row below them does

A heading over a row that owes takes its status from its leaves — `partial`, or `reported` where every
leaf is — which `cargo test -p conformance` checks and `--bin ledger -- --write` sets (ADR 1599); this list is
exactly the rows `Ledger::is_aggregate` names and the frontier gate fails on any other. Each note opens
"Aggregate of the rows below" and the reason is the owing row's. They are not independently actionable
— do not brief a round to *take* one; they flip when the last binding row flips.

§7.4, §7.6, §7.6.5, §12.5, §12.5.6, §12.7, §12.7.4, §12.7.5, §12.8, §12.8.3, §12.8.3.4, §12.10.


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
§12.5.6.11's caret and §12.5.6.12's stamp legends are `implemented` with the choice named:
`doc/questions/A72` put their artwork on the far side of its bound, which made it a decision rather
than a debt (ADR 1367), and a choice the standard leaves is not a departure (ADR 1622).
§12.6.4.6 did not hold: Table 207's `/F` names a document to open as well as an application to run,
and the half that names a PDF is built (ADRs 1368, 1358).

### Expired premises — a decision whose factual ground the tree has since removed

A seventh shape, and it is not a bucket of ledger rows: these are *decisions* whose stated premise was
a fact about this tree, and the fact has changed. Each is re-tested against the code it names rather
than against its own words (ADR 1201's method), and what stands here is the build the expired premise
no longer blocks. A round takes one of these the way it takes a ledger row; the ADR that recorded the
premise is a record and stays as written.

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
classifier reads a function's code and skips its `//` lines, so a helper whose only mention of
`doc/pdf.js` is a comment leaves the tests calling it fixtures (`ledger.rs`'s `a_corpus_root_in_a_comment_reads_no_corpus`).
