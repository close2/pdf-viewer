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
presentations. **What builds a row here:** the host work of `doc/todo/30`–`38` and RFC 0004's
print path. A new host reopens the entries it has not yet connected, even when the shared core
already reads them. Every window with a print dialogue presets Table 147's five dialogue entries, the
tray box among them (ADR 2919), and the WinUI window's print dialogue is `doc/todo/30`'s N4.

- **Beside this bucket and not in it** — §13.2.4.2 is `implemented`: its clip is judged by §13.2.2,
  Table 286's /TF asked at the reader's restriction level for `Operation::HandOverMedia`, and the data
  handed to the host as `viewer_core::Event::Media` (ADRs 1960, 1961). The host's own level for
  *starting* this machine's player, `--media-files=refuse|ask|warn|open` (`ask` by default, A254), is
  in every Linux window's menu, and the hand-off goes through the XDG desktop portal (ADRs 3201,
  3202). What is left owes no row: the WinUI window's opener, which declines by name until built.
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

§12.8.3.4.4 was here, and is `implemented`, with §12.8.3.4, §12.8.3 and §12.8 above it: a C caller's
fetched policy copy is handed to the document's validation by `quorra_policy_copy` (ADR 3169), a
distinguished name's values compare after RFC 4518's whole preparation (ADR 3170), and what its note
still names is a limit or the reader's own choice not to fetch (ADR 3171).

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

**Last run on 2026-11-16**: the same answer — `ed448-goldilocks` `0.14.0-pre.15`, no
`bp512`, no description naming an audit. **Run on 2026-10-05**: `ed448-goldilocks` `0.14.0-pre.15` (0.9.0 its newest stable), no `bp512`
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

- §12.10.4 — a projected system whose method is past the eighteen built is refused by its number and
  the method's name: 64 of the carried EPSG table's 5780 projected systems, over 29 methods, which
  `doc/todo/79` orders (ADRs 3821 to 3823, the owner's `doc/questions/A1095`). Each built moves the row.

### 3. Hard rendering / architecture — a real build across several rounds

The bucket is for a genuine model or rasteriser gap that only a build across several rounds closes;
the last rendering one, §10.7.4's mark for a stroke its matrix collapses, was built (ADR 1360).

 *One raster carries the product of shape and opacity where the clause wants the pair* is not
among them: §11.4.6 is the one reader of the pair the standard has, every knockout element states its
shape, and §11.3.7.3's NOTE 2 licenses the product everywhere else (§11.4.3's row, ADR 1340).

**Beside this bucket and not in it**: §11.6.6 and §11.7.2 are `implemented`, and §11.5.3 is `departed`
from its colorimetric branch alone by the owner's answer A23, every mask group weighed by the device
branch (ADR 2552), its rest implemented: every distinct four-component profile a page names gets its press, charged to the page's memory bound with its
display list, and a page whose presses do not fit is refused the one that crosses it, so its elements
are painted in the parent's and `PagePress::Beyond` names why (ADR 2532; ADR 1254 reads what one
profile is worth, Table 69's four intents against §8.6.5.9's `/UseBlackPtComp` less the pair that
clause forbids). §11.5.3's blend mode inside a subtractive group of more than one component is composited in
the group's four components (ADR 1342). §11.3.4 is `implemented`, its route into a one-component
blending space a choice §10.4.2.1's ranking leaves to the processor (ADRs 0790, 1622), which
`doc/todo/23` still prices; the precision that stood beside it here is closed, the cube into a parent's components being carried as
the device's decoding, a linear grid and the space's own encoding rather than as one sampled grid
(ADR 1267).

**Beside this bucket and not in it**: §10.7.4 is `departed` on three sentences — anti-aliasing,
the area it costs and a reduced image's average, departures §10.7.1's NOTE describes and does not
license (ADR 1560), none lifted by the folding stroke's exact measure (ADR 2331) — and on one shape,
a subpath that retraces itself off any one line (ADR 2330). Everything else it states is executed: a
point paints the pixel under it and a subpath on a line across the axes the band along it, for a fill,
a clip and a matrix of rank zero (ADR 2330, which also moved §8.5.3.3.1 to `implemented`); a fill or
a stroke its matrix carries onto a line is that line (ADRs 1348, 1360); a path whose subpaths overlap
is measured as the set its rule declares inside (ADRs 1341, 1347). No contradicted verdict the oracle
holds rests on any departure (ADR 1622); §10.7 is `implemented` with it. §10.7.5 is `implemented`:
both of its requirements are drawn under `/SA` in every backend, the grid fit by default as the owner
chose (`doc/questions/A714`, ADRs 2530 and 2553).

**Beside this bucket and not in it**: §8.7.4.5.7 and §8.7.4.5.8 are `implemented`. Every patch
travels to the backend and the fineness is derived there in device pixels (ADR 1217); a patch whose
colours §8.7.4.4 requires be converted between its corners carries them at a lattice of its unit
square, converted in `pdf-colour` to §10.7.3's tolerance (ADR 2546).

### 4. Feature depth — a built feature with cases still owed

The feature draws; the residue is a case the first build did not reach. **What would unblock them:**
a normal round extending the existing code.

The rich text build's last residue, pair kerning in §9.6.2.2's fourteen, is laid out from Adobe's
published pairs (`doc/questions/A308`, ADR 1916).

- §13.2.5 — Table 291's /F 3, "A scrolling user interface shall be provided": no window provides
  one over a screen annotation, so it is non-viable in MH and set aside in BE; the other values fit
  the picture (ADR 3553).
- §13.7.2.3.5 — Table 344's /Snapshot, "shall be displayed in place of the current artwork": read
  and said, not displayed, until the players tell the core when a clip associated with an
  annotation leaves their hands, Table 336's `XD` (ADR 3881, `doc/todo/80` section 1 step 4).

§13.4 is `implemented`: a negative `/Rate` and `Palindrome` play backward (ADRs 3837, 3841 to
3843), and Table 307's `/Synchronous` holds the window until the movie is completed or dismissed
(ADR 3853).

§13.2.6.2 and §13.2.6.3 are `implemented`: `pdf_model::media::judge` reads an offset and
its timespan, a section's `/B` and `/E` and a `BE` media duration's `/T` alike, and an unrecognised Table 296
or Table 300 `/S` makes "the rendition" non-viable from either dictionary (ADRs 2447, 2452, 2472, 2473).

- **Beside this bucket and not in it** — §12.5.6.23 is `departed` on two refusals with their cost
  recorded, a `JPXDecode` component past the twenty-four bits the decoder reads exactly (ADRs 1371,
  2470) and a two-input calculator function under a function-based shading the region meets (ADR
  2528), a one-input calculator function serving region-only colours being served by a stitching
  function written over the producer's program (ADR 2528); a component of
  seventeen to twenty-four bits, or one stating a sign, is written back as JPEG 2000 by `pdf-jpx-write`,
  as a JPX file where the image has other than one or three colour components, refused where a raw codestream of two, five or more states no colour box (ADRs 2480, 2481); every other content class the clause reaches is removed, and
  Table 195's overlay is a choice its own `should` leaves (ADRs 1124, 1622).

### 5. Built — the public-key security handler

**One cluster, ordered built by the owner (`doc/questions/A800`, which lifted `doc/questions/A66`'s
trigger) and built (ADRs 2801, 2802, 2809, 2810, 2821, 2822, 3157).** `crates/pdf-pki` holds the one ASN.1 reader, the X.509
reader, the big-integer seam, RFC 5652 `EnvelopedData` under all seven of §7.6.5.3's ciphers, RFC
8017's RSA decryption and RFC 5753's elliptic-curve key agreement, below both `pdf-syntax` and `pdf-signature`, with fuzz targets beside it.
`crates/pdf-syntax/src/crypt/public_key.rs` scans the recipient lists, opens the one naming the reader's
certificate, digests §7.6.5.3's key, and derives a key of its own for a crypt filter with Table 27's
per-stream recipient list. `crates/pdf-syntax/tests/public_key_witnesses.rs` opens `doc/questions/A168`'s
four `PDFBox` witnesses with their keystores' keys, and documents a script writes test s3, s4 and the
per-stream list. A host supplies the reader's certificate and key (`--recipient-certificate`,
`--recipient-key`, `quorra_recipient_key`; ADR 2822), so ADR 2801's order is built. RSAES-OAEP, the
Triple DES key wrap and 1-Pass ECMQV run (ADRs 2826, 2827, 2904), and key agreement runs on all six of
ISO/TS 32002 Table 3's curves, brainpoolP512r1 on the tree's own curve module that verification
already used (ADR 3157). §7.6.5.1, §7.6.5.2, §7.6.5.3 and §7.6.6 left the map: the four rows are
`implemented`.

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
§12.10.2 paragraph in bucket 6, which A436's datum transformation settles), A192 closes the Ed448 search (bucket 2's curve paragraph), A193 makes
JavaScript a build stream whose Tier 0 runs the one-call `AF*` scripts (ADRs 1578, 1579) and whose
Tier 1 engine runs in every window at the reader's `Scripts` level, `off` by default (ADRs 1590,
1591, 1616), A227's `zune-jpeg` fork is the one the manifest pins, carrying
both patches (ADRs 1589, 1730), and A209 owes nothing; none of the last four moves a row this map holds.
Of the answers of 2026-10-08, A286 amended `CLAUDE.md`'s script exclusion with `doc/questions/Q286`'s
sentence and moved §12.6.4.17 to `implemented`, an outline item's `/A` the site its build added
(ADR 1904); the row was never this map's. A254 amended the clause 13 entry with RFC 0009 section 11's
question 2 sentence, which moved §13.4, §13.6.2, §13.7.2.1 and §12.5.6.25 to `implemented` and
brought §13.2.2, §13.2.4.2 and §14.9.2.4 into this map as owed (ADR 1946), and RFC 0009 section 12's
item 3 took all three out of it again (ADR 1960).

### 6. Genuinely buildable now — the campaign's next targets

A normal round can advance or close each of these today; there is no missing surface, no unheld
package, no cross-round architecture. Membership is re-derived from the ledger rather than carried: a
row is in this bucket when its note names none of those three.

§12.2 was here, and is `departed` again on `/HideMenubar` alone, the reading ADR 1622 gave it: Table
147's `/Enforce` is a restriction at the reader's four levels (`doc/questions/A834`, ADR 2840), and where
the level holds the print scaling every dialogue that offers one locks it — `quorra-qt`'s control,
GTK's own dialogue by its scale capability and the job's scale, and a C caller's through
`quorra_owed_print_scaling` and `quorra_event_printing_scaling` (ADR 2908).

§9.9.2 was here, and is `implemented`, with §9.9 above it: a save names every subset it constructs or
imports whole from an FDF, and an archive conversion every one it embeds or constructs, through one
`variable_text::SubsetNames` against the file's own names, and a subset retagged is retagged in its
program wherever the format states the name there (ADRs 2832, 2838, 2862).

§7.4.9 was here and in the unheld-text bucket, and is `departed` on CIEJab (19) alone, whose CIE
Publication 131 the owner declines to buy (A348): sYCC is drawn by PIMA 7667's Annex C, and a writer is
refused only an sYCC image with a colour outside sRGB's gamut (ADRs 2793, 2863). §7.4 above it is
`implemented` with it; ROMM-RGB stays drawn by the codec, PIMA 7666 declined (A825).

§13.2.2 and §14.9.2.4 were here, and are `implemented`: `pdf_model::media` judges a rendition's
viability by the clause's rules for the one player this program has — the hand-off, told nothing but
the file — and the report's first sentence for a clip is its /Alt, chosen for the reader's language by
§14.9.2.4's search (ADR 1960).

§7.10.2 is `implemented`: Table 39's `/Order 3` is the not-a-knot cubic spline, the choice ADR 1636
writes down against the clause's own four-sample threshold, and §7.10 settles with it.

§12.10.2 was here, and is `departed`: a `/DCS` on another datum is reached through Guidance Note
7-2's Helmert transformation where the WKT states the parameters — the older form's `TOWGS84` or
ISO 19162's bound CRS — and refused by both datums' names where it states none, on `doc/questions/A436` (ADR 1958); the one sentence
departed from is Table 269's eastings and northings, read as the base system's degrees on
`doc/questions/A271` (ADR 1918). §12.10 above it is `implemented` with it.

§13.4 is not here: with video opened it is built and `implemented`
(ADRs 3545, 3818): the movie plays in place, told Table 307 (ADRs 3594, 3595) and turned by Table 306's
`/Rotate` (ADR 3817), Table 306's boolean poster is the movie's first picture decoded beside the window and drawn in
`/Rect` until it plays (ADR 3609), and its stream `/Poster` is drawn in `/Rect` at a placement this program chose (ADR
1561).

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

§12.10 — over the projected coordinate system row, the projection methods past the thirteen (ADR 3823).

§13.2 — over the media play parameters row, Table 291's /F 3's scrolling interface (ADR 3553).

§13.7, §13.7.2 and §13.7.2.3 — over the view dictionary row, Table 344's snapshot (ADR 3881).

The three before it, §12.6, §12.6.4 and §12.6.4.14, are `implemented`, the window holding the
association a rendition's stop, pause and resume act through (ADR 3433).


### Not owed — a documented choice, an exclusion, a deprecation, or a standard-gap

The residue here is not fresh implementation work: the clause hands the feature to a project
exclusion, deprecates it, states no artwork, or the case is one the standard leaves undefined and this
tree reports rather than guesses. **This is the bucket principle 5 says decays** — a *not owed* claim
is a claim about the specification, so a revisiting round re-reads the titles around the clause before
trusting the word (the DeviceCMYK and transfer-function precedents in `CLAUDE.md`). A row that
arrives here is read against its clause and against Errata Collection 3, and its note records the
reading.


The bucket was otherwise emptied by re-reading it rather than by building.
§12.6.4.9's and §12.6.4.10's own opening sentence hands them to 13.2, whose clause 13 entry admits
the media file handed over, so both are `implemented` with their files crossing as §12.6.4.14's
rendition clip does and the playing excluded (`doc/questions/A631`, ADR 2382).
§12.5.6.11's caret and §12.5.6.12's stamp legends are `implemented` with the choice named:
`doc/questions/A72` put their artwork on the far side of its bound, which made it a decision rather
than a debt (ADR 1367), and a choice the standard leaves is not a departure (ADR 1622).
§12.6.4.6 did not hold: Table 207's `/F` names a document to open as well as an application to run,
and the half that names a PDF is built (ADRs 1368, 1358).
Every `inapplicable` row was read against its clause once no row was `partial` (ADR 3177). §14.8.5.4.1
did not hold: its owner sentence decides what a layout attribute is, and the accessibility tree reads
layout attributes, so it is `implemented`. The rest hold on their clauses' own words — §10.6.1's
continuous-tone device, §14.2's PostScript device, §3.15's *deprecated* for web capture and four
prepress rows, §14.12.4.2's production workflow — and §14.8.3.2, §14.8.5.4.6 and §14.8.5.4.7 serve a
reflowing view, which `doc/questions/A634` keeps out.

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
