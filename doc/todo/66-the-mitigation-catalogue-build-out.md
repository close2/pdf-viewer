# 66 — The mitigation catalogue, site by site: the remedies a profile names and this version does not carry out

Status: **open**, written in the round after 1156 when the owner asked where the PDF/A converter
stood and whether the "as-if-printed" profile worked. A 20-band item (a real document asks and the
converter refuses) numbered past its band because the band is full.
Priority: **high** — it is the bulk of RFC 0007, which its own section 4.7.5 says: *the catalogue
is the bulk of this feature, not the mechanism*. The mechanism is built (the format, the sites, the
departures, the tool executor, the appended page, the relocation); what a profile answers is only
carried out at the sites that have code behind the answer.
Companions: `doc/rfc/0007` (the design), `doc/pdf-a-mitigations.md` (the catalogue: 118
requirements, four answers each, and the 29 whose honest answer is *none*), `doc/profiles/` (the
six shipped profiles), `doc/todo/62` (the exemption), `doc/questions/A54`–`A60`.
Corpus: the archive corpus walk, `cargo test --profile gates -p pdf-transform --test archive_corpus
-- --ignored --nocapture` under the heavy-walk lock, prints per target how many documents convert
by default and with everything authorised, and the ten sites that refuse most.

## The instrument, and the sentence that names the gap

```sh
quorra-transform archive --remedy-sites --to 2b      # every site the target binds, and what this version carries out
quorra-transform archive --to 2b --config doc/profiles/as-if-printed.toml -o out.pdf in.pdf
```

The second prints, for each answer the profile gives at a site without code behind it:

> note: the configuration answers "<site>" with `discard`, which this version does not carry out
> yet; that site stays refused with the sentence it names

**The item is done when a shipped profile produces no such note at any target**, and **two
milestones are reached**: `only-metadata-loss` and `as-if-printed` each answer every site they name
at PDF/A-2b, 2u and 2a with a remedy this version carries out. `tools/state.sh remedies` is what says so, and what says
which targets each of the other profiles still owes. Both halves of
that condition are now instruments rather than numbers in this file. `--remedy-sites` prints its own
two totals — `N of M sites not built yet`, and, given `--config <file>`, every answer that profile
gives which this version does not carry out and `N of M sites answered with a remedy not carried out
yet` — and `tools/state.sh remedies` filters those two sentences per target and per shipped profile.
It runs in `quick`; `section_archive` calls it, so one section prints both halves.

**`keep-everything` at the four targets holding no file** states `stop` where its only way of keeping
something is an attachment — six rows, each qualified to 4f and 4e for the attachment — because ISO
19005-2 section 6.8 and ISO 19005-4 section 6.9 admit an embedded file only where it is itself
PDF/A (ADR 1285). What it still answers there with a remedy this version does not carry out is what
`--remedy-sites --to 2b --config doc/profiles/keep-everything.toml` prints, and not a number here.

**The profile half needs no corpus, which is a property of the question rather than a shortcut.**
The note comes from `Configuration::unbuilt`, which reads the answers a profile gives and asks which
of them have code behind them; both are properties of the profile and the target, so a conversion
prints the same notes whatever document it is handed, and walking a corpus for them would count the
corpus. The corpus question is a different one and is `section_archive`'s: how many documents
convert, by default and with everything authorised, and which sites refuse most.

## What each family needs, so that a round can take one

Grouped by the code one build unlocks, not by clause; every site's own entry in
`doc/pdf-a-mitigations.md` carries the argument and the per-target answer.

- **Actions** (`actions/*`, eight sites): **built** — `crates/pdf-transform/src/archive/actions.rs`,
  under one `Loss::InteractiveBehaviour`, with `forms/no-action-on-widget-or-field` beside them
  because one routine answers all nine. A removed action's §12.6.2 `/Next` subtree is promoted into
  its place, so the permitted actions behind a forbidden one still run (ADR 1175).
- **Forms** (`forms/*`, four): **built** — widget actions with the Actions routine above,
  `/NeedAppearances` cleared with the field appearances constructed first, and the `/XFA` removed
  from a form the document does not itself say is dynamic (ADR 1257). §12.7.3's Table 224 makes
  clearing the flag a claim that streams have been provided, so the construction is what makes the
  claim true and a widget whose appearance cannot be built keeps the flag with its field named;
  §7.7.2's Table 29 is the dynamic predicate, because Annex K states no test. `keep-xfa =
  "attach"` is **built** too (ADR 1270), on the attachment machinery `original = "attach"` uses:
  Annex K says the array form's packets are a string and a stream each, the first and last carrying
  the `xdp:xdp` tags, so the streams end to end are the resource. Nothing is left in this family.
- **Metadata** (`metadata/*`, eight): **built but for one mechanism.** The fresh packet is written
  where this tree cannot read the producer's, with the original laid out on an appended page
  (ADR 1245, catalogue section 9; the owner's *append or prefix the packet as a page*); the
  extension schema container is written from what the packet itself states, with one fixed sentence
  in each of the three fields no file holds (ADR 1245); the malformed amendment identifier is cut
  by span (ADR 1246). The appended page carries the structure entries it owes a tagged document
  (ADRs 1025, 1163). `original = "attach"` is **built**
  (ADR 1270): the producer's packet stays as an embedded file at 4f and 4e, filed in §7.7.4's name
  tree with §14.13.3's catalog `/AF` beside it and §14.13.2's own `application/octet-stream` as its
  media type — the clause names that value for a type the writer does not know. What is left in
  this family is `metadata/provenance-recorded-action-fields-four`, whose catalogue entry
  recommends a departure over every remedy.
- **The document information dictionary** (`file-structure/document-information-dictionary-*`,
  two sites at part 4 only): **built** (ADR 1269). §14.3.3 deprecates the dictionary and Table
  349's NOTEs name the XMP counterpart of every key, so the values move and the dictionary goes;
  `pdf_model::xmp::supplement` is additive because §14.3.4 permits an addition only into a silence.
  A catalog stating a `/PieceInfo` keeps `/Info` holding `/ModDate` alone, which is the clause's
  own carve-out and §14.5's reason for it. `unmapped = "discard"` drops a key Table 349 names
  nothing for; `unmapped = "extension-schema"` is recognised and **refused**, because a container
  describes a schema a packet uses and a `/Info` key is in none — putting one in needs a namespace
  URI no file states, which is the property's identity rather than a label for it.
- **File structure and encryption** (`file-structure/no-encryption`, `crypt-filter-is-identity`,
  `permissions-dictionary-keys`): **built** — `Loss::Encryption`, the word `encryption`, with
  `crates/pdf-transform/src/archive/protection.rs` carrying the producer's Table 22 flags into the
  report and the output's own `xmpMM:History` (catalogue item 5, ADR 1187). The third row was
  already `Mechanical` (ADR 1007).
- **Colour** (`graphics/separations-of-one-name-agree`): **built** as a `supply` —
  `winner = "first" | "most-used"` names which of the file's own definitions of an ink the archive
  keeps, so every byte written is the producer's (ADR 1188). What is left in this family is
  `no-overprint-mode-one-under-icc-cmyk`'s `discard`.
- **External stream data** (`file-structure/no-external-stream-data`): **built** for the bytes a
  caller can resolve — `crates/pdf-transform/src/archive/external.rs`, ADR 1199. The conversion
  names what it needs in `Conversion::external_data`, `quorra-transform archive
  --resolve-external-data` reads a plain file name beside the document under ADR 1155's rule, the
  plan carries the bytes, and §7.3.8.2's Table 5 is the rewrite; a stream stating the filter keys
  and no `/F` needs nothing from outside and is `Mechanical` on its own. The
  `tool = "resolve-external"` route — what §7.11.5's URL needs and what every corpus witness turns
  out to be, the archive sweep counting per target how each such stream names its data — is
  **built** (ADR 1209): a `preserve` row naming the site and a tool returns one request per stream
  carrying the document's own file specification, and what the caller's executor gets back lands
  in the same `ArchivePlan::external_data`. The enumerability wrinkle ADR 1199 left is closed with
  it: a `Mechanical` answer the decision table does not settle by itself is
  `census::Kind::Conditional`, which `--remedy-sites` lists and describes.
- **Graphics state keys** (`graphics/no-transfer-function-*`, `no-halftone-*`,
  `second-transfer-function-is-default`, `rendering-intent-*`): a key removed from an `ExtGState`
  or a halftone dictionary, each a `discard`; at 4f the sampled function may be attached.
- **Annotations** (`printable-and-visible`, `appearance-dictionary-holds-only-normal`): **built** as
  two authorised losses (ADR 1234). The `discard` at the flag site removes the annotation — §12.5.3's
  Table 167 offers no fallback and `doc/pdf-a-conversion-limits.md` section 3.7 makes removal the
  default of its two futures — and the `discard` at the appearance site reduces the `/AP` to `/N`,
  which §12.5.5's Table 170 already makes what a reader draws in the other two states. `preserve` at
  the flag site is **built** too (ADR 1285): the word alone is the mechanism, the annotation stays
  and its `/F` becomes `pdf_archive::flags_permitting`'s value, and the report names every
  annotation shown with the flags it had.
- **Reference XObjects** (`graphics/no-reference-xobjects`): **built** as `preserve` (ADR 1285) —
  §8.10.4.1's proxy is what an archive's reader draws anyway, so the `Ref` entry goes, the form
  stays, and the report names the file and page it pointed at.
- **CMaps** (three sites): `fonts/cmap-embedded-or-predefined` is **built** as `preserve` with
  `source = "shipped-cmaps"` (ADR 1286) — Adobe's published program for the name, byte for byte,
  refused by name where it builds on a CMap off the list or describes another collection than the
  font's; `fonts/embedded-cmap-states-its-own-write-mode` is **built** as `supply` with
  `write-mode = "program" | "stream"`. `fonts/cmap-uses-only-predefined-cmaps` **cannot take** the
  shipped-CMap answer — embedding the CMap a chain names leaves the reference to it — and the
  configuration refuses that answer by name.
- **Optional content** (two sites): `/AS` removed is **built** (ADR 1234), a part 2 target's loss
  alone because ISO 19005-4 section 6.10 keeps the key and has a processor ignore it. Configuration
  names are what is left, and the catalogue's entry makes them a `supply` with the converter as the
  supplier — a different argument, and one no shipped profile answers yet.
- **Embedded files** (four): the media-type `supply` and the `derive` were already built, and the
  `discard` is **built** (ADR 1258) — every reference to the file specification goes, the name tree
  and the `/AF` arrays with it, and the report names each file that left. What is left is appending
  the attachment's pages, whose blocker is now only the page composition, and the 4f `preserve`
  (catalogue section 1.2) for anything a byte string can hold.
- **Fonts, content-stream marks**: the catalogue's *none* — leave them, and say so; ADR 0816's
  fence is the reason, and ADR 1124's content-stream splice is the precedent to cite if a later
  round argues the fence should move. Two have a narrower reading than the family's:
  `fonts/cid-system-info-agrees-with-the-cmap` keeps its *none* for the file whose CMap and font
  genuinely belong to different collections, and the catalogue entry names the buildable part —
  correcting a `/CIDSystemInfo` **from the program it describes**, which §9.7.4.2 makes a copy —
  with the two readers it needs; `fonts/embedded-programs-define-every-glyph-shown` and
  `fonts/no-notdef-glyph-shown` hold their *none* on a re-reading of both clauses (ADR 1200), and
  the difference between them is ISO 19005-2 section 6.2.11.8's *regardless of text rendering
  mode*, which its neighbour's NOTE 2 does not say.
- **Fonts, the supply this tree describes**: `--font <base-font>=<path>` is **built** (ADR 1209),
  on ADR 1200 section 4's reading — the licence ISO 19005-2 section 6.2.11.4.1 demands is a fact
  only an operator can state, which §9.9.1 says outright, so the flag is `doc/rfc/0007`'s `supply`
  in its oldest form. The program is carried in the plan, `pdf_font::restate` makes its advances
  the numbers the dictionary already states so no glyph moves, and the report and the output's own
  `xmpMM:History` record whose authority the face was in. **Every pairing §9.9's Table 124 states is
  now written** (ADRs 1221 and 1222): the `OpenType` row's third bullet puts an sfnt carrying a
  name-keyed `CFF ` table under a `Type1` dictionary as `/FontFile3` with `/Subtype /OpenType`, and
  a composite font's program goes into the descendant CIDFont's descriptor, where §9.7.4.2 puts it,
  restated against §9.7.4.3's `/W` and `/DW`, with Table 115's `/CIDToGIDMap` `Identity` written
  beside it. **Which corpus documents refuse at this site and for which of the several reasons is
  printed by the archive sweep itself**, because a count alone does not rank a site that refuses
  for more than one. What `--font` still does not reach, each refused with the clause that refuses
  it: a `glyf`-based face under a `Type1` or `MMType1` dictionary, which Table 124 opens no key to
  at all; a CMap that is neither `Identity-H` nor `Identity-V`; a CID-keyed program, whose own
  character collection this converter may not copy into `/CIDSystemInfo`; a `/CIDToGIDMap` the
  producer wrote beside a program that was never in the file; and a font stating no
  `/FontDescriptor`, which `--font` names no descriptor for.
- **Implementation limits**: *none* for nine of the ten, and **not** for
  `implementation-limits/page-boundary-sizes` — §7.7.3.3's Table 31 makes four of §14.11.2's five
  boxes optional and §14.11.2.1 gives each a default that is another box in the file, so removing
  an out-of-range optional entry moves no mark. ADR 1200 section 1 has the predicate that decides
  whether the removal is mechanical or a loss; **built** in ADR 1210, with
  `--authorise page-boundary` for the second case and the media box keeping its refusal.

What every site needs alike: the answer carried out at rewrite, the report naming what left and
where it went, a fixture per site in `crates/pdf-transform/tests/archive.rs`, the output validated
clean, and the mitigation catalogue's entry updated from *catalogued* to *built*.
