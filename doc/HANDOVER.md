# Handover

Written 2026-07-26, updated 2026-08-01 at the end of the **hundred-and-nineteenth** working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles, what *done* means, and the
closed list of exclusions. **Principle 5 is the one that changes how to work**: the specification
is the only source of truth, and agreement with poppler, mupdf or pdf.js is evidence that we read
it right, never the definition of right. `doc/PLAN.md` holds the phases and the conformance
ledger's design; `doc/adr/` holds every decision's argument. **This file is only the state of
play, the traps, and what to do next** — where something is also written there, this is a pointer.

Each session's own reasoning lives in its ADR, and every session is one line in "How the project
got here". This file keeps a lesson **exactly once**: in a trap if it changes how you write code,
in "Habits" if it changes how you work, and in the numbers if it is a fact about today. It was
halved in the fifty-ninth session by deleting twenty per-session narratives that duplicated those
three homes, and cut back to three prose entries in the seventy-ninth; the ten sessions since have
kept to that rule by dropping the oldest entry as each new one arrived. If you find yourself
retelling a session here, the retelling belongs in its ADR and the lesson belongs in Traps or
Habits.

## What the last three sessions changed

Every session before these is one line in the table below, with its argument in its ADR. Three
are kept in prose because their findings are recent enough to still be acted on.

**Hundred-and-seventeenth — a pattern is a group, and the alpha belongs to it.** ADR 0107.

- **§11.6.7's implicit transparency group is built for a tiling pattern.** Every cell now runs
  with the blend mode, alpha constant and soft mask at their *defaults*, which the clause
  requires in as many words, and the state's own are applied once to the finished tiling by one
  group over all the tiles — NOTE 2's own advice.
- **Two consequences, and only one was written down.** An `0.5 ca` was applied per mark, so a
  cell with two overlapping shapes reached 0.75 where the clause reaches 0.5; and the graphics
  state's **soft mask reached a tiling pattern not at all**, because nothing copied it onto the
  cell and nothing applied it to the result.
- **Where all three are at their defaults no group is built** (§11.4.4's NOTE 5), which is all
  **122** tiling paints in the corpus — counted — so no page witnesses the change and both gates
  are unmoved. Two tests carry it, and the discriminating one asserts the *alpha*.
- **The note's reason had expired.** It said this was "the closest available approximation while
  §11.4.6 does not exist"; §11.4.6 was drawn in the seventy-first session. **A note that gives a
  reason gives a trigger, and nothing fires it.**

Tests 871 → 873; no gate moved.

**Hundred-and-eighteenth — a reason that expired, and the action it was blocking.** ADR 0108.

- **The sweep the previous session's lesson asked for**, and it cost twenty minutes: one regular
  expression over the ledger's 823 notes for `while … does not exist`, `until …`, `needs §…`.
  Sixty-two matches, most of them history; **three live**.
- **§12.6.4.5's `GoToDp` was refused because §14.12 was `unreviewed`** — which it has not been
  since the fifty-sixth session. It is now performed: Table 206's `/Dp` names a `DPart`
  dictionary and §12.6.4.5 says the action "changes the view to the Start page" of it. **Eleven
  of §12.6's actions.**
- **§14.12's `inapplicable` had decayed, for the second time in this project's history.** A
  `DPart` dictionary decides *which page is shown*, which no marking clause does — so
  `document_part.rs` reads §14.12.3's depth-first ordering and Table 409's `/Start`, and nothing
  else. The rest of the family stays `inapplicable`.
- **§12.7.8.3.1 said `/Pages` "needs §12.7.7's named pages"**, seventeen sessions after they
  landed and while `read_pages` applied it.
- **The class no gate can watch is a row that names a *blocker* rather than a gap**: true when
  written, false when the blocker lands, and nothing re-reads it because nothing changed in its
  own clause.

`reported` falls **36 → 35**; tests 873 → 876; no gate moved.

**Hundred-and-nineteenth — everything re-verified after nine sessions of change.** No ADR: a
verification session that finds nothing has nothing to argue.

- **Every gate re-run rather than inherited.** 876 tests, `clippy` silent under `pedantic` +
  `unwrap_used`/`panic`/`arithmetic_side_effects`, `cargo fmt --check` clean, `cargo deny` clean
  on all four checks, **all five fuzz targets clean at 50 000 runs apiece**, and the four
  censuses unmoved: 90 incomplete of 974, 840 agreeing and 65 contradicted of 1666, 97.9% of
  `pdftotext`'s words, 1545 dates.
- **The two performance numbers are at their drift floors and are quoted as such.**
  Interpretation is **2 119.5 M** against the 2 110.6 M recorded in the hundred-and-sixth
  session — +0.42%, which is exactly the floor this file measured for *the same commit rebuilt*.
  Against `hayro`: our total **7.08 s over 864 complete pages** against 6.99 s over 862, on an
  afternoon when `hayro`'s own total moved 39.59 s → 41.28 s with nothing here touching it.
  Median 2.13× against 2.14×.
- **Nine sessions, and not one moved a gate**, which is the honest shape of what they were:
  four ledger audits, two corrections of a refusal that was drawing nothing where a clause
  states something, one clause read whole, one implicit group, one action. Every one of them
  was measured on the corpus and every one of them was found by *reading*, not by a picture.

## How the project got here

One line per session; the argument is in the ADR, and every durable lesson is in Traps or Habits
below rather than here.

| Session | What landed | Where the reasoning is |
|---|---|---|
| 5 | The reference oracle, over every page of the corpus | ADR 0011 |
| 6 | `CalGray`/`CalRGB` through XYZ; annotation appearance streams | ADRs 0012, 0013 |
| 7 | JBIG2 and JPEG 2000, in a sandboxed worker; the first speed comparison | ADR 0014 |
| 8 | §9.6.5.4, the `TrueType` code-to-glyph algorithm, in full | ADR 0015 |
| 9 | The conformance ledger and citation checker; optional content | ADRs 0016, 0017 |
| 10 | Type 3 fonts; dashed lines, which had never been dashed | ADR 0018 |
| 11 | Inline images; `/Interpolate`; `Indexed`, `Separation` and `DeviceN` images | ADR 0019 |
| 12 | A cache for the oracle's reference renders; `CCITTFaxDecode`; `/Rotate` | ADRs 0020, 0021 |
| 13 | All eight text rendering modes; §9.3 and §9.4 reviewed; table numbers checked | ADR 0022 |
| 14 | `/Mask` in both its forms; §11.6.4 reviewed; §9.3.8 reports | ADR 0023 |
| 15 | Soft masks at any resolution and `/Matte`; §11.3.7, §11.5, §11.6 reviewed; a shading carries `ca` | ADR 0024 |
| 16 | Area averaging for reduced images; §10.7 reviewed, and it forbids what was built | ADR 0025 |
| 17 | Transparency groups; §8.10 and §11.4 reviewed; the page group is isolated | ADR 0026 |
| 18 | Soft masks in an `/ExtGState`; §11.7 reviewed, and overprinting is silent | ADR 0027 |
| 19 | `/SA` and the device's thinnest line; §8.6.6 and §8.6.7 reviewed, and overprinting is *not* a gap | ADR 0028 |
| 20 | Embedded `CMap`s and `/CIDToGIDMap`; the whole of §9.7 reviewed | ADR 0029 |
| 21 | Constructed annotation appearances; the whole of §12.5 reviewed; `/CA` belongs to the construction | ADR 0030 |
| 22 | Encryption, every revision and method §7.6 states; the whole of §7.6 reviewed; a locked file is not an unreadable one | ADR 0031 |
| 23 | §12.7.4.3's variable text; §12.7.4, §12.7.5 and §7.9.2 reviewed; regenerating an appearance is a splice | ADR 0032 |
| 24 | §8.5.3.2's degenerate strokes and zero-length dashes; the whole of §8.4 and §8.5 reviewed; an empty clipping path admits nothing | ADR 0033 |
| 25 | §8.9.5.2's `/Decode` array in full, Table 88 included; the whole of §8.6.5 reviewed; a fast path inherits no clauses | ADR 0034 |
| 26 | An image's colour space is a fill's; §8.6.4 reviewed; an exact memo where a lookup grid was the obvious answer | ADR 0035 |
| 27 | `LZWDecode`, the last standard filter; the whole of §7.4 reviewed; a corpus stating an invariant about itself | ADR 0036 |
| 28 | A shading's `/BBox`; the whole of §8.7 reviewed; a contradicted page's diagnosis refuted by measuring it | ADR 0037 |
| 29 | `/UserUnit`, and the geometry list emptied; the whole of §7.7 reviewed | ADR 0038 |
| 30 | An embedded program's own encoding is the base; `/MissingWidth` is 0; §9.6 and §9.8 reviewed | ADR 0039 |
| 31 | Bare Type 1 fonts (`/FontFile`); the oracle's tolerance class asks whether glyphs were drawn; §9.9 reviewed | ADR 0040 |
| 32 | All five bit depths; an inline image's abbreviated keys win; `BX`/`EX`; §7.8 and §7.3.7 reviewed | ADR 0041 |
| 33 | §10.4.2.5 exists; Table 57's `/Font`; the whole of clause 10 reviewed | ADR 0042 |
| 34 | §12.5.6.10's text markup appearances; §7.9 and §14.11 reviewed; `REVIEW_OWED` emptied | ADR 0043 |
| 35 | §8.11.4.4's usage application dictionaries — the ledger's last original `silent` row | ADR 0044 |
| 36 | Vertical writing: §9.2.4's second set of metrics, §9.7.4.3's `/W2` and `/DW2` | ADR 0045 |
| 37 | The blend-mode scene nobody had written; clause 11 completed as a review | ADR 0046 |
| 38 | Clause 8 completed as a review — the graphics clause, 20 rows | ADR 0046 |
| 39 | §11.3.5.3's four modes taken back from `tiny-skia`; §7.2 and §7.3 reviewed | ADR 0047 |
| 40 | Four unexplained pages are one shared ICC profile; §7.5 reviewed, and clause 7 is complete | ADR 0048 |
| 41 | A CID into a bare Type 1 program; §9.10 reviewed, and clause 9 is complete | ADR 0049 |
| 42 | A suffixed glyph name is the program's, not the AGL's; §14.1–§14.6 reviewed | ADR 0050 |
| 43 | One mesh raster instead of 4096 flat triangles; §12.3 reviewed, and `silent` is 15 | ADR 0051 |
| 44 | A font's own tables say which way round its offsets are; §12.1, §12.2 and §12.4 reviewed | ADR 0052 |
| 45 | A contradicted page's label measured and replaced; §12.6's actions reviewed | — |
| 46 | Clause 12 completed as a review; the median page profiled at last | — |
| 47 | A negative line width is a choice, written down; §14.7 reviewed | — |
| 48 | Name and number trees, and §12.4.2's page labels on top of them | ADR 0053 |
| 49 | §12.3.2's destinations, all three spellings; §7.11 and §7.12 reviewed, and clause 7 is complete for real | ADR 0054 |
| 50 | §12.3.3's outline, and a `/Count` the clause states as an algorithm; §14.13 reviewed | ADR 0055 |
| 51 | A tiling pattern's cell clipped to its `/BBox`, per cell; §14.12 reviewed | ADR 0056 |
| 52 | A pattern inside a form maps to the form's space; §14.9 reviewed | ADR 0057 |
| 53 | A glyph filled with a tiling pattern is tiled; §14.10 reviewed | ADR 0058 |
| 54 | A ramp that can hold a step; §14.8's page-content half reviewed | ADR 0059 |
| 55 | §14.9.4's `/ActualText`, and the property list that was never a dictionary; §14.8.4 reviewed | ADR 0060 |
| 56 | **The ledger reaches zero unreviewed rows**; §14.7.5.4's parent tree closes the last one | ADR 0061 |
| 57 | A click follows a link | ADR 0062 |
| 58 | Everything re-measured, and one feature's cost attributed | — |
| 59 | The corpus's own bug trackers read; a written conclusion corrected | — |
| 60 | §14.9's four accessibility entries, in both places each may sit | ADR 0063 |
| 61 | The page's top edge is raster row zero; 11 contradicted pages agree | ADR 0064 |
| 62 | §12.6's actions, and the third input a viewer has | ADR 0065 |
| 63 | A third gate: the text, over the whole corpus | ADR 0066 |
| 64 | §9.10.2's closing sentence is a permission, and three documents took it | ADR 0067 |
| 65 | A ramp is not a gradient: 144 G instructions become 54 G | ADR 0068 |
| 66 | §14.9 completed, and the text gate's remaining list is all naming | — |
| 67 | Table 99's layer-panel half: `/Order`, `/ListMode`, `/Locked` | — |
| 68 | Two contradicted pages are §10.7.4's own departure, measured | — |
| 69 | §12.4.4.2's sub-page navigation, on the control two sessions built | — |
| 70 | §12.4.4.1's page transitions, read from Table 164 and played by nobody | — |
| 71 | §11.4.6's knockout groups, drawn where a shape is a coverage | — |
| 72 | §9.3.8's text knockout and §11.6.2's one object in parts, on it | — |
| 73 | One shading object, built once: a fifth of the corpus's worst page | ADR 0069 |
| 74 | §10.7.3's `/SM`, the silence that was hiding inside a `partial` row | — |
| 75 | Eight "unexplained" contradicted pages are one population, measured | — |
| 76 | Table 170's rollover and down appearances, on the pointer | — |
| 77 | §12.6.3's trigger events, and the one precedence rule Table 197 states | — |
| 78 | §14.7.2's structure tree, read downwards at last | — |
| 79 | Everything re-verified, including the fuzzers and `cargo deny` | — |
| 80 | §12.6.4.8's URI resolved by RFC 3986, and §12.6.4.12's four page commands | ADR 0070 |
| 81 | §12.2's viewer preferences, and the two that decide which boundary is displayed | ADR 0071 |
| 82 | §14.7.6's attributes and §14.7.4's namespaces; the ledger's own prose checked | ADR 0072 |
| 83 | §14.8.2's artifacts and reversed show strings; an inline array nobody parsed | ADR 0073 |
| 84 | §7.12's extensions and §12.11's requirements: what a document says it needs | ADR 0074 |
| 85 | §12.5.6.7's leader lines, and the first corpus ratchet in six sessions | ADR 0075 |
| 86 | §7.11.4's embedded files, listed; everything re-measured | ADR 0076 |
| 87 | §14.13's associated files: one array in seven places | ADR 0077 |
| 88 | §14.8.4's forty-one standard structure types, and what a tag means | ADR 0078 |
| 89 | §14.8.5's owners and the five-step priority for an attribute's value | ADR 0079 |
| 90 | §12.4.3's articles and §12.6.4.7's thread action: a ring of beads nobody writes | ADR 0080 |
| 91 | §12.3.4's thumbnails, and the page as its own producer drew it | ADR 0081 |
| 92 | §12.9's viewports and §12.10's geospatial dictionaries, and a formatting algorithm | ADR 0082 |
| 93 | §12.3.5's collections, §12.3.6's navigators, and clause 7's last two silences | ADR 0083 |
| 94 | §14.8.2.5's logical content order, and what a tagged page owes its reader | ADR 0084 |
| 95 | §14.8.5.6's `PrintField` and §14.7.7's example: clause 14 reaches zero silences | ADR 0085 |
| 96 | §9.8.3's substitution hints, the ledger's oldest silences | ADR 0086 |
| 97 | §12.7.6.3's reset-form action, and nine refusals the viewer had swallowed | ADR 0087 |
| 98 | §12.8's signatures: what a renderer can say about one without a trust store | ADR 0088 |
| 99 | §12.8.4's store, §12.8.7's attestations, and everything re-verified | ADR 0089 |
| 100 | §12.7.8's forms data format read and §12.7.6.4's import performed | ADR 0090 |
| 101 | §12.7.7's named pages, and the ledger reaches zero silences | ADR 0091 |
| 102 | §7.9.4's dates audited over 1542 corpus strings; §8.9.5.4's alternate images | ADR 0092 |
| 103 | §11.4.4's NOTE 5: the non-isolated group that need not be built | ADR 0093 |
| 104 | §12.6.4.4's embedded go-to: the document inside the document | ADR 0094 |
| 105 | Table 192's `/R` drawn; §12.5.6.23's overlay read as writer-side | ADR 0095 |
| 106 | A fifth fuzz target for §12.7.8 and §7.9.4; everything re-verified | ADR 0096 |
| 107 | Two recovery rules from the file's own declarations: 11 pageless → 5 | ADR 0097 |
| 108 | §7.10 audited out of the file-only evidence population, and two claims corrected | ADR 0098 |
| 109 | §12.6.4.15's transition action, which is §12.4.4's table at a different moment | ADR 0099 |
| 110 | §7.5's free entries: an update that deletes an object is no longer undone | ADR 0100 |
| 111 | Clause 8's file-only rows audited; a retired claim found in four places | ADR 0101 |
| 112 | The file-only evidence population reaches zero, over four audits | ADR 0102 |
| 113 | A to-do item measured at a hundredth of its listed price, and removed | ADR 0103 |
| 114 | §7.11's file specifications, read whole and opened never | ADR 0104 |
| 115 | Five `reported` rows that understated what the tree already does | ADR 0105 |
| 116 | §12.5.6.7's `/LE` and `/Cap` named beside the line rather than instead of it | ADR 0106 |
| 117 | §11.6.7's implicit group: a pattern's alpha belongs to the pattern | ADR 0107 |
| 118 | §12.6.4.5's `GoToDp`, and a sweep for reasons that had expired | ADR 0108 |
| 119 | Everything re-verified: four gates, five fuzzers, both performance numbers | — |

**The two gate numbers, across the whole history.** Contradicted pages: 174 → 120 → 108 → 106 →
104 → 108 → 103 → 103 → 104 → 103 → 100 → 93 → 96 → 96 → 98 → 102 → 102 → 102 → 102 → 102 → 102
→ 102 → 101 → 101 → 99 → 88 over sessions 6 to 31, then 87 → 86 → 83 → 82 (steady to 50) → 81 →
78 → 77 → 76 (steady to 60) → **65** over 41 to 61, and **steady at 65 from 61 to 89** — where
four sessions of new features (§11.4.6, §9.3.8, §11.6.2, §12.5.6.7) added six pages to the
*judged* set without adding one to this list, so the agreeing count rose 836 → 837 while this one
did not move. Corpus documents drawing incompletely: 291 → 368 → 250 → 290 →
283 → 263 → 251 → 235 → 232 → 231 → 231 → 237 → 220 → 220 → 189 → 147 → 137 → 129 → 130 (steady)
→ 110 → 106 → 105 → 97 → 94 → 95 → 97 → 96 (steady to 70) → 95 → 90 over 71 and 72, and
**89** in the eighty-fifth.

Both move in both directions on purpose: a rise in the first can mean pages *joined* the
comparison, and a rise in the second is honesty when a silence ends. The sections below say
which.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images, shadings,
patterns, embedded text, transparency groups, soft masks, and annotations both from their stored
appearance streams and constructed where the standard states one — on a CPU and a GPU backend,
with JBIG2 and JPEG 2000 decoded in a confined worker, encrypted files decrypted at every
revision and method §7.6 states, **a form field's value laid out from its `/DA` string**, a page's
own label from §12.4.2 shown in the title bar, **§12.3.2's destinations**, which decide the page a
document opens at, **§12.3.3's outline**, which names the section the page being shown is in,
**§12.5.6.5's links, which a click follows** and which now perform **eleven actions** —
a layer turned off, an annotation hidden, a page turned, a URI resolved, an article thread
followed to its next bead, a form reset to the values the file says it starts at,
**§12.7.8's form data imported from a second file beside the document**, **a document
embedded inside this one opened and shown** and **the page a §14.12 document part begins at** —
and **§14.9's four
accessibility entries, which say what a page *says* rather than what it shows**, and which reads
§12.4.4's whole presentation — a page's transition, its advance timing and its own sub-page
states — for a caller that has a presentation mode to play. **Since the eightieth session it also
reads what a document says *about itself***: §14.7's logical structure entire, §14.8's tagged-PDF
vocabulary, §7.11.4's embedded files and §14.13's associated ones, §12.2's viewer preferences,
§12.11's requirements and §7.12's extensions. It is not yet a
PDF *viewer* in the full sense — nothing edits a field, asks a person for a password, speaks a
page or runs a slide show — and the gap is now measured *by clause* as well as by corpus: **not
one of the ledger's 823 rows is `silent`**, so nothing in the eight technical clauses is missing
without the tree saying so. What is owed is 35 `reported` rows and the notes on 240 `partial`
ones.

- **876 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks, and **all five fuzz targets
  clean at 50 000 runs apiece** — every one of those re-run in the hundred-and-nineteenth
  session rather than inherited. (The thirteenth session found this line had been *wrong*: eleven warnings had
  accumulated because `allow-panic-in-tests` does not reach an integration test's helper
  functions.)
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and 101 318
  objects — all parse, all render page one with **nothing reported at all**, and all extract
  **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open except ten that are
  encrypted — 8 waiting for a password, 2 by something §7.6 does not specify or we do not
  implement — **959 reach page one**, 869 draw with nothing reported, and everything the other 90
  cannot draw is named. 1501 of 1501 PDF functions parse; all 1793 shadings build, mesh types
  included. The whole gate runs in **~2 s** with no named slow document left. Counts are
  ratcheted.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against poppler,
  mupdf and ghostscript over **1794 pages** — every corpus page plus page one of each
  specification PDF — in **~35 s**, because the references' renders are remembered between runs
  (ADR 0020). Of the 1666 pages we claim to draw completely, **840 agree with the reference
  consensus, 65 are contradicted and 750 are pages the references cannot agree about among
  themselves**. The 65 are named, grouped and ratcheted in both directions. Nineteen pages
  do not rasterise at all: 7 documents that have no such page, 10 encrypted ones, and 2 whose
  target size is degenerate or past the pixel limit. **None is a page we decline to draw** —
  the last four of those left in the twenty-fourth session (ADR 0033). ADR 0011.
- **JBIG2 and JPEG 2000 decode in a sandboxed worker.** `pdf-sandbox` confines it with resource
  limits, Landlock and a seccomp-BPF allow-list; `--no-sandbox` turns it off for trusted documents
  and says what that costs. The strongest evidence the decode is right is not a reference
  renderer: the corpus encodes **one image ninety-six ways** and all ninety-six produce
  byte-identical pixels. ADR 0014.
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B evaluator
  written here, `CalGray`/`CalRGB`/`Lab` convert through XYZ, `/DefaultCMYK` and output intents
  are honoured, and there is exactly one route from XYZ to a pixel and one `DeviceCMYK`
  conversion. ADRs 0009, 0012.
- **A composite font is a `CMap` and a `CIDFont`, and both are read.** §9.7 in full except for
  data: an embedded `CMap` stream decides how many bytes each code takes and which CID it selects,
  byte by byte against its codespace ranges, with §9.7.6.3's recovery for a code that matches
  none; a CID reaches a glyph through a CID-keyed CFF's charset or a `/CIDToGIDMap` stream. What
  is left is Table 116's predefined `CMap`s (registered data files, so a licensing question) and
  vertical writing (§9.2.4's `/W2`). The parser is fuzzed on the property that matters: a `CMap`
  that consumed zero bytes per code would hang a page. ADR 0029.
- **An encrypted document is decrypted, and a locked one says so.** §7.6's standard security
  handler at revisions 2, 3, 4 and 6 over `/V` 1, 2, 4 and 5, with `V2`, `AESV2`, `AESV3` and
  `Identity`; every one of the clause's numbered algorithms is written out against its own
  subclause. `Document::open` tries the empty password §7.6.4.1 requires first and returns
  `PasswordRequired` when that fails, which is a *locked* file rather than an unreadable one.
  Refused by name: `/R 5`, which Table 21 says "shall not be used" and states no algorithm for;
  §7.6.5's public-key handlers; and a revision 4 password outside the range where PDFDocEncoding
  and Unicode provably agree. ADR 0031.
- **A glyph may be a content stream** — Type 3 fonts (§9.6.4), read in `pdf-model` because drawing
  one means running the interpreter. ADR 0018.
- **Every standard filter a PDF may name decodes** — all ten of Table 6's, `LZWDecode` last
  (§7.4.4.2, ADR 0036) and `CCITTFaxDecode` before it (§7.4.6, ADR 0021). No corpus page one
  reaches an LZW stream, which is why it took the specification track to get there. An image
  may be written into the content stream (§8.9.7, ADR 0019).
- **An image is masked every way §8.9.6 and §11.6.5.2 define**: its own stencil, an explicit
  `/Mask`, a colour-key `/Mask` (ADR 0023), and an `/SMask` of any size (ADR 0024), combined on
  the finer of the two grids — a documented choice, since the clause puts both on the unit square.
  §11.6.4.3's precedence is honoured and Table 144's `/Matte` is undone where the arithmetic is
  exact.
- **A reduced image is averaged**, by `Image::area_averaged` — a **documented departure from
  §10.7.4**, which requires point sampling and says "there shall not be averaging over the pixel
  area". §10.7.1 licenses it, this tree already takes two others in the same subclause by
  anti-aliasing at all, and the page that argues for it is otherwise illegible. ADR 0025.
- **A `/Group` is composited as one object** (§11.6.6), with the blend mode and both alpha
  constants reset *inside* it, and **the page is a group too, an isolated one** (§11.4.7) — so a
  page is drawn onto transparency and the medium's white imposed on the result. Non-isolated
  groups that blend, a knockout element whose shape is not its coverage, and a blending space that
  is not the device's are reported.
  ADR 0026.
- **A soft mask is a group evaluated for its opacity** (§11.5): positioned by `/Matrix` and the
  transform at the `gs`, rasterised by each backend at its own target, with `SoftMask::value` the
  one place rendered pixels become mask values — because §11.5.3's coefficients are not the
  luminance either rasteriser offers. ADR 0027.
- **Annotations draw, and are constructed where they have to be.** `/AP /N` is placed by §12.5.5's
  algorithm; an annotation without one gets a content stream built from its subtype's clause, or a
  report naming what the clause does not state. ADRs 0013, 0030.
- **A field's value is laid out from its `/DA` string** — §12.7.4.3 in full for the two field
  types that hold text, with quadding, auto-sizing, wrapping, comb cells and a password field's
  bullets. A stored appearance under `/NeedAppearances` is *spliced* rather than rebuilt: the
  clause replaces the stream "from … BMC to the matching EMC" and everything outside that pair
  survives. Refused by name: a `/DA` font `/DR` does not define, a composite `/DA` font, and
  §12.7.5.4's list-box selection, for which the clause states no appearance. ADR 0032.
- **A text string is decoded as §7.9.2.2 defines one** — UTF-16BE with surrogate pairs, UTF-8, or
  Annex D Table D.3's `PDFDocEncoding`, with §7.9.2.2.2's language escapes removed. The table is
  compiled in and is *not* ISO Latin 1.
- **A layer the document turns off is not drawn** — §8.11 in full as far as it decides what is
  marked, including `/VE` visibility expressions. ADR 0017.
- **One device pixel is the thinnest line, and both backends agree what that means.**
  `Stroke::device_width` is §8.4.3.2's zero-width minimum and §10.7.5's stroke adjustment in one
  function. ADR 0028.
- **A stroke that spans no distance still marks the page** — §8.5.3.2's degenerate subpaths are
  filled circles under round caps and nothing under the other two, a zero-length dash paints
  every cap oriented along the path, and both rules are `pdf-render`'s rather than either
  rasteriser's. So is what an empty clipping path admits, which is nothing (§8.5.4). ADR 0033.
- **Overprinting is ignored, and §8.6.7 is what says to ignore it**: this device has three
  additive colourants and no separations, and both §8.6.7 and §11.7.4's Table 146 reach the same
  answer — the special blend function is the source colour, which is Normal. The one configuration
  that would differ is a `DeviceCMYK` group space, which §11.6.6 already reports. `/Separation`
  `/All` and `/None` are honoured before the tint transform is parsed. ADR 0028.
- **The citations are checked.** `tools/conformance` holds every `§` in the tree to a clause the
  standard has — 2623 of them — every rustdoc blockquote to the standard's own words, and the
  ledger's 823 rows to the standard's subclauses. It prints the title of every table the tree
  cites, which is how the twentieth session found six comments calling Table 57 "Table 58". ADR
  0016, `doc/PLAN.md` §5a.
- Both backends draw everything the display list can express and agree on **every one of the
  sixteen blend modes, to the channel**: **fifteen** headless GPU scenes hold `tiny-skia` and
  Vello to the same pixels at more than one scale and along both axes (see trap 2), plus one
  single-pixel test, `vello_hands_back_straight_alpha`. The fifteenth is the one that found
  something — `cpu_and_gpu_agree_on_every_blend_mode` caught `Hue`, `Color` and `Luminosity`
  113 of 255 apart with §11.3.5.3's closed form saying the CPU backend was wrong (ADR 0046), and
  the thirty-ninth session wrote those four modes in `render-cpu/src/blend.rs` (ADR 0047). The
  list of disagreeing modes is empty and is still a *list*, ratcheted both ways.

### Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

Arrow keys / Page Up / Down / Space turn pages, Home and End jump to the ends, Escape quits. The
title bar names anything on the page that could not be drawn. A click follows §12.5.6.5's links
and performs the eleven actions of §12.6 this program can, printing every refusal — including
§12.7.6.4's import, which reads an FDF file **beside the open document** and nowhere else, and
says so when it declines (ADR 0090). `--no-sandbox` decodes JBIG2 and
JPEG 2000 in the viewer's own process — faster by a process spawn and a pipe round trip,
appropriate for documents whose origin you trust, and it prints a line saying what it gave up.

### Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
# The conformance gate is part of that run; its summary is worth reading rather than only passing.
cargo test -p conformance -- --nocapture   # 2762 citations, 296 quotations, 179 tables, 823 rows
# It prints every table the tree cites *and* every table the ledger cites, each with its title:
# a number that names nothing fails the gate, and a number that names the wrong table only ever
# gives itself away in the title (ADR 0095).
cargo run -p conformance --bin ledger      # regenerates the rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild another
# package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages, ~35 s
# The third gate: the text, against pdftotext, over the same 974 documents (ADR 0066).
cargo test --release -p pdf-model --test text_extraction -- --ignored --nocapture  # ~30 s
# A fourth census, whose denominator is a grammar rather than a page (ADR 0092).
cargo test --release -p pdf-model --test dates -- --ignored --nocapture   # 1545 dates, ~0.6 s
# The first oracle run on a fresh build directory is ~95 s and writes 319 MB of remembered
# reference renders; every run after it is the ~30 s above. Read the printed hit rate rather than
# the clock. Two environment variables matter:
#   PDFREF_CACHE=off              ask the three renderers again, which is how "the cache changes
#                                 no verdict" is re-checked over the whole corpus
#   PDFVIEWER_ORACLE_ONLY=a,b     compare only pages whose names contain a or b — 0.2 s for a
#                                 handful of documents; a filtered run refuses to check the
#                                 ratchets and says so
cargo build --release -p hayro-compare --bins
cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
# Two callgrind examples measuring different halves: the first stops at the display list, so a
# backend change measures as exactly zero there; the second rasterises.
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
cargo +nightly fuzz run cmap  -- -runs=50000     # §9.7's CMap parser and its decoder
cargo +nightly fuzz run crypt -- -runs=50000     # §7.6's encryption dictionary and key algorithms
cargo +nightly fuzz run variable_text -- -runs=50000  # §12.7.4.3's /DA parser and its layout
cargo +nightly fuzz run forms_data -- -runs=50000     # §12.7.8's FDF reader and §7.9.4's dates
```

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It arrives
through `iai-callgrind`, a dev-dependency that reaches no shipped binary, and `deny.toml` records
the exception with its reasoning. Nothing to chase.

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document`, decryption | Touches untrusted bytes first. `date.rs` is §7.9.4's date string — beside `text_string.rs` because NOTE 1 makes a date a *text string* that happens to spell one, with an ordering by the instant because §12.3.5.1's Table 156 sorts a collection by one (ADR 0092). `crypt.rs` is §7.6's standard security handler — every algorithm the clause numbers, written against its own subclause; `document.rs` is where §7.6.2 decides *what* is decrypted, because that is where an object's identity is known (ADR 0031). `tree.rs` is §7.9.6's name trees and §7.9.7's number trees, one module because the second clause defines itself as the first with integer keys — `lookup` for a caller with a key, `name_pairs`/`number_pairs` for one without, and `name_entries` for §12.7.7's named pages, which need the leaf's *reference* rather than what it resolves to — the component the conformance ledger found by four `silent` rows in two clauses naming it (ADR 0053). `text_string.rs` is §7.9.2.2 and Annex D's Table D.3, which is a code-to-Unicode table and so belongs here rather than beside `pdf-font`'s glyph-name encodings. `object.rs` is §7.3.1's nine basic types plus the reference that labels any of them, and `parser.rs` is where §7.3.7 drops a null-valued entry and keeps the first of a duplicated key. `filter.rs` is §7.4's ten standard filters — four decoded here, one a pass-through for §7.6.6, four image codecs deliberately answered `None` so a *content* stream naming one is visibly unsupported. `xref.rs` is §7.5 whole, and the one thing to know about it is that an entry is an `Option<Location>`: a free entry and an entry type this version of PDF has no meaning for both *record* that the number names nothing, because §7.5.6 makes a deletion the most recent copy of an object and skipping it would let an older section answer instead (ADR 0100) |
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3 fonts, image decode | Where PDF semantics live. `annotation.rs` is selection and placement (§12.5.5) and knows no subtype; `appearance.rs` is where a missing appearance is *constructed* from what its subtype's clause states, where a stored one is *spliced* under `/NeedAppearances`, and where the refusals are argued (ADRs 0030, 0032). `named_page.rs` is §12.7.7's two page-naming trees — the one place a page reaches this program from outside the page tree, with the clause's own four invariants checked against the document (ADR 0091). `action.rs` is §12.6's twenty action types, ten of which are performed — the seventh, §12.7.6.3's reset, being the first whose effect is ink rather than which page is shown (ADR 0087), and the eighth, §12.7.6.4's import, being the first that needs a *second file* — with `forms_data.rs` holding §12.7.8's Forms Data Format, which is that file: Tables 244 to 254 read through the same lexer and parser a PDF uses, since §12.7.8.1's four differences from clause 7 are all relaxations, and the values matched to widgets by §12.7.4.2's fully qualified name (ADR 0090). `file_spec.rs` is §7.11 whole — the two forms, Table 43, the string format's components as *bytes* because §7.11.2.1 says they are passed to an operating system "without interpretation or conversion of any sort", §7.11.2.2's resolution with both of the clause's own examples as tests, and §7.11.5's URL — and it produces no path for anything, because reading a specification is not fetching one (ADR 0104). `document_part.rs` is the one thing §14.12's `inapplicable` family owes a screen — the page a `DPart` begins at, because §12.6.4.5's `GoToDp` makes that dictionary decide which page is shown (ADR 0108). `uri.rs` sits beside them holding RFC 3986's reference resolution and no PDF at all (ADR 0070), `requirements.rs` is §12.11's requirements and §7.12's extensions — what a document says it needs, printed when it opens rather than reported per page, for the reason §12.11.6's own NOTE 1 gives (ADR 0074), `navigation.rs` is §12.4.4's whole presentation — Table 164's transition, `/Dur` and the `/PresSteps` list — read as data because a slide show is a window's job and its input is the document's, and `view.rs` is the `ViewState` that performs them — the one thing in this crate that is neither the document nor the page, because a layer a click switched off is not in the file (ADR 0065). `structure.rs` is §14.7 whole — the parent tree upwards, `Tree` downwards, §14.7.6's attributes with both of their precedence rules, PDF 2.0's namespaces with §14.8.6.2's role mapping, §14.8.2.2's artifacts, and §14.8.2.5's logical content order, which is the *other* order a tagged page has and the one its author meant (ADRs 0072, 0073, 0084). `accessibility.rs` is §14.9 and holds no PDF at all — it is the three substitution rules and §14.9.2.3's language hierarchy over spans, which is where they belong because the clause states them over *adjacency* and a concatenated string has thrown that away (ADR 0063). `variable_text.rs` is §12.7.4.3 and the one place in the tree that writes a content stream rather than reading one — it knows nothing about annotations or field types, only about a string, a box and a `/DA`. `soft_mask.rs` reads Table 142 and nothing else. `optional_content.rs` answers "is this layer on". `type3.rs` reads a font whose glyphs are content streams. `inline_image.rs` turns `BI` … `EI` into the stream an image `XObject` would have been. `image.rs` owns §8.9.6's and §11.6.5.2's masking, with `combine_on_the_finer_grid` the one place two rasters of different sizes are combined rather than refused; its `Decode` is §8.9.5.2's map held as one table per component and its `Conversion` is an *exact* per-image memo, which is what makes converting every image through its real colour space affordable (ADRs 0034, 0035). `page.rs` is §7.7.3: the tree walk, the four inheritable entries and the twelve that are not, `/UserUnit` (ADR 0038), and §14.11.2's five boundaries — of which `display_box` and `clip_box` are the two §12.2's `/ViewArea` and `/ViewClip` name, read by `viewer_preferences.rs` (ADR 0071). `signature.rs` is §12.8's signatures read and never verified — Table 255, §12.8.6's permissions, and `coverage`, which compares a `/ByteRange` with the file's length and is the one thing a renderer can say about a signature on its own evidence (ADR 0088). `attachment.rs` is §7.11.4's embedded files and §14.13's associated ones, listed and never written out (ADRs 0076, 0077). `collection.rs` is §12.3.5's portable collections — the columns, the sort, the folder tree, and §12.3.5.2's convention of writing a folder's identifier into an embedded file's *name* (ADR 0083). `measurement.rs` is §12.9's viewports and §12.10's geospatial dictionaries — the scale a drawing is at and the earth a map is of, with §12.9.2's five-step formatting algorithm and no transformation, because a projection is an EPSG registry and ISO 19162 rather than a clause (ADR 0082). `thumbnail.rs` is §12.3.4's `/Thumb` — the one image in this tree decoded from a dictionary with entries *removed*, since the clause makes eighteen of Table 87's insignificant (ADR 0081). `article.rs` is §12.4.3's threads — the one linked list in this tree that is a *ring*, so the visited set is how a well-formed walk ends rather than how a broken one is survived, and Table 31's page `/B` is read only to be checked against it (ADR 0080). `page_label.rs` is §12.4.2 in full over `pdf-syntax`'s number tree — the clause's four traps are no default numbering style, letters that repeat rather than carry, subtractive Roman numerals and a `/St` floor of 1, and its own worked example is the test |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both simple-font encoding algorithms (§9.6.5.2 for name-keyed programs, §9.6.5.4 for `TrueType`, ADR 0015). `name_keyed.rs` is what a name-keyed program offers a code — glyph by name, glyph by built-in code, and that code's name — and `cff.rs` and `type1.rs` each produce one, because §9.6.2.1's NOTE 1 makes them one format's two spellings (ADR 0040). `type1.rs` is §9.9's `/FontFile` and is the one program kept *parsed*, measured: re-parsing per distinct glyph put 11 ms on `tracemonkey.pdf`. `simple_code_table` takes no font descriptor, which is the shape of ADR 0039's finding: Table 112 makes an *embedded* program's own built-in encoding the base, and the Symbolic flag decides only among the cases where nothing is embedded. `DEFAULT_WIDTH` is Table 120's 0 rather than a preference. `code_for` is the one *backwards* route — a character to the code that draws it — and it is built by running the forward mapping over every code the font defines, so the two cannot disagree. `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D data; `substitute.rs` is the only machine-dependent code in the tree, and it now ranks three sources of a substitution request with an argument for the ranking — the font's name, then §9.8.3.2's PANOSE classification in `panose.rs`, then Table 121's flags, which producers set carelessly (ADR 0086). `cmap.rs` is §9.7's composite encoding, where `Code` carries a value *and* a length because the clause looks a code up "in the character code mappings for codes of that length" (ADR 0029). Deliberately not `tounicode.rs`: same file format, different destination. A Type 3 font is refused here |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. Three device decisions live here so the two backends cannot make them differently: `Image::is_smoothed`, `Image::area_averaged` (a departure from §10.7.4, ADR 0025) and `Stroke::device_width` (§8.4.3.2 with §10.7.5, ADR 0028). `soft_mask.rs` turns rendered pixels into §11.5's mask values. `Command::Group` is the one nested command (ADR 0026) and `impose_on_medium` is §11.4.7. `Path::extend_transformed` is the one place geometry moves rather than travelling with a transform (§9.3.6, ADR 0022). `MeshRaster` is §8.7.4.5.5's Gouraud interpolation, evaluated per device pixel and shared by both backends because neither rasteriser has the primitive and a second copy would only drift (ADR 0051). `Transform::max_stretch` is *not* `determinant().abs().sqrt()`: a shear separates the singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path. `blend.rs` is §11.3.5.3's four non-separable modes and §11.3.6's compositing formula, written here rather than in `pdf-render` on purpose: the clause states the arithmetic, Vello states it again in its own shader, and the cross-backend scene compares the two — sharing them would make it compare one implementation against itself (ADR 0047) |
| `render-gpu` | Vello/wgpu backend | Headless by construction. `soft_mask.rs` renders each mask to a texture and reads it back, because Vello's own luminance mask is the SVG formula and no blend mode is a `/TR` |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here. `cache.rs` remembers what each renderer produced, keyed on the invocation itself (ADR 0020); `digest.rs` is the SHA-256 that key is built from |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `pdf-sandbox` | Confined worker + the three image filters | Its `decode.rs` is the only place a JBIG2, JPX or CCITT codestream is looked at |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it |
| `tools/conformance` | Citation checker and the conformance ledger | Depends on nothing but `thiserror`. The one crate the citation scan skips — its own comments cite clauses that do not exist, deliberately |
| `viewer-core` | Empty | Documented responsibility only |

## Traps — read these before writing code

### 1. The metrics lie. Look at the page.

This is the most important thing in this file. `Interpretation::is_complete()` tells you what the
interpreter *knows* it skipped. It cannot tell you that a font loaded and produced garbage, that
a page is upside down, or that a gradient came out opaque.

The archetype: wiring bare-CFF support in made every affected document report `unsupported: []`
and render **almost no text**. The font loaded, nothing was reported, the wrong glyphs were drawn.
`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable` writes PNGs;
the oracle's artefacts are better (see "Things worth knowing").

Two automated checks catch a wrong mapping, both in `crates/pdf-font/src/lib.rs`:
`the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths` and the CFF
charstring's own advance are independent statements of the same fact, so this verifies the mapping
without consulting the mapping — and `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`.
Both were confirmed to fail when their defects are reintroduced. Neither replaces looking.

**Every page a new feature makes drawable is a page nobody has ever looked at**, and the habit has
paid every session since the tenth. What it found, in order: dashed squares that should not have
been solid (the `d` operator, nothing to do with the Type 3 fonts being written); `/Interpolate`,
a `Lab` table scaled 0..1, and a dropped soft mask (none of them inline-image defects); a
fax-encoded page **upside down** because `/Rotate` 90 and 270 had been exchanged since the first
page tree; a solid red page that turned out to be §9.3.6 behaving *correctly* on a malformed
composite glyph; `alphatrans.pdf`'s gradient painted opaque because one `return` dropped
§11.6.4.4's alpha; a knockout group whose report had been hidden by the soft-mask report; a `0 w`
line invisible on the GPU; `issue7901.pdf` drawing `üãÍ†Ë` because Table 115's presence
condition had been read as a condition on meaning; and a shading painted across a rectangle its
clipping path admits none of, on the first page that §8.5.3.3.1's trailing-`m` rule made
rasterisable at all.

**A page a feature makes drawable can be one that never rendered *at all*.** The
twenty-fourth session's rule turned four `no render` pages into drawn ones, and the group's
label — "path is empty or contains non-finite coordinates" — described the *symptom* of a
missing rule, not the defect the pages then revealed. A `no render` count is a to-do list of
pages nobody has looked at, and it is now 19 — six left it in the hundred-and-seventh session
(ADR 0097), and all three of those with any content on them were rendered and read. One draws a
title block correctly with a giant **سلام** across it, which is a word rather than garbage and
exactly what a file filed under an Arabic-rendering issue would contain; no reference opens that
document at all, so the readback is the whole of the evidence.

**A contradicted page's group names a hypothesis, not a diagnosis — seven for seven on being
wrong.** Type 3 fonts, `/Rotate`, `alphatrans.pdf`'s gradient and `french_diacritics.pdf` all sat
under labels whose stories were *true about the page* and not the disagreement. The fifth and
sixth came together in the twenty-eighth session: `mesh_shading_empty.pdf`'s entry said
"displaced horizontally" and the mesh is not displaced at all, and `issue8092.pdf` sat under
*substituted fonts* while its difference was a shading's `/BBox`. The seventh is
`issue20232.pdf`, whose entry said a descriptor setting both the Symbolic and the Nonsymbolic
flag left §9.6.5.4's symbolic route "unreachable here" — it is not unreachable, it is taken,
and the glyph at the far end of it is one this subset embeds with no outline. Open the artefact
before believing the label — **and measure it, because a label this project wrote is still a
label**. Twice now the instrument that settled one was the font's own `cmap`, `loca` and `post`
tables read directly, which costs ten minutes and answers exactly.

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect in code. `issue6621.pdf` blanked a
court seal under the only reading its `/Mask` samples admit, and `issue7901.pdf` drew garbage
under a defensible reading of Table 115. In both the code was right about the clause it cited.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape, so the
transform you hand a gradient, a pattern or an image is read **in the space the path is stated
in**; composing the page-to-device transform into it yourself applies it twice. Both backends did
exactly that, and it shipped: every gradient was mirrored about the page's horizontal centre line
(at scale 1.0 the page-to-device transform is its own inverse, so the second application leaves
just the flip), and `issue19971.pdf`'s 2500×1364 photograph came out as one flat rectangle.

Three things about how it survived:

1. **No metric saw it** — `unsupported: []`, right shape, colours from the right ramp.
2. **The CPU-versus-GPU comparison could not see it**, because both backends had it and therefore
   agreed. Two implementations agreeing is evidence *only where they can fail independently*.
3. **Every scene compared them with a gradient running along x**, where a y mirror is invisible.

The guards are `render-cpu/tests/shading_placement.rs` and `image_placement.rs`, pinning values
against §8.7.4.5.3 and §8.9.5.2 **at three scales**, plus `headless_gpu.rs`'s vertical-gradient
and image scenes. All were confirmed to fail when the defects are reintroduced.

**The sharpest form is about a convention rather than an axis.** `tiny-skia` treats a stroke width
of `0.0` as a hairline, which is exactly what §8.4.3.2 requires — so the CPU backend got the
clause right without anybody writing the rule down, and `kurbo` expands a zero-width stroke into
an *empty* outline, so every `0 w` line in every document was invisible on the GPU for fifteen
sessions. **Where two backends are the oracle, a decision either of them can make alone is a
decision neither has made**, which is why the device decisions live in `pdf-render`.

**It has now happened four times, and the fourth is the clearest.** §8.5.3.2's stroke with no
length: `tiny-skia` paints a projecting square cap where the clause asks for no output, `kurbo`
drops the contour before a cap is considered, and a path of one `m` is an *error* on one and
silence on the other — three different answers, none of them the standard's. `pdf-render`'s
`degenerate.rs` states it once, with the circle as this crate's own geometry rather than either
round cap's. `Clip::admits_nothing` is the same story for an empty clipping path, where Vello
happens to be right and was *verified* to be right by convention rather than by the clause —
which is exactly the position that reads as agreement and is not.

**The fifth instance is about what a library cannot say at all.** §11.4.6's knockout is
Porter-Duff Source modulated by coverage, which `tiny-skia` takes as a per-draw blend mode and
Vello has no way to state: its layers carry a compose mode, and a layer's compose runs over the
layer's whole *bounding box* with the clip's coverage applied to the source — so `Compose::Copy`
erased a row outside the shape, and the CPU backend was right first. What settled it was the
cross-backend scene, in one run, before anything reached a corpus. The rule that follows is trap
2's in the other direction: **where one backend can state a clause directly and the other has to
build it, the built one needs a scene at the magnitude *and* the fractional coverage where the
two constructions differ** — the knockout scene has a diagonal edge for exactly that reason, and
the difference there is `(1 − a·αs)` on what the destination keeps, written down in
`render-gpu`'s `knock_out`.

**And a scene set is worth what its scenes can *express*.** Fourteen cross-backend scenes
existed and every `Command` in every one of them carried `BlendMode::Normal`, so the two
backends' sixteen blend functions had never been compared at all — and three of them disagree by
113 of 255. The question to ask of a scene set is not "does it pass" but **"what parameter does
every scene in it leave at its default"**. ADR 0046.

**And a scene must be able to fail at the defect's *magnitude* as well as in its axis.** The
sixteenth session's first reduced-image scene was in the right axis and **passed with the GPU's
filter removed altogether**: 32 differing channels out of 160 000 is under
`MAX_DIFFERING_FRACTION`. It now draws an 800×800 image across most of the page and fails at mean
6.50 against 0.5. Deleting the code a scene guards is one command, and it is the only thing that
establishes the scene guards it.

### 3. An oracle is only as good as how it invokes the other renderers

The corpus oracle's first run reported 54 documents whose page *size* we disagreed about, which
looked like a `MediaBox` defect. `pdftoppm` and `gs` default to the **media box**; `mutool` and we
use the **crop box**, which §14.11.2.1 defines as the region "to which the contents of the page
shall be clipped (cropped) when displayed or printed". The harness had been asking two of three
references for a different page — and on a page whose crop box has the same size as its media box
but a different origin it would have compared a correct render against a displaced one and called
us wrong. Every invocation is now explicit about the page box, *including* `mutool`'s, whose
default was already right: a default that silently changes is a comparison that silently changes.

The twenty-first session found the same shape one level up: `gs` renders for a **printer**, so
Table 167's Print flag decides what it draws, and four link borders disagreed for that reason
alone. Check what question each reference is being asked before reading its answer as a verdict.

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them was "the
caller's responsibility" and then did not, so every modern PDF failed with a misleading `/Root is
not a dictionary`. Unit tests on fragments would never have caught it; the corpus caught it on the
first run. `crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. The converse is trap 8.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is what makes
the comparison harness trustworthy and what caught trap 1. Do not "helpfully" fall back to a
default that renders something plausible. **A rise in the incomplete count is not a regression
when it is a new report.**

The rule is easiest to lose *inside* a feature that is partly implemented, because the operator is
handled and the code path exists: `Tr` was parsed with three of its eight modes reported and the
four that change a clip silently absent; `/TK` was not read at all. The twenty-fourth session
found the same shape one level up — Table 57's `/LC`, `/LJ` and `/ML` read nothing at all while
`J`, `j` and `M` set the very same parameters, so three corpus documents silently drew with the
wrong caps and joins. **Where a clause gives a parameter two routes, implementing one of them is
the failure mode that reports nothing.**

There are now five places where a report accompanies drawing rather than replacing it, each
deliberate. An `/AcroForm` setting `/NeedAppearances` says its stored appearances may be stale and
we draw them anyway, because they are all the file offers (§12.7.4.3). §11.6.5.2's `/Matte` in a
colour space whose pre-blending cannot be undone after conversion is applied, because refusing it
would draw a rectangle of pure matte colour. A constructed appearance draws what its clause
states while reporting what it does not — a widget's background with its field's value named
(ADR 0030). And §8.11.4.4's `/User` and `/Language` categories leave a layer's state as the
configuration set it and say so, because switching it off would answer a question about this
machine that nobody asked (ADR 0044). The fifth is §12.5.6.7's `/LE` and `/Cap`, which decorate
a line the clause makes *required* while being optional themselves — **so the question to ask of
a refusal is whether the entry it refuses is additive or substitutive**, and a cloudy `/BE`
stays a whole refusal because a different border is not an extra mark (ADR 0106). Two different true statements; suppressing either loses
information. Do not generalise
it further without the same argument.

### 6. Colour: one conversion, and the specification often has no answer

Three separate `DeviceCMYK` → RGB conversions used to live here and they disagreed: `0.5 0 0 0.5
k` gave a red channel of 0.25, the same colour through `scn` gave 0.0, and a CMYK image gave a
third answer. Nothing about a rendered page reveals that. `crates/pdf-model/tests/colour_paths.rs`
drives one value through all three routes and demands they agree, and was verified to fail when
the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB, and
`colour::xyz_d50_to_srgb` the only place an XYZ becomes a pixel — that second rule exists because
the same defect had recurred one level down, with `lab()` and `icc::xyz_to_rgb` each holding their
own copy of the nine-constant D50-to-sRGB matrix.

The other half was written here for thirty-two sessions and was **wrong**. This file said "ISO
32000-2 defines no `DeviceCMYK` conversion at all", on the evidence of §8.6.4.4, which says
"concentrations of process colourants" and stops. **§10.4.2.5 defines one** — and it is the
formula the code's own comment called naive. What the standard does is *rank two answers*:
§10.3's ICC route for an ICC-enabled processor, which this tree is, and §10.4.2's "crude
approximations" for a less-capable one, with §10.3.2 licensing the fallback table itself. The
three sources that outrank the table are the same as before — `/DefaultCMYK` (§8.6.5.6), an
output intent's `/DestOutputProfile` (§14.11.5), an `ICCBased` profile. When you touch that
table, read ADRs 0009 and 0042 and change it as a documented choice. The same shape recurs
for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point compensation to the processor
whenever `/UseBlackPtComp` is `Default`, which is every real document, and ADR 0012 explains why a
stretch built from the entry is *undefined* on input Table 63 permits.

### 7. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it stops
being necessary, which has already removed several stale ones. A bare `allow` hides that forever.

### 8. A corpus finds what documents contain, not what the specification says

The mirror of trap 4. The ICC evaluator agreed with two other readers on every real profile in the
corpus; a test that assembled a profile *by hand* produced one whose darkest colour equalled its
white point, and black point compensation divided by floating-point noise and turned white into
pure green. `calrgb.pdf` page 14 states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`,
which Table 63 permits and no sane producer writes — and it is what proved the black point stretch
has no well-defined answer.

**Three rules have now been measured to be unreachable by all 974 documents, and the method is
worth as much as the finding.** §9.7.6.2's per-byte codespace test (as against comparing the whole
code numerically) and §12.5.2's rule that a stored appearance ignores `/CA` were each measured by
breaking the rule deliberately and running both gates: all 1794 oracle verdicts identical. §7.6.2's
signature exception was measured differently and more cheaply — **eight corpus documents carry a
signature dictionary, twenty-six carry an `/Encrypt`, and the two sets are disjoint**, which is one
`grep` rather than two gate runs. Each rule is required of any valid PDF, and in each case the only
thing defending it is one synthetic test. **That turns "the corpus does not cover this" from a
suspicion into a fact — sometimes for the price of a gate run, sometimes for the price of a
question about what the corpus contains.**

**And there is a fourth shape: a rule the corpus *does* exercise and cannot show you.** Three
documents delete an object in an incremental update (§7.5.6) and none of the three still
references what it deleted, so a reader that resurrects a deleted object renders all three
byte-identically. `crates/pdf-syntax/tests/cross_references.rs` is where §7.5's rules are pinned
by hand for that reason, each as a *pair* of files differing only in the rule.

This trap is why `CLAUDE.md` principle 5 defines *done* against the specification with a closed
exclusion list, and why the conformance ledger exists. A caution that changes no plan changes
nothing.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle's authority rests on a premise from ADR 0005: two implementations sharing no code
agreeing about a page is evidence. There are four ways for that to fail, and the second and
third are the common ones.

**A shared gap.** An unimplemented feature almost always falls through to a *default*, so two
unrelated programs that skipped the same clause produce the same picture and the gate reads it as
agreement. `visibility_expressions.pdf` is the case: `mupdf`'s `pdf-layer.c` carries `/* FIXME:
Calculate visibility from array */ return 0;` and `ghostscript`'s `pdf_optcontent.c` prints
`WARNING: OCMD contains VE, which is not supported (ignoring)`, while `poppler` and pdf.js
implement `/VE` and §8.11.2.2 is unambiguous. So the page stays contradicted, listed with the
source citations beside it.

**Shared data.** `mupdf` and `ghostscript` disagree with us about four pages whose colour
reaches `DeviceCMYK`, and agree with each other to under a level — because they are running the
same CMYK ICC profile. What settled it was *this tree's own* A2B evaluator, pointed at
`/usr/share/ghostscript/iccprofiles/default_cmyk.icc`: it reproduces both of their ramps to
within five levels while ours interpolates the sixteen corners. The general move is worth more
than the finding — **when two references agree suspiciously closely, ask what data they are
both reading, and evaluate it yourself.** ADR 0048.

**Shared code, and it is wider than `jbig2dec`.** One `ldd` in the forty-first session:
`pdftoppm`, `mutool` and `gs` on this machine all link the same `libfreetype.so.6`, while this
tree rasterises glyphs with `skrifa` and `tiny-skia`. So on a page whose difference is a
letter's edges the three references are one rasteriser and we are the only second opinion —
recorded on `Reference::independence` and in `Tolerance::widened_to`, whose comment had asserted
they share no code "with each other", and **acted on nowhere**: marking three references
`Shared` for text would leave nothing to vote, when what they share is one component of a page
and everything else about it is still three readings.

`mupdf` and `ghostscript` also both link `jbig2dec`, and on seven corpus pages it
decodes nothing, renders noise, or prints `segment marks bitmap coding context as retained (NYI)`.
Both emit the *same warning text*, because it is the same code emitting it. What settles those
pages is `tests/jbig2.rs`'s ninety-six encodings of one image, not anybody's agreement.

**Two answers to two different questions**, found in the twenty-first session: `mupdf` constructs
no link appearance at all while `ghostscript` renders for paper, where Table 167's Print flag says
not to draw one. Their agreement is a coincidence of two unrelated reasons.

The shape recurred immediately, and in a form where *we* are the minority: `mupdf` and
`ghostscript` both refuse `encrypted-attachment.pdf` and `auth-event-ef-open.pdf` for wanting a
password, `poppler` and this tree open them, and §7.6.6 says the refusal belongs to the stream
whose key is missing rather than to the file. Two against two is not a tie; it is a question with
an answer, and the answer is in the clause.

**So ask what a reference is made of and what it was asked, not only what it produced.** The
general form is in the type: `Reference::independence` says whether a renderer's agreement is
evidence and `Reference::voting` is what the gate iterates. `hayro` is marked `Shared` — it draws
a fourth panel and never votes, because we share its font rasteriser, its deflate, its JPEG
decoder and both new image codecs. `mupdf` and `ghostscript` are deliberately *not* marked
`Shared`: they share only `jbig2dec`, so recording the sharing where it applies keeps the evidence
of a thousand pages that marking them wholesale would throw away.

When a contradiction looks like "everyone disagrees with us", the cheap next step is not to
re-read our own code: search the other projects' source for the clause. A `FIXME` there is
stronger evidence than any number of agreeing pixels.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-model's targets and pdf-sandbox's *library*, not its
`pdf-sandbox-worker` binary — Cargo never builds another package's binaries. So the tests run
against whatever worker was last compiled. This is not hypothetical: while verifying that
`tests/jbig2.rs` can fail, the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed, because the stale worker was still decoding correctly.

`cargo test --workspace` or `cargo build -p pdf-sandbox --bins` builds it. Both gates call
`require_the_sandbox()`, which fails loudly if the worker is *missing* — but a missing worker and
a stale one look nothing alike, and nothing detects the second. `pdfref-hayro` carries the same
caveat, less dangerously: it never votes.

### 10a. A cached reference render is a fourth thing that can be stale

The oracle remembers what `pdftoppm`, `mutool` and `gs` said (ADR 0020), which took it from 75 s
to 25 and introduced exactly one new way to be wrong. The key is built from the invocation
itself — `Reference::build_command`'s own argument list, plus the renderer's version and the
document's SHA-256 — so **a flag that is not in the key is a flag that is not passed to the
renderer either**. What it cannot see is a renderer whose output changes while its version string
does not.

- `PDFREF_CACHE=off` runs the gate the old way, which is how "the cache changes no verdict" is
  checked over the whole corpus. **The variable names a *directory*, and only the literal `off`
  disables it** — so `PDFREF_CACHE=on` silently starts a fresh 319 MB cache in a directory called
  `on`. If a run takes 95 s and reports a 0% hit rate, look at the variable before the corpus.
- **The hit rate is printed and it is the tell.** Under 99% on an unchanged tree means the corpus
  or a renderer moved.
- **A remembered *timeout* is the one entry whose truth decays**, so it is counted on its own line
  and expires after a week. The argument for remembering it at all — two decompression bombs were
  46 of a 57-second run — is in `pdfref::cache`.

### 11. A report is only as good as the condition it fires on

Trap 5's other edge. Principle 3 says unsupported input must stay loud, and the reflex that
produces is to report whenever the unimplemented thing *could* be involved. Four instances, and
each cost something to get right:

- **§9.3.8, text knockout.** `Tk`'s initial value is true, so every text object in every document
  is composited under a model we do not implement. The first draft asked one of the clause's two
  conditions and named 7 documents — and took **three pages that agreed with the reference
  consensus out of the gated set**, for a difference that could not have appeared on any of them.
  Asking both conditions (the paint composites *and* two glyphs overlap) names 2. **Both of
  those conditions outlived the report**: they are what decides whether the seventy-second
  session's implementation builds a group, and a group per text object would cost a page-sized
  buffer per line of text for a difference almost none of them can show. A condition worked out
  for a report is a condition worth keeping when the feature lands.
- **§11.6.2, one object in parts.** The first check named six documents, two of which had been
  agreeing. Printing the actual alphas showed three of the six set `ca` or `CA` to **zero**, so
  one of the two parts paints nothing and there are no two portions to composite. The clause says
  "portions", plural; the code had taken the operator as proof of them.
- **§11.7.4, overprinting.** 63 documents, six `silent` rows, top of the demand list — and the
  honest condition has **no members** on this device. The instrument that settled it was not a
  corpus run but Table 146 read against a list of this device's colourants.
- **§12.5.6.19, an empty widget.** The report fired where the clause asks for nothing at all: a
  field with no `/MK` and no value *states* no appearance, and 23 documents were being named for
  it.

So: **derive the condition from the clause, print what it matched before trusting the count, and
cost it in gated pages** — a page that reports is a page the oracle stops judging. And the reverse
worry is real too: **a report can hide another report.** `knockout_smask.pdf`'s knockout gap was
covered by its soft-mask report for four sessions, which is an argument for closing reports rather
than accumulating them.

### 12. A bound derived from two agreeing references is tighter than the arithmetic

`oracle.rs` judges us relative to how far the consensus references sit from one another, widened
by a factor. That is the right rule — it stops a page where every renderer differs from being
called our defect — and **where two references agree very closely the bound can be tighter than
eight-bit arithmetic.** `smask_luminosity_oob_transfer.pdf` is one flat composite through a mask
of 0.75: the closed form is `(223, 99, 80)`, `mupdf` gives `(222, 98, 79)`, `ghostscript`
`(223, 99, 79)`, we give `(223, 100, 81)`. Everybody is within a level of the arithmetic, but the
two references are within a level of *each other*, so the bound is a mean of 1.11 and ours is
2.02.

What to do with such an entry is not to chase it: check the *closed form* — write the clause's
arithmetic down and see whether we are within a level of it, which `render-cpu/tests/soft_mask.rs`
now does — then list the page with the calculation beside it. The reflex the number invites,
tightening our rounding until a reference's rounding is matched, is curve-fitting with extra
steps. The same effect makes small-text pages judged against two `FreeType`-based references
harsher than two independent rasterisers can be.

## Environment

The agent runs as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer` through the
`coders` group. This causes recurring friction:

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`: `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie.** Anything needing a window fails at `XOpenDisplayFailed`. The
  GPU backend is headless by construction precisely so it can still be tested; the viewer binary
  cannot be run by the agent past event-loop creation.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`, so the two users never fight over `target/`. Do not "fix" this.
- **`pdfref` needs `--work-dir`** for the same reason; its default is `./target/pdfref`.
- **`cargo-fuzz` needs `+nightly`** explicitly, because `rust-toolchain.toml` pins stable 1.97.1.
  That pin is deliberate.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build without
  `git submodule update --init`.

## What is not implemented

Every one of these is *reported* at runtime rather than silently skipped — that is what makes the
corpus numbers trustworthy, and it is principle 3's requirement rather than a nicety. Sized by the
corpus: the count is how many of the 974 documents' first pages it affects.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| Variable text: a `/DA` font `/DR` does not define | 5 | Small | What is left of §12.7.4.3 (ADR 0032), and it is a *malformed file* rather than a clause gap: the clause requires the `/DA`'s font name to "match a resource name in the Font entry of the default resource dictionary" and states no recovery. Four are `FreeText` annotations in files with no interactive form dictionary at all, naming `/Helv`. Reported by name, exactly as a content stream naming an absent font already is; inventing Helvetica from the resource name would need a name-to-typeface table no clause states. |
| Variable text: a composite `/DA` font, a list box, `/DS` and `/RV` | 0 | Medium | The rest of §12.7.4.3's edges, and none is reached by any corpus document. A composite font needs a `CMap`'s codespace ranges inverted (§9.7.6.2) to turn a character into a code; §12.7.5.4's list box states which items are selected and nothing about how a selection *looks*; `/DS` and `/RV` are XFA rich text, which principle 5 excludes. |
| Encryption: a password prompt | 8 | Small | §7.6 is implemented (ADR 0031); what is missing is the *interaction* §7.6.4.1 describes — "the interactive PDF processor should prompt for a password". `Document::open_with_password` takes one and nothing asks for it, so 8 corpus documents are refused at the gate that a viewer with a window would open. This is `viewer-ui` work, not clause work. |
| Encryption: public-key handlers (§7.6.5) | 0 | Medium | Refused by name. Needs CMS enveloped data (RFC 5652), X.509 certificates and access to the user's private keys — a public-key infrastructure and a threat model rather than a cipher. No corpus document uses one. |
| Encryption: `/R` 5, and a non-ASCII revision-4 password | 1 | Small | Two refusals, one of them now cheap to close. Table 21 says `/R` 5 "shall not be used" and states no algorithm for it, so implementing it would mean copying another reader; `issue21579.pdf` writes it anyway. §7.6.4.3.2 step (a) wants a password in `PDFDocEncoding`, which `crypt.rs` refuses outside the range where it and Unicode provably agree — **and `pdf-syntax` now holds Table D.3**, since §12.7.4.3 needed it, so inverting that table would close this refusal outright. Nobody has done it and no corpus document needs it. |
| Annotation icons (§12.5.6.4, .12, .15, .16) | 2 | Small | A `Text`, `Stamp`, `FileAttachment` or `Sound` annotation with no `/AP` displays an icon whose artwork no clause states. Refused and named. Every stamp in the corpus carries an `/AP`, which is what a producer who cares has to do. |
| Predefined `CMap`s (§9.7.5.2) | 12 | Medium | 15 fonts name one of Table 116's registered `CMap` files (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …), which are not in the tree. Vendoring them is a licensing decision; guessing draws plausible text that says something else. The machinery they would plug into exists. |
| Text: a substitute that cannot be addressed | 42 | Medium | Counting *fonts*: 27 composite fonts with no `/ToUnicode`, so a CID cannot be taken to a character a substitute could draw, and 23 whose substitute draws none of the declared codes. Honest refusals rather than clause gaps; closing them means better substitution. |
| Optional content: the interactive half | — | Small | §8.11 is honoured wherever it decides what is *drawn*, and since the thirty-fifth session that includes §8.11.4.4's `/AS` usage application dictionaries for the `View` event (ADRs 0017, 0044). **What feeds a layer panel is read since the sixty-seventh session** — `/Order` as a tree with the clause's label-against-nesting distinction, `/ListMode`, `/Locked`, `/RBGroups` and a group's `/Name` — and `ViewState::set_group` is the switch, so what is missing is the panel itself. Still unread: alternate `/Configs`, which exist to be chosen between and need someone to choose, and the two usage categories that are questions about this processor rather than about the document, `/User` and `/Language`, which are reported. |
| Transparency group and mask departures (§11.4, §11.5.3) | 19 | Medium | What is left of Table 145's answers after the seventy-first session drew §11.4.6 and the hundred-and-third flattened what §11.4.4's NOTE 5 permits, each reported where it can change a pixel (ADRs 0026, 0093): **a knockout element whose shape is not its coverage** (§11.4.6, 5 documents — a soft mask, an image's own alpha, a nested group, or a non-isolated group that also blends; the clause's own sentence about a separate shape value is the reason), **non-isolated with a blend mode inside it and a `Do` NOTE 5 cannot flatten** (§11.4.4, 6 documents — one whose own alpha, blend mode or soft mask makes the group's result composite non-trivially, so the backdrop genuinely has to be removed and NOTE 4's second alpha channel is what that needs), and **a blending colour space that is not the device's three components** (§11.6.6, 4 documents, all `/DeviceCMYK`, which means a second raster format). Plus **a soft mask's group with such a space** (§11.5.3, 7 documents). |
| Grid-fitting a stroke's coordinates (`/SA`, §10.7.5) | — | Small | The clause's single-pixel rule is implemented; adjusting "the line width and the coordinates of a stroke … to produce lines of uniform thickness" is a **documented departure**, because the non-uniformity it removes is an artefact of the binary scan conversion §10.7.4 requires and this tree already departs from by anti-aliasing. Nothing reports it: there is no page on which this device could do better. |
| Image `/Mask` on a filtered image, `/Matte` outside the device spaces | 0 | Small | What is left of §8.9.6 and §11.6.5.2 after ADRs 0023 and 0024, and no corpus document writes any of it. A colour key is a test on the samples a filter delivers, and a `DCTDecode` or `JPXDecode` image has become RGBA before the unpacker sees it — the clause's own NOTE 2 names that pair as the one lossy coding makes unreliable. A `/Mask` stream that is not an image mask is here too, which Table 87 excludes and 1 document writes. So is a `/Matte` on an image whose space is not `DeviceGray` or `DeviceRGB`: §11.6.5.2 requires the pre-blending to be undone *before* colour conversion, and this crate holds one RGBA raster per image, so the inversion is exact only where that conversion was the identity on components. |
| A degenerate subpath's single device pixel (§8.5.3.3.1) | — | Small | "[A] degenerate subpath … shall be considered to enclose the single device pixel lying under that point" when *filled* — distinct from §8.5.3.2's stroking rule, which is implemented. Neither backend paints it, and the clause calls the result "device-dependent and not generally useful" in the same breath. Recorded in the ledger rather than reported, because a report would name pages on which no reader could tell. |
| Annotation `NoZoom`, `NoRotate`, `/FixedPrint` | — | Small | Table 167 bits 4 and 5, and a watermark's `/FixedPrint`, make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Soft masks and `/Mask` at a grid the bound refuses | 1 | Small | `issue16263.pdf` gives a 2x2 image a 34862x4332 mask — 151 million samples, 604 MB — and that pair is refused and named. The answer the clause describes is compositing at *device* resolution, which means the display list carrying an image and its mask separately. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. The format's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. |
| A stream whose data is in an external file (§7.3.8.1) | 0 | Small | Table 5's `/F`, `/FFilter` and `/FDecodeParms`. The clause says the bytes between `stream` and `endstream` "shall be ignored" and the data is in a named file, which the renderer has no filesystem to open (principle 3, ADR 0014). So such a stream is refused by name rather than drawn from the bytes the clause discards — which is what it used to do, silently, for the project's whole life. No corpus document writes one, measured. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | A field's *appearance* is built (ADR 0032), §12.7.6.3's reset is performed (ADR 0087) and §12.7.6.4's import is too, over §12.7.8's FDF read whole (ADR 0090). What is left of the clause's **behaviour** is §12.7.6.2's submission, which needs a network, calculation order and validation. In scope wherever it *displays*. **JavaScript and script-driven field behaviour are excluded** by principle 5. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents and §14.9. §14.9 is what `CLAUDE.md`'s "as far as accessibility needs it" means, and since the sixtieth session all four of its text entries are read (ADR 0063) — what is missing is a *consumer*: `Interpretation::speech()` returns runs of text with their languages and nothing hands them to AccessKit. Left after that: §14.7's structure tree as data and §14.8's types and attributes, which are a vocabulary for *repurposing* rather than for a screen. |
| Sandboxing the *rest* of the renderer | — | Large | Spike D is done for the image codecs (ADR 0014). Interpreting and rasterising still happen in the main process. |

## How much of the specification is implemented

Four answers, in ascending order of how much they should worry you: what we *report*, what an
independent implementation *sees*, what the standard contains, and what a person has actually
read. The first two are measured. **The third is a self-assessment and has been wrong twice** —
it called clause 9's encoding algorithms "implemented in full" while §9.6.5.4 was one line
covering about one and a half of its five routes. Both errors were found by pixels.

**The fourth is the conformance ledger, and since the fifty-sixth session it has no unasked
questions left**: all **823** subclauses of the eight technical clauses have been read against
this code.

| status | rows | |
|---|---|---|
| `implemented` | 366 | every normative requirement in the clause is executed |
| `partial` | 240 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so |
| `inapplicable` | 88 | a marking device, a layout engine or a production workflow |
| `out-of-scope` | 87 | principle 5's closed exclusions, which the row names |
| `reported` | 35 | not implemented, detected and named at runtime |
| `writer-side` | 7 | addresses a PDF writer; we do not create files |

**The `silent` count is zero**, for the first time since the ledger was built in the ninth
session. Read it narrowly, because it is a narrow claim: **there is no requirement in the eight
technical clauses that this program fails without saying so.** It is not "the standard is
implemented" — `partial` and `reported` are 283 rows between them and each names what it owes.

**These counts come from `cargo run -p conformance --bin ledger`, which prints them, and not from
arithmetic in this file**: the ninety-ninth session's entry said fourteen silences where the
ledger held thirteen, which is the second time this file's own count has been wrong about the
ledger and the reason the command is quoted rather than the number remembered.

**Twenty-two sessions took it from 178 to 0**, and almost none of that was rendering: §12.6's
remaining actions, §12.2's viewer preferences, §14.7's attributes and namespaces, §14.8's
vocabulary, §14.13's associated files, §7.11.4's attachments, §7.12's extensions, §12.11's
requirements, §12.7.8's forms data format and §12.7.7's named pages.
Two of those sessions moved a *gate* — §12.5.6.7's leader lines took the corpus's incomplete
count 90 → 89 — which is the honest shape of specification-track work and the reason both tracks
exist.

**The ledger has been wrong four times and this file's arithmetic about it once**, which is worth
knowing before trusting a row: §8.9.5.3's note said reduction was unaddressed and §10.7.4
addresses it in the opposite direction; §8.4.3.2's row described `tiny-skia`'s behaviour rather
than the clause; and three `implemented` rows claimed behaviour the code never had (ADRs 0056,
0057, 0060). **A row that names a rasteriser's behaviour has recorded that rasteriser**, and a
row written during a review describes what the code *should* do. The defences are reading the
*family* rather than the row, and `FILE_ONLY_EVIDENCE_CEILING`.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 964 | 99% — the other 10 are 8 needing a password and 2 encrypted beyond us |
| reaches page one | 959 | 98% |
| **draws with nothing reported** | **869** | **89%** |
| draws, with something reported | 90 | 9% |

**This number measures honesty, and honesty can fall as capability rises.** It fell from 72% in
the eighth session when 24 documents began saying they carry a Type 3 font; it rose by twenty in
the thirty-first when a bare Type 1 program began to be *read* rather than silently substituted
for. So a rise is only good news when you can name the capability that caused it, and a fall is
only bad news when you cannot name the silence that ended. The whole trajectory is in "How the
project got here".

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1666 we claim to draw
completely:

| | count | share |
|---|---|---|
| agree with the reference consensus | 840 | 50% |
| **contradicted by it** | **65** | **4%** |
| the references cannot agree among themselves | 750 | 45% |
| not comparable (geometry, or fewer than two renderers) | 10 | 1% |

**One page in twenty-five that we say we drew completely, two independent implementations say we
did not.** All 65 are named and grouped in `oracle.rs`; §5 below has the breakdown, and 15 of
them have nothing on the page to explain it — down from 23, because eight were measured rather
than fixed.

**Read the 45% ambiguous with care.** It is not "half the corpus is unsettled": 372 of those
pages are two long books whose text uses fonts nobody embedded, so each renderer substitutes
differently. Ambiguity concentrated in a handful of documents says more about those documents
than about the gate — so read that row as "reported nothing", not "drew it right".

### By clause

823 subclauses, and counting them is a poor proxy for work: clause 12 is 166 of annotation
subtypes a viewer adds one at a time, while clause 8's 128 decide whether any page looks right at
all. The ledger's own notes are the detail; this is the shape.

| Clause | Rows | Where it stands |
|---|---|---|
| 7 Syntax | 138 | Objects, **every standard filter**, both xref forms, object streams, incremental updates, recovery by scanning, and **encryption at every revision and method §7.6 states**. §7.11's file specifications are refused by architecture (no filesystem, no network) except the embedded ones, which are read with §7.11.4.2's related files and §7.11.6's collection items (ADRs 0076, 0083); §7.12's extensions dictionary is read (ADR 0074). **No row of this clause is `silent`.** |
| 8 Graphics | 128 | The clause with the most coverage: the whole graphics state, path construction and painting, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, masking, ICC colour management, optional content. |
| 9 Text | 65 | Simple and composite fonts through **every font program Table 124 defines**, the standard 14 by substitution, Type 3, all eight rendering modes, all nine text state parameters, both encoding algorithms, §9.7's two mappings, both writing modes, and **§9.8.3.2's PANOSE classification, which outranks Table 121's flags where they disagree** (ADR 0086). **No row of this clause is `silent`.** Missing: Table 116's predefined `CMap`s, which is a licensing decision rather than an algorithm. |
| 10 Rendering | 36 | 19 rows `inapplicable` — halftones and transfer functions describe a marking device. Colour management, rendering intents and §10.7.3's smoothness tolerance are done; §10.7 carries four deliberate departures, all licensed by §10.7.1's NOTE and each named. |
| 11 Transparency | 58 | All sixteen blend modes on both backends, `ca`/`CA` reaching a shading, `/SMask` at any resolution, `/Group` composited as one object with the page itself an isolated group, **§11.4.6's knockout wherever an element's shape is the coverage it is drawn with**, and **§11.4.4's NOTE 5, which says the commonest non-isolated group need not be built at all** (ADR 0093). Left and reported: a knockout element whose shape is not its coverage, the non-isolated group NOTE 5 cannot flatten whose elements blend, a blending space that is not the device's. |
| 12 Interactive | 166 | **Appearances, constructed ones, a field's own text, navigation, thumbnails, and the ten actions a viewer can perform** — a go-to, a set-OCG-state, a hide, §12.6.4.12's four page commands, §12.6.4.8's URI, resolved against Table 211's `/Base` and printed rather than opened (ADR 0070), and §12.6.4.7's thread action, which needed §12.4.3's articles built before it could be performed (ADR 0080) — and the whole of §12.4.4's presentation read and none of it played, §12.4.3's articles read as threads of beads, §12.3.4's thumbnails decoded with the eighteen entries §12.3.4 makes insignificant dropped (ADR 0081), §12.9's viewports and §12.10's geospatial dictionaries including §12.9.2's formatting algorithm (ADR 0082), §12.3.5's portable collections with §12.3.6's named layouts (ADR 0083), and §12.6.3's trigger events with the appearance each one asks for, **§12.7.6.3's reset-form action, which is the one form action whose whole effect is a picture** (ADR 0087), **§12.8's signatures read and never verified, with the one check a program without a trust store can make** (ADRs 0088, 0089), **§12.7.8's forms data format read whole with §12.7.6.4's import performed** (ADR 0090) **§12.7.7's named pages, whose templates that import adds to the document** (ADR 0091) and **§12.6.4.4's embedded go-to, which opens the document inside this one** (ADR 0094). **No row of this clause is `silent`**, which as of the hundred-and-first session is true of every clause; what is missing is *reported* — a submission, a launch, an FDF annotation, a signature's validity. |
| 13 Multimedia | 81 | **Excluded** by name on principle 5's closed list. The rows carry the exclusion rather than being omitted, because an invisible exclusion is indistinguishable from an oversight. |
| 14 Interchange | 151 | Output intents, page boundaries, marked content as a bracket, §14.7's structure tree in both directions **with its attributes, classes and namespaces** (ADR 0072) and — since the sixtieth session — **the whole of §14.9's accessibility text**: `/Lang`, `/Alt`, `/ActualText` and `/E`, each in both places the clause puts it. **§14.8.4's forty-one standard structure types** (ADR 0078), §14.8.5's attribute mechanism (ADR 0079), §14.13's associated files (ADR 0077) and **§14.8.2.5's logical content order, which is a second reading of every tagged page** (ADR 0084). **§14.8.5.6's `PrintField`, which says what a flattened form field was** (ADR 0085). **No row of this clause is `silent`**, and §14.7.7's worked example is a test rather than a row — 12 of §14.8.5's 16 rows are `inapplicable`, because a layout attribute describes the process that made an appearance this reader already has. |

So: the parts of the standard that decide whether a page is drawn correctly are largely done; the
parts that make a document *interactive* have just started.

### Feature by feature, from the source

| | |
|---|---|
| Content-stream operators | **73 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler). `MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored. |
| Filters | **10 of 10** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `LZW`, `RunLength`, `Crypt` (pass-through, because §7.6.6's crypt filter is applied when the object is loaded), `DCTDecode`, `JBIG2Decode`, `JPXDecode`, `CCITTFaxDecode`. `LZWDecode` was the last one absent and landed in the twenty-seventh session, written from §7.4.4.2 including Table 8's `/EarlyChange`. Table 92's abbreviations are expanded in `inline_image.rs`. Not read: Table 13's `/ColorTransform`, whose one corpus witness contradicts the clause (§7.4.8's ledger row). |
| Encryption (§7.6) | **Revisions 2, 3, 4 and 6**, `/V` 1, 2, 4 and 5, methods `V2`, `AESV2`, `AESV3` and `Identity`. Every numbered algorithm a *reader* runs — 1, 1.A, 2, 2.A, 2.B, 4, 5, 6, 7, 11, 12, 13, and 3's first four steps. All four of §7.6.2's exceptions, plus Table 20's two. Refused by name: `/R` 5, public-key handlers, `/CFM /None`, a non-ASCII revision-4 password. |
| Colour spaces | **11 of 11** families, the three CIE-based ones converted rather than approximated, plus §8.6.5.1's withdrawn `CalCMYK`, which the clause redirects to `DeviceCMYK`. An *image* in an `ICCBased` space is still unpacked as a device space where a fill in it is not (§8.6.5.5). |
| Function types | **4 of 4**. Shading types **7 of 7**, on both backends. Pattern types **2 of 2**. Blend modes **16 of 16**. |
| Font programs | **All five of Table 124's**: bare Type 1 (`/FontFile`), TrueType, CFF, CFF-in-OpenType and CID-keyed CFF — plus Type 3, whose glyphs are content streams and are run by `pdf-model`. Which reader applies is decided by the program's leading bytes, not by the key or Table 125's `/Subtype`. A CIDFont writing `/FontFile` — which Table 124 does not permit — has its charstrings indexed by the CID, because §9.7.4.2 says that of a non-CID-keyed CFF and §9.6.2.1's NOTE 1 makes the two one format (ADR 0049). |
| Vertical writing (§9.2.4) | Both sets of a glyph's metrics: mode 0's from `/Widths` or `/W`, mode 1's from `/W2` and `/DW2` with Table 122's `[880 -1000]` default and `v`'s horizontal component at half the glyph's width. §9.4.4's three writing-mode-dependent terms follow — the displacement is `ty`, `Th` multiplies `tx` alone, and a `TJ` adjustment moves along the writing direction (ADR 0045). |
| Composite fonts (§9.7) | **Both of the clause's mappings** (ADR 0029): codespace ranges matched byte by byte and deciding a code's length from 1 to 4, `cidrange`, `cidchar`, `notdefrange`, `notdefchar`, `bfchar`, `/WMode`, `usecmap`, Table 118's `/UseCMap`, §9.7.6.3's recovery; then a CID-keyed CFF's charset, a `/CIDToGIDMap` stream, or the identity, chosen by what the embedded program *is* rather than by `/Subtype`. `/W` and `/DW` are indexed by CID. |
| Text rendering modes | **8 of 8** in §9.3.6 Table 104: fill, stroke in user space, both per glyph, invisible, and the four that add glyphs to the clipping path at `ET`. An operand outside 0..7 is reported. |
| Text state parameters | **9 of Table 102's 9**. `Tk` (§9.3.8) was the last, and it is a text object becoming §11.4.6's knockout group where the two models can differ. |
| Word spacing (§9.3.3) | A property of the *code's encoded length*, not of the font: an embedded `CMap` may define codes of several lengths in one font and four of the corpus's do. |
| Annotations | Placed by §12.5.5, drawn from `/AP` — **the one of Table 170's three the pointer asks for** — and **constructed** where there is none: a link's border, a square, a circle, a polygon, a polyline, an ink scribble, a line **with §12.5.6.7's leader lines** (ADR 0075), §12.5.6.10's four text markup subtypes, a widget's `/MK` frame, and **its field's text** (ADRs 0030, 0032, 0043). Icons are refused and named, and so are the two entries that state an effect with no shape: Table 179's line endings and a line's `/Cap` caption. |
| Form fields (§12.7.4.3) | A text field's `/V`, a choice field's selection, a button's Table 192 caption and a `FreeText`'s `/Contents`, laid out from a `/DA` string resolved in `/DR`: quadding, auto-sizing, wrapping, Table 232's comb cells, and a password field's bullets. `/NeedAppearances` splices the `/Tx` region of a stored stream and keeps the rest. |
| Text strings (§7.9.2.2) | All three encodings, chosen by the clause's prefix, with surrogate pairs paired, §7.9.2.2.2's language escapes removed and Annex D Table D.3 compiled in. |
| Image masking | All four mechanisms an image can carry plus the graphics state's own, combined on the finer of the two grids with a bound on the growth; a graphics-state mask is combined at *device* resolution. §11.6.4.3's precedence decides which wins. |
| Transparency groups | §11.6.6's `/Group` with the blend mode and both alphas reset inside, and §11.4.7's page group, which is why a page is drawn onto transparency and imposed on the medium afterwards. |
| Sample decoding (§8.9.5.2) | The clause's linear map in full, per component, with Table 88's defaults — including `Lab`'s `[0 100 …]` and `Indexed`'s `[0 2^n − 1]` — and its closing clamp. One lookup table per component, built once per image, so the unpacker's arms do not know what a `/Decode` array is. Applied on all five routes, `DCTDecode` included. |
| Image resampling | Magnification is §8.9.5.3's `/Interpolate`; reduction is §10.7.4's, and is the one place this tree knowingly does what a clause forbids (ADR 0025). Both decisions live in `pdf-render`. |
| Scan conversion (§10.7) | **Four** deliberate departures, all licensed by §10.7.1's NOTE — anti-aliasing twice over, area averaging, and §10.7.5's grid-fitting. `/FL` is ignored by the clause's own permission; `/SM` decides a ramp's sampling, upwards only; `/SA`'s single-pixel rule **is** implemented. |
| Line width (§8.4.3.2) | A zero width is one device pixel on both backends, in `Stroke::device_width` alongside §10.7.5's rule, because the clause's own NOTE makes them the same width. |
| Overprint control (§8.6.7, §11.7.4) | Ignored, and the clause says to. Special colourants `/All` and `/None` are honoured before the alternate space and tint transform are parsed. |
| Font descriptors (§9.8) | Table 120's `/Flags`, `/MissingWidth` — default 0, not a guess — and the three `/FontFile` entries, plus `/FontWeight` and `/ItalicAngle` for choosing a substitute. Table 121's Symbolic bit decides §9.6.5.4's route, and §9.8.2's "historical accident" paragraph decides a descriptor that sets Symbolic and Nonsymbolic together. The dimensional metrics are unread because this tree selects an installed face rather than synthesising one. |
| Simple font encodings (§9.6.5) | The base encoding, `/Differences` over it, and — for an *embedded* program — the program's own built-in encoding as the base Table 112 says it is, with the Symbolic flag deciding only among the cases where nothing is embedded (ADR 0039). |
| Page geometry (§7.7.3.3) | Table 31's `/MediaBox`, `/CropBox` intersected with it, `/Rotate` clockwise as displayed — which in this y-up space is a negative rotation — and `/UserUnit`, "the size of default user space units, in multiples of 1/72 inch", which scales the page and everything on it. The four inheritable entries are inherited and the twelve that are not, are not (§7.7.3.4). |
| Optional content | §8.11 wherever it decides what is drawn: configuration, membership, `/VE`, intent, all three places `/OC` can appear, and §8.11.4.4's `/AS` usage application dictionaries for the `View` event, at a magnification of 1.0 (ADR 0044). Not read: `/Order`, `/ListMode`, `/RBGroups`, and the `/User` and `/Language` categories, which are reported. |

## What to do next

**Two tracks, and the discipline is to take from both in every session.** *Demand-driven* is
everything the corpus and the oracle name. *Spec-driven* was "read the next unreviewed clause
family" for forty-seven sessions and **is not that any more**: the ledger reached zero unreviewed
rows in the fifty-sixth session, and reached zero `silent` ones in the hundred-and-first — so the
specification track is now its **35 `reported` rows and the notes on its 240 `partial` ones**,
each of which names what it owes and where. A project running only the first
track finishes when the corpus goes quiet, which can happen with a great deal of the standard
unimplemented and nothing able to say which parts; one running only the second ships features no
file exercises. This is a `CLAUDE.md` principle-5 rule, not a suggestion.

**There is no silence left, so the map is the reports**, and that is the shape of the next
several sessions:

- **Every row of clause 12 is read, performed, or *reported* by name.** So the
  specification track's map
  is no longer "which clauses are unread" nor "which are silent" — it is the **35 `reported` rows
  and what each would
  take**, and the three nearest are `viewer-ui` work rather than clause work: **a field a person
  can edit**, which §12.7.6.3 has now proved the machinery for (a changed value redraws
  correctly, ADR 0087) and which needs an editor; **a layer panel**, whose data model has been
  read since the sixty-seventh session; and **a presentation mode**, whose data model has been
  read since the seventieth. **The corpus's own action
  census, taken in the sixty-second session by walking every object of all 964 openable
  documents:** `GoTo` 269 actions in 138 documents, `JavaScript` 234 in 55 (excluded by principle
  5), `URI` 217 in 8, `GoToE` 31 in 2, `Named` 9 in 3, `ResetForm` 3 in 2, `Rendition` 2 in 1,
  **`SetOCGState` 1 in 1, `Launch` 1 in 1, and no `Hide` at all**; 32 documents carry an `/AA`.
  `GoToE` was the largest action nobody had written and **is performed as of the
  hundred-and-fourth session** (ADR 0094): its target is inside the file already open, so it
  needs no filesystem, and `EmbeddedGoTo::target_in` opens it. **§12.7.8.3.4's FDF annotations
  want the machinery that leaves behind** — a dictionary in one file drawn onto a page in
  another — and it now exists and has a caller.
- **Clause 14 is closed.** §14.7 is read whole — the tree in both directions, its content items,
  attributes, classes, namespaces and now its `/IDTree` — §14.8.4's forty-one standard types,
  §14.8.2's artifacts and reversed show strings, §14.8.2.5's logical content order, §14.8.5's
  attribute mechanism with §14.8.5.6's `PrintField`, §14.13's associated files, and §14.7.7's
  worked example as a test (ADRs 0072, 0073, 0078, 0079, 0084, 0085). **What it owes is a
  consumer**: `Interpretation::speech()` returns runs of text with languages, `Tree::logical_text`
  returns a page in the order its author meant, `Tree::standard_role` says what each element is,
  and nothing hands any of them to AccessKit — which is the one change that would make six
  sessions of reading visible.
- **Substitution quality is no longer a silence** (ADR 0086), and what is left of it is the
  *choosing*: §9.8.3.2's classification now outranks Table 121's flags, and 42 corpus fonts still
  substitute badly for reasons no clause states — 27 composite fonts with no `/ToUnicode`, and 23
  whose substitute draws none of the declared codes.

Three shapes of session have worked, in order of how often they paid: **the same family for both
tracks** (§8.11 in the ninth session, §9.7 in the twentieth, §12.5 in the twenty-first); **take
the demand item, then read the family the code you just wrote cites** — before writing it, not
only after, since the sixteenth session found the clause governing its demand item *forbade what
the demand item asked for*; and **take the demand item from the ledger's own silence list**,
which in the nineteenth session dissolved the demand item entirely.

Every family review has produced findings the demand item could not reach — fifty-three of them.
**A gap sized by a corpus is a hypothesis about a clause**, and the only instrument that can test
it is the clause. Conversely **a demand curve cannot rank a requirement no file exercises**, nor
notice one a *fallback* hides: `/FontFile` sat at a corpus count of zero while 57 documents
embedded one and drew a substitute in silence.

**The one-line version of each track.** Demand: **65 pages we claim to draw are contradicted, 15
of them for no reason visible on the page, and none of those is above its bound**, and **90 of
974 documents still draw incompletely**, of which 42 are fonts, 13 transparency and 10
annotations — and the fifty-ninth session's reading of the
corpus's own issue trackers says most of that is glyph rasterisation on files chosen for having
hard fonts, which the sixty-eighth session then measured on one of them. The largest item any corpus document still names is §9.7.5.2's predefined
`CMap`s at 12, which is a licensing decision rather than code, followed by a password prompt at
8, which is `viewer-ui` work. Spec: **`REVIEW_OWED` is empty, 0 of 823 subclauses are unread**,
and the debt is **0 `silent` and 35 `reported`, down from 201 thirty-four sessions ago** — plus
the notes on 240 `partial` rows, which is where the next map has to be drawn from.

### 0. The ledger, and where a false claim can still hide

- **Keep `REVIEW_OWED` empty.** A clause the code cites and nobody has read is the cheapest debt
  this project can accrue, and the list fails the build the moment one appears.
- **`FILE_ONLY_EVIDENCE_CEILING` is zero and the assertion is now `==`.** 58 → 0 over four
  sessions of auditing §7.10, §7.5, clause 8 and the remaining twenty-three (ADRs 0098, 0100,
  0101, 0102), and **every one of the four found a false or unheld claim**. A new `implemented`
  row arriving with a whole file for evidence fails the build. What it does *not* say is that
  the right test was named — the gate cannot tell whether a named test covers the clause, and
  three of the four false claims this population hid were caught by the oracle rather than by a
  row.
- **The population with no gate at all is the 240 `partial` rows**, and reading them has now
  paid three sessions running. What to look for, in the order the findings came: a note that
  *understates* what the code does (five in the hundred-and-fifteenth), a note whose **reason**
  has expired — "while §X does not exist", "needs §Y" — which is the class no gate can watch
  (the hundred-and-seventeenth and hundred-and-eighteenth), and a note whose "what IS done" half
  is simply wrong, which is the one that costs pixels and has not yet been found in this
  population. Roughly 190 rows remain unread against the code.
- **A silence is not the same as a gap**, and the first move on one is neither a report nor a
  feature: work out what the clause asks *of this device*. The nineteenth session closed two
  differently — §10.7.5's `/SA` was implemented in the half a display can state and recorded as a
  departure in the half it cannot, and §11.7.4's overprinting was six rows a reading of Table 146
  removed altogether.
- **A `partial` row's note describes what somebody found, not what is there.** The one silence
  that was hiding inside one — §10.7.3's `/SM` — was closed in the seventy-fourth session, and
  the note had named it exactly. There is no known second; that is not the same as there being
  none.

Four items that are small, listed before the big lists because they are small:

- **~~Bound a group's buffer to the band its clip admits.~~ Measured and removed** (ADR 0103):
  it is **0.14%** of `bug1721218_reduced.pdf`, the page this file named as the one that would
  show it. What is worth 24.34% of that page is `MaskCache::get`, and the shortcut nobody has
  taken is in `MaskCache::build`'s own comment — a child's band is inside its parent's, so a
  chain could be one crop and one intersect instead of a fill and three. It needs the
  intermediate clips cached, and the page is already at 87% of `MASK_BUDGET`, so it starts with
  a measurement of what the intermediates cost.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest of the
  renderer runs in the main process, which is the half of principle 3 not yet built. The protocol
  would have to carry a display list rather than an image, which is a real design question.
- **Ask whether the *decompression* can be avoided rather than made faster.** 28.0% of
  interpretation is `zlib_rs` inflating the page; a content stream is inflated once per
  interpretation and nothing caches it between the corpus gate's two passes. The one measured
  regression, §14.7.5.4's parent tree at 4.5%, is inflation too — of object streams the drawing
  path never touches.
- **Carry an image and its sampling intent to the backends, rather than a finished raster.** One
  `pdf-render` change unblocks three items, which is why they are one question: reduction happens
  at *decode* resolution today, a mask of a very different size is bounded rather than composited
  at device resolution (ADR 0024, `issue16263.pdf` still trips it), and the JPEG 2000 decoder
  cannot be given a target resolution, so `issue19517.pdf` is refused for being 212 megapixels
  where the format's own answer is to decode a lower resolution level. All three need the scale a
  page is about to be drawn at to reach `image.rs`, which the display list deliberately does not
  carry.

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 15 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. **Not one of them is above its bound any more** — the list starts at 0.85 — which is
what the seventy-fifth session's eight left behind. **Read trap 9 before starting**, because an entry may be any of its three shapes, and
checking costs a web search of the other project's source.

**Rank the list before opening anything, by our worst measurement over the bound it is held to
— the largest of mean, worst tile and SSIM.** The fifty-first session did that and the top of
the list was a different *kind* of thing from the rest: `tiling-pattern-large-steps.pdf` at
**25.7×** against a 3.2× runner-up, and it was a rule nobody had implemented rather than a page
needing a careful eye (ADR 0056). **It paid a second time in the sixty-first**: the 1.81 at the
top was `issue3694_reduced.pdf`, and what was wrong with it was the device transform rather than
anything on the page — 11 pages agreed after one line (ADR 0064). The ranking as it stands after
that, five entries lighter:

| | ratio | |
|---|---|---|
| `issue7891_bc1.pdf` page 1 | 1.78 | tile 10.76 against 6.04. **Measured in the sixty-first session and not a defect**: one word inside a luminosity soft mask over a 676×436 image reduced 2.8-fold, five renderers' column centroids spanning 0.76 px, the voting pair 0.25 apart, and our own two resampling strategies moving the raster without moving a printed metric. Trap 12. |
| the eight now in `CONTRADICTED_GLYPH_EDGES` | 1.61 down to 1.08 | **measured in the seventy-fifth session and one population**: each fails on mean alone, and the page's total ink is within half a level of both references the gate votes with. The table is in `oracle.rs`. |
| the remaining 14 | 0.85 down to 0.22 | text pages against two references that share `FreeType`, none of them above its bound |

**`colors.pdf` pages 1 and 2 left this list in the sixty-eighth session and are not fixed.**
They are grids of flat swatches whose interiors all five renderers agree about to the byte, and
whose *boundaries* put the five on a spectrum of edge softness — `poppler` hardest, then
`ghostscript`, then `mupdf` and `hayro`, then us. §10.7.4 asks for the hard edge ("painting any
pixel whose half-open square region intersects the shape, no matter how small the intersection
is"), this tree's anti-aliasing is the first of that subclause's four documented departures, and
the pair the gate votes with is the pair nearest the clause. So the departure now has a corpus
witness and its own group, `CONTRADICTED_ANTIALIASED_EDGES`. **A page can be contradicted by a
departure this project decided on purpose**, and that is a different thing from an unexplained
one.

`issue6231_1.pdf` was the 3.17 at the top of this list until the fifty-second session; it was a
whole surface drawn 180 points from where it belonged (ADR 0057). **A worst tile far above its
bound with a small mean is the signature worth chasing**: it is a
region drawn by one implementation and not by another, and on a large page the mean hides it.
**And the signature that named the sixty-first session's defect was different again** — a
reference whose *raster is the same size as ours* putting the content somewhere else. Compare
the sizes before believing "one pixel out" is about rounding.

The one cause that was identified, measured and live is **closed**: the subdivision lattice
took `mesh_shading_empty.pdf`, `issue2948.pdf` and `issue18816.pdf` with it in the forty-third
session (ADR 0051). Its entry had said, for fifteen sessions, that closing it "needs a Gouraud
rasteriser in **both** backends, since the cross-backend scenes hold them to identical
pixels" — right about the requirement, wrong about the difficulty. One shared raster satisfies
that constraint better than two implementations could and is *less* code than what it replaced.
**Measure an entry before believing its label, including a label written here** — and price the
work before believing a reason not to do it.

Three entries that used to be here are the argument for spending the hour, because none was one
page's problem. `issue20504.pdf` was worth **15 of the 81**: it looked like one page's
`/Differences` quirk and was a whole subclause (ADR 0015). `close-path-bug.pdf` looked like one
page's closed path and was **every dashed line in every document**. `issue11279.pdf` looked like
one page and was §8.10.1 step c) — a form XObject's `/BBox` clipping nothing, on every form since
the first one. Against that, four `knockout_*.pdf` entries left this list by starting to *report*
rather than by being fixed. The only way to find out which kind an entry is, is to open the
artefact: `<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side
strip and a difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how ADR 0012
started. And principle 5 is not suspended by a list: each entry is a question to take to the
specification, and "make it match mupdf" is exactly the failure this project forbids.

### 2. The features the corpus still names

- **A password prompt** (8 documents) is all that is left of encryption, and it is not a clause:
  §7.6.4.1 says "the interactive PDF processor should prompt for a password" and
  `Document::open_with_password` already takes one. It needs a dialogue, a retry loop and a
  decision about where a wrong password is reported — `viewer-ui` work that nothing else on this
  list depends on.
- **§12.5.6.23's redaction overlay and a widget's `/R` were both closed in the hundred-and-fifth
  session, and only one of them by writing code** (ADR 0095). Table 192's `/R` is drawn, in a box
  whose sides are `/Rect`'s swapped so that §12.7.4.3's layout sees the width the text has.
  Table 195's overlay entries are *not* an appearance at all: every one says "after the affected
  content has been removed", and removal is a rewrite of the document.
- **§12.5.6.10's text markup appearances landed in the thirty-fourth session** and are worth
  reading about before the next refusal is written: the clause states the mark, the region, the
  orientation and the colour, and leaves a thickness, where the refusal that stood for thirteen
  sessions said it states nothing (ADR 0043).
- **Colour-managing an image in parallel** is what the twenty-sixth session left behind rather
  than a clause gap. An `ICCBased` image is now converted through its profile (ADR 0035), which
  is work that was not being done, and interpreting `issue19971.pdf`'s 3.4-megapixel photograph
  went from 30 ms to 120 ms. The loop is embarrassingly parallel apart from its memo, one cache
  per row band would keep it exact, and this tree already has rayon. Nobody has tried it, and
  the sixteenth session's lesson about benchmarks that measure nothing applies.
- **Predefined `CMap`s** (12 documents) are a decision about vendoring third-party data and its
  licence, not an algorithm. **Vertical writing** (4) is §9.2.4's `/W2` metrics rather than §9.7.
  **Type 1 fonts landed in the thirty-first session** and were the opposite of small: the entry
  above them said "no corpus page one reaches one", and 57 do.

### 3. Where the time went, and where it still goes

**There is one fair thing to measure against.** Every other renderer here is C, so a timing
difference against `poppler` confounds the language, the allocator and thirty years of tuning.
`hayro` is Rust, forbids unsafe as we do, and rasterises on the CPU single-threaded as we do.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each file
with both, alternating, best of N.

Measured again in the **seventy-third** session, on either side of one change in one sitting —
which is the only way this number means anything:

| | hundred-and-nineteenth | hundred-and-sixth | ninety-ninth | seventy-third | sixty-fifth | fifty-eighth |
|---|---|---|---|---|---|---|
| total, ours | **7.08 s** over 864 complete pages | 6.99 s over 862 | 7.08 s over 859 | 6.91 s over 858 | 6.20 s over 852 | 7.13 s over 852 |
| total, `hayro` | 41.28 s | 39.59 s | 49.03 s | 41.87 s | 34.93 s | 39.03 s |
| **median page** | **2.13×** slower | 2.14× | 2.15× | 2.14× | 2.15× | 2.29× |
| worst page | 65×, `issue19176.pdf` at 1.06 ms against 16.3 µs — a 9x11-point page where the absolute numbers are too small to mean anything | 68× | 50× | 63× | 56× | |

**The hundred-and-nineteenth session's 7.08 s is not a regression against the 6.99 s beside it
and is not comparable to it**, which is this table's standing caution rather than a new one: two
more complete pages, a different afternoon, and `hayro`'s own total moved 39.59 s → 41.28 s with
nothing in this tree touching it. Thirteen sessions of reading went into the interval and the one
that touched a drawing path — §11.6.7's implicit group — is reached by no corpus page at all.

**The hundred-and-sixth session's 6.99 s fell while the page count *rose*** — three pages joined
the complete set in the hundred-and-third (ADR 0093) and the total still went down, which is that
session's 2.5% showing up in aggregate. `hayro`'s total moved 49.03 s → 39.59 s with nothing here
touching it, the third such swing this file has recorded.

**The ninety-ninth session's 7.08 s is not a regression against the 6.91 s beside it and is not
comparable to it**, for the reason the paragraph below already gives about that pair: different
afternoon, one more complete page, and `hayro`'s own total moved 41.87 s → 49.03 s between the
same two runs. Ten sessions of new readers went into the interval and none of them touched a
rendering path, which the oracle confirms by not moving. **Our own total fell 7.13 s → 6.20 s in the sixty-fifth session**, and the two sessions between
added features rather than removing work, so the fall is ADR 0068's: a ramp now hands a
rasteriser the stops it cannot compute for itself.

**The 6.91 s above is not a regression against that 6.20 s and is not comparable to it**: it
counts 858 pages rather than 852, on a machine that had just built the workspace, and the
seventy-third session's own A/B — stash, rebuild, measure, restore, measure — put ADR 0069's
change at 6.92 s before and 6.91 s after. Over *all* 946 pages the same pair is 8.19 s and
7.55 s, and essentially the whole difference is `bug1721218_reduced.pdf`, which is incomplete and
so is not in the first total at all. **Quote a total against a total taken the same afternoon.**

**Read the two totals before reading the median.** Our total *fell* by 14% and `hayro`'s fell by
**65%**, which is far outside the run-to-run variance this file has recorded for it (4.5× to
5.8× on the same corpus). Something about the other program or its build changed between the two
measurements; nothing in this tree can say what, and the honest conclusion is that **the median
ratio moved because the denominator did**. The number to trust across sessions is our own total,
and it went down.

**The totals and the median answer different questions and only quoting both is honest.** In
aggregate we are 5.5× faster, because their distribution has a long tail and ours no longer does.

**Per-change interpretation costs, by callgrind on `examples/callgrind_interpret`**, kept
because they are the only honest scale for "what a feature costs": text rendering modes +0.46%,
masking +0.12%, soft masks +0.05%, composite fonts +0.44%, constructed appearances +0.34%,
variable text +0.31%, and §8.4 and §8.5's path rules **−0.21%** — collapsing consecutive `m`
operators leaves fewer commands to build than the rules cost to apply. `callgrind_rasterise.rs`
exists because the first example stops at the display list, so a backend change measures as
exactly zero there.

**Where interpretation goes on the median page**, `callgrind_interpret` over the specification's
own page, re-measured in the fifty-eighth session:

| | share |
|---|---|
| `zlib_rs::inflate` | **28.0%** |
| `Interpreter::show_text` | 6.5% |
| `Lexer::next_token` | 5.1% |
| `inflate_table` | 4.0% |
| `read_fonts::ps::agl::name_to_char` | 3.2% |

**Nearly a third of interpreting a page is inflating it.** That is `flate2` doing its job and is
the answer for the typical page; the guess this file carried before the forty-sixth session had
been "parsing, font loading and per-page setup". The one item that was *ours* and avoidable was
the AGL: §9.10.2's second method searched a four-thousand-entry list per character shown, in a
font with at most 256 codes, and a cache took the whole of interpretation from 2 013.8 M
instructions to 1 989.1 M.

**Interpretation costs 2 110.6 M today**, and the rise over the fifty-eighth session's 1 989.1 M
is one feature — measured rather than guessed, by stubbing it out and running the same page:

| | instructions |
|---|---|
| at the start of the seventieth-to-seventy-ninth run (`dd49639`) | 1 989.5 M |
| with §14.7.5.4's parent tree stubbed out | 2 003.9 M — **everything else those sessions added costs 0.7% together** |
| as the seventy-ninth session shipped it (`a9d423e`) | 2 094.9 M as measured then, **2 103.8 M when the same commit was rebuilt in the eighty-sixth** |
| the hundred-and-sixth session | 2 110.6 M — +0.33% over that rebuilt baseline, at the drift floor |
| today, after thirteen more sessions | **2 119.5 M** — +0.42% over the same baseline, still at it |

**0.42% of drift on one commit with no code between two measurements** is the second time this
project has caught its own performance number moving under it, and it widens the 0.23% floor the
sixtieth session recorded. Quote a measurement against one taken the same afternoon.

**The sixtieth session added §14.9's four accessibility entries and measured 2 099.5 M against
2 099.8 M** for the same page built in the same sitting — no cost, because the entries are read
from a property list the `BDC` handler already opens and an untagged page allocates nothing.

So **reading a page's structure costs 4.5% of interpreting it**, and almost none of that is the
tree descent: the parent tree's nodes carry `/Limits`, so a lookup visits about one node per
level. It is that structure elements live in **object streams the drawing path never touches**,
and reaching them inflates those streams. A page that states no `/StructParents` pays one
dictionary lookup — 885 of the 974 corpus documents. The cost is on `structure.rs`'s own doc
comment with the two ways to get it back, both API changes, neither to be made without a second
measurement.

**Still open, and the largest items.** This profile predates two fixes and its shading half is
still live:

That profile was re-measured in the sixty-fifth session and **its diagnosis was right and had
never been acted on**. `bug1721218_reduced.pdf` was **144.05 G instructions** with
`tiny_skia::pipeline::lowp::gradient` at **68%** — the file's own sentence, that a `Ramp` carries
256 samples so a shading becomes a 256-stop gradient the rasteriser scans per pixel batch. A ramp
now drops every stop that lies on the line its neighbours draw, to within half a level in eight
bits, so a `/FunctionType 2` with `/N 1` is **two stops instead of 256** (ADR 0068):

| on `bug1721218_reduced.pdf` | before | after |
|---|---|---|
| whole page | 144.05 G | **54.05 G** |
| `tiny_skia::pipeline::lowp::gradient` | 97.9 G (68%) | 15.8 G (29%) |
| `Function::parse` | 3.6 G (2.5%) | 3.6 G (6.7%) |

Two of the old profile's claims did not survive the re-measurement, and both are worth keeping
as warnings. `Function::parse` was recorded at **23.2%** and was 2.5% of the larger page — the
profile had aged past its conclusion.

**And the second correction was itself wrong, which is the sharper lesson.** The file asked
whether the page's "3576" shadings are distinct functions or one re-parsed; the sixty-fifth
session instrumented the *pattern* path, counted eight, and wrote "parsing was never the cost".
Instrumenting `Function::parse` itself counts **7000+ calls**, and instrumenting both call sites
says why: the pattern path runs once and `sh` runs 3576 times. **A count taken at one call site
is not a count** — the same shape as the `unreviewed` rows counted over the families a session
had touched. The seventy-third session cached the built shading per object and the page went
**53.96 G → 43.13 G** (ADR 0069):

| on `bug1721218_reduced.pdf` | before ADR 0069 | after |
|---|---|---|
| whole page | 53.96 G | **43.13 G** |
| `Function::parse` | 3.61 G (6.7%) | gone |
| `Function::eval` | 2.23 G (4.1%) | gone |
| `pdf_model::shading::ramp` | 1.72 G (3.2%) | gone |

What is left on that page, in order: `tiny_skia::pipeline::lowp::gradient` 36.6%,
`Mask::intersect_path` 8.1%, `build_soft_mask` 8.0%, `fill_path_impl` 6.4%, `calloc` 4.5% —
re-measured in the hundred-and-thirteenth session at 43.15 G, unchanged. **This file used to say
the next item was the group buffer and that `calloc` and `Mask::intersect_path` were an eighth of
it; both halves of that were wrong.** `calloc`'s callers are `Mask::intersect_path` 3.08%,
`Mask::new` 1.31% and `Pixmap::new` **0.14%** — the group buffer is the last of those. What the
two mask lines add up to is one *item*, not two: **`MaskCache::get` is 24.34% of the page**, 3608
chains per render with no eviction and no duplication worth removing. ADR 0103. One caution about the old table's fourth row: `to_rgb_at` was 2.6% when `CalGray`
was a pass-through; it now runs a Bradford adaptation and a matrix per colour, and per *sample*
for a Cal-space image.

Two fixes are worth carrying as patterns, and both are in Habits: unpacking JPEG output cost 6.89
G instructions until two paired `chunks_exact` iterators took it to 1.25 G — **the safety habits
this project enforces everywhere are expensive in a loop that runs per pixel** — and
`Triangle::is_subpixel` took `personwithdog.pdf` from 17.3 s to 1.06 s *while* moving every mesh
page closer to the references.

### 4. Reproducing the numbers above

The oracle survey is `oracle.rs` and the corpus counts are `corpus.rs`; both print their evidence
per document. The ledger's counts come from `cargo test -p conformance -- --nocapture`.

Two classification counts are still throwaway, deliberately — scratch-quality diagnostics do not
belong in a repository held to `clippy::pedantic`. **Whether a page's fonts are embedded** walks
each `/Font` resource and its `/DescendantFonts` for `/FontFile`, `/FontFile2` or `/FontFile3`.
**The annotation subtype breakdown** comes free from the corpus gate's own output:
`grep -o 'Annotation { detail: "[^"]*"' | sort | uniq -c`.

### 5. What the three gates report today

Corpus, ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only go down, except where a
rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| needs a password | 8 | §7.6.4.1's prompt is the missing piece, not the clause |
| encrypted beyond this reader | 2 | 1 is `/R` 5, which the standard states no algorithm for; 1 is a file whose `/Encrypt` does not resolve to a dictionary |
| no page one | 5 | 11 until the hundred-and-seventh session, whose two recovery rules took six of them (ADR 0097). What is left is refused by every reference too or is not a PDF defect at all: one prototypes the `/BrotliDecode` filter the PDF Association is standardising, three are fuzzer crashers kept as regression fixtures, and one is a Firefox worker-shutdown bug |
| draws incompletely | 90 | Counted by each document's *first* report; 20 left it in the thirty-first session when `/FontFile` began to be read, 4 in the thirty-second when the last three bit depths did, 2 in the seventy-first when §11.4.6's knockout was drawn, 5 in the seventy-second when §9.3.8 and §11.6.2 followed it, 1 in the eighty-fifth when §12.5.6.7's leader lines stopped being refused, and 3 in the hundred-and-third when §11.4.4's NOTE 5 stopped a non-isolated group being built (ADR 0093) |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

- **The `Content` row was 10 and is 1** (ADR 0031). Nine of
  those ten content reports were an encrypted `/Contents` refusing to inflate because it was
  ciphertext, and three of the operator reports were the same ciphertext lexing as operator names.
  Six of those twelve documents now draw with nothing reported and six say they need a password.
  Nothing on either row is a feature.
- **The annotation row was 67, then 24, then 17, and is 10 reports over 10 documents** — counted
  from the gate's own output in the eighty-fifth session, where this file had said 17 with a
  breakdown that no longer matched anything: 5 a `/DA` naming a font the `/DR` does not define
  (4 `FreeText`, 1 `Widget`), 1 a check box the file calls on with no mark stated for it, 1 an
  appearance stream with no `/BBox`, 1 an unknown subtype whose clause states no geometry, 1 a
  `Line` whose line endings state no size (Table 179), 1 an `Ink` with no usable `/Rect`.
  **Nothing on it is a `/NeedAppearances`, nothing is a field value, and nothing is text markup
  any more.**
- **The font row was 100 before ADR 0029, then 67, and is 42 documents** — recounted from the gate's own output in the eighty-ninth session. Nothing on it is a
  `CMap` question and nothing on it is a Type 1 program: what is left is fonts with no
  `/ToUnicode` so a substitute cannot be addressed, substitutes that draw none of their declared
  codes, the 15 naming a predefined `CMap`, the 4 asking for vertical writing, and malformed
  programs.
- **The operator row was 33** until the text rendering modes landed, and is 8. Nothing on it is a
  feature: `BT` without `ET`, `BDC` without `EMC`, and the byte soup a fuzzed content stream lexes
  as operator names.
- **The image row was 161 before JBIG2 and JPEG 2000, and is 8** — one image apiece, recounted in
  the eighty-ninth session, and nothing on it is a feature: 4 malformed streams (three JPEGs whose
  dimensions contradict their dictionary or whose headers do not parse, one with no `/Width`), one
  `/Mask` that is not an image mask, one JBIG2 segment type ISO/IEC 14492 does not define, one
  212-megapixel JPEG 2000 scan, and one `/SMask` of 34862x4332 against a 2x2 image.
- **The shading row is gone.** It held 28 documents and every one was a soft mask in an
  `/ExtGState`, filed under shading because nothing else fitted.

**Text, ratcheted in `crates/pdf-model/tests/text_extraction.rs`** and new in the sixty-third
session (ADR 0066): `Interpretation::text` against `pdftotext` over the same 974 documents,
30 seconds, **97.9% of the reference's words** over the pages we draw completely, with **44** named
below the 0.90 floor — counted from the gate's own output in the eightieth session, where this
file had said 43 in one place and 46 in another and the breakdown below summed to 45. About 31
of them are fonts where all three of §9.10.2's methods fail *and* whose program names nothing
either, 7 are right-to-left text read back in painting order — **not §14.8.2.5.3's
`ReversedChars`, which no corpus document writes**, measured in the eighty-third session — 1 is
a Symbol
font naming Greek glyphs, and 6 are undiagnosed — **all six of which agree with the reference
consensus on pixels**, so the whole of what is left on this list is naming and none of it is a
drawing defect. The 14 specification PDFs keep their own 0.99 floor and still score 100%.

Oracle, ratcheted in `crates/pdf-model/tests/oracle.rs` by name and in both directions.

| of the 1666 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 840 | |
| **contradicted** | **65** | 4 page rounding, 2 our own anti-aliasing at a shape's edge (§10.7.4's first departure, measured), **8 pages of glyph edges whose ink matches the consensus to half a level** (measured, seventy-fifth session), 7 a shared JBIG2 decoder, 1 a shared *gap*, 3 a link border two references do not draw for two unrelated reasons, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, 4 a `DeviceCMYK` conversion (ADR 0048), 2 a reference that drew nothing (ADR 0049), 1 a CID width two references space inconsistently, 1 a negative line width, 14 substituted fonts, **15 unexplained** |
| ambiguous | 750 | the references disagree with each other; 372 are two long books set in fonts nobody embedded |
| our page geometry differs | 0 | all three were `/UserUnit`, applied in the twenty-ninth session (ADR 0038) |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 128 incomplete pages are compared and printed too, but cannot fail the gate: a page we already
say we cannot draw is expected to differ. **The denominator moves in both directions on purpose**:
it grows when reports stop firing (46 pages in the twenty-first session) and shrinks when a
silence ends (8 in the seventeenth, 43 in the eighth). A report should never be reached for as a
way of making a contradiction go away — see trap 5 for the exchange.

**Where the oracle's time goes, measured and printed by the gate itself.** It used to be roughly
1000–1300 s of processor time in the three external renderers against 45–55 s in ours — so **the
gate was essentially a measurement of `pdftoppm`, `mutool` and `gs`**. ADR 0020 is the answer, and
the run is now ~34 s with ~23 s in them at a 99.7% hit rate, every verdict unchanged, which was
checked by running the whole corpus both ways. What is left is ours: roughly 600 s of processor
time over 24 cores on our own render, the comparison and the artefacts — the SSIM and heatmaps for
the thousand pages that are not agreement — so if 34 s ever becomes the constraint, that is where
to look and not at the subprocesses.

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a document
that never returns hangs the suite rather than failing it. A real budget has to live inside the
interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each document on stderr as it
starts and finishes, which is how a hang gets identified from a killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) holding those 974 PDFs and 459
more behind link files. It is optional to clone — every test that uses it reports being skipped
rather than failing — but the ratchets only mean anything where it is present, so CI must have it.

## Habits these sessions earned

Each was paid for once. The traps above are about code; these are about how to work. They are
grouped, and every one keeps the anchor that makes it checkable — the file, the clause or the
number that taught it.

### Reading the specification

**A subclause is a checklist; check the code against it, not the code against itself.** §9.6.5.4
names five routes from a code to a glyph and the code that stood in for it implemented one and a
half — self-consistent, commented, and right about every document anyone had opened.

**Read the whole subclause before believing the sentence that answered your question.**
§12.7.4.3 opens by describing a processor *constructing* an appearance stream and closes by
describing it *splicing* one; only the closing sentence says what happens to a stored stream.

**A clause read and dismissed is worth as much as one implemented**, and costs a minute against
the 20 to 60 a real review costs. The ledger has `inapplicable` and `writer-side` for exactly
that; treating it as a to-do list of features is the trap.

**A cheap family review is where the expensive findings are.** Clause 10 was picked because most
of it was expected to be `inapplicable`. Nineteen rows were; one was §10.4.2.5.

**A claim that the standard is silent is a claim about the whole standard, and it is checkable.**
Thirty-two sessions asserted that ISO 32000-2 defines no `DeviceCMYK` → RGB conversion; §10.4.2.5
is titled "Conversion from DeviceCMYK to DeviceRGB". Twice a recorded silence has been a clause
four subclauses from one the tree cites constantly. `grep -n '^## '` the titles in `doc/md/`
first; it takes a minute.

**"The clause says nothing" and "the clause says the opposite" are different findings, and only
one is a licence.** Image reduction was recorded as unspecified from §8.9.5.3, which is about
magnification; §10.7.4 says "there shall not be averaging over the pixel area". Same code, but
only the second produces a *departure*, which must be argued and costed.

**A departure is only honest once you have looked for the others.** Reading the rest of §10.7.4
found that anti-aliasing had departed from its first rule since the first commit. One departure
looks like a compromise; three in one subclause, all in the same direction, is a reading.

**Where the standard defines nothing, refusing is a result.** `issue6621.pdf`'s `/Mask` is a
one-bit greyscale image where Table 87 requires an image mask; both readings damage some file, so
neither, and the entry is named.

**Where the standard defers to another document, the deferral is a citation.** §9.7.5.3 hands a
`CMap`'s syntax to Adobe Technical Note #5014, so ISO 32000-2 never states that `notdefrange`
gives its whole range one CID while `cidrange` numbers upward.

**A default written in a table is not a suggestion, and a comment arguing for a nicer one is a
preference wearing a reason.** `/MissingWidth` defaults to 0 (Table 120); this tree used half an
em, and it cost `issue7439.pdf` six half-ems of invented space in one line.

**A presence condition is not a restriction on meaning.** Table 115 says `/CIDToGIDMap` is
"Required for Type 2 CIDFonts with embedded font programs" and then says what it *means*; reading
the first sentence as bounding the second drew one page as garbage.

**A rule about how something is *encoded*, implemented as a rule about its value, is invisible
forever.** §9.3.3 applies word spacing to "the single-byte character code 32", not to any code
numerically 32 — so every `Identity-H` string containing `00 20` had its line pushed right.

**Where two subclauses each condition a branch on one of two flags, the clause that defines the
flags breaks the tie.** §9.6.5.4 cannot decide a font setting Symbolic *and* Nonsymbolic; §9.8.2
calls the pair "a historical accident" and says to check Symbolic.

**One dictionary, two clauses, and only the second says who wins.** §8.9.6 defines an image's
`/Mask`; that an `/SMask` "shall override any explicit or colour key mask" is in §11.6.4.3.

**Two callers of one clause can use disjoint halves of it, and the half nobody uses is not
implemented.** §14.6.2 gives a property list two forms and §8.11's optional content *cannot* use
the inline one, so fifteen sessions of optional-content work proved nothing about it.

**When two clauses disagree, ask which reading makes a file's own words mean nothing.** §12.5.2
and §12.5.5 disagree about `/CA` beside an appearance stream; honouring both applies
`highlight.pdf`'s 0.8 twice and gives 0.64.

**The clause can tell you two readers are one algorithm.** §9.6.2.1's NOTE 1 calls a CFF "an
alternative, more compact but functionally equivalent representation of a Type 1 font program",
which is why `cff.rs` and `type1.rs` share a type rather than a copy of the rules. That NOTE has
now settled three separate design questions.

**A clause one analogy away is still the clause.** §9.9's Table 124 forbids a `/FontFile` on a
CIDFont, which answers what a *writer* may do and not what a reader does with one; §9.7.4.2 and
§9.6.2.1 between them say exactly what to do with one.

**A rule that changes nothing today can become load-bearing tomorrow, and the trigger is the
clause beside it.** Table 58's rule that one `m` overrides the previous changed no pixel until
§8.5.3.2 made a single-point subpath a dot — then 205 unwanted dots on one page.

**A clause about the whole page can be invisible until one construction needs it.** §11.4.7 is
two paragraphs saying the page is an isolated group, and it decides how every blend mode in every
document composites against paper. It survived three reviews of clause 11's other families.

**A default of `true` on an entry nobody implemented is a gap on every file that uses the
feature.** Table 217's `/PreserveRB` defaults to true, so §8.11.4.5's `/RBGroups` — recorded as
unread for the project's whole life, and correctly, because that clause gives it no part in the
*initial* state — became load-bearing the moment an action could change a state.

**Ask what a feature looks like when its parameters are not their defaults.** Under `Identity-H`
with `/CIDToGIDMap /Identity`, §9.7's two mappings both collapse to nothing, so nineteen sessions
never asked what either one *is*. `Tk`'s initial value is the same lesson from the other end: a
parameter whose default is the unimplemented behaviour is a gap on every page in the world.

**A rule whose common case is the identity is a rule nobody tests, and the test written beside it
will agree with it.** §7.6.4.3.2 step (a) appends "the first 32 − n bytes of the padding string";
for the *empty* password the wrong reading gives the same 32 bytes, so nineteen documents opened
and every document with a password was refused.

**Ask what the clause requires of *this* device before deciding it is a gap.** Overprinting was 63
documents and six `silent` rows until Table 146 was read against this device's colourants. **A gap
sized by a corpus is a hypothesis about a clause.**

**The standard sometimes states answers rather than rules, and those are the tests to write.**
§12.4.2 gives no algorithm for Roman numerals — it gives nine labels beside a tree, and each of
the four mistakes a first implementation makes fails at least one of them.

**A clause that states an algorithm is a clause that can audit a corpus.** §12.3.3 defines
`/Count` in three steps and every document states the result, so running the steps checks the
reader against 146 producers at once — 144 agree, and the two that do not contradict themselves.

**§6.3.2.2 ranks what a corpus cannot.** What you measure decides what you build: two gates that
take the pdf.js corpus as their universe produce a demand curve, which cannot rank a requirement
no file exercises and converges on "done" when the last file goes green.

### Judging against other implementations

**Compare the references with each other before opening a page.** Four unexplained contradicted
pages sorted themselves into one group from a table of pairwise means — two clusters of two is a
fact about the question, not about the page.

**Rank the suspects by a ratio, not a distance** — our worst measurement over the bound it is
held to. Five times now it has chosen the next item before an artefact was opened: at 25.7×
against a 3.2× runner-up the top of the list was a rule nobody had implemented, and at 1.81 it
was the device transform, worth 11 pages.

**Before believing "one pixel out" is a rounding difference, compare the raster sizes.** Four
renderers put `issue3694_reduced.pdf`'s type a row above ours and one of them produced a raster
*the same size as ours*, which no disagreement about how many rows a page gets can explain.
A whole group of eight was named for page rounding and half of it was our own y flip (ADR
0064).

**When a metric accuses you, find one that measures the same thing differently.** Eight
contradicted text pages failed on mean absolute difference and passed every other bound; the
page's *total ink* — the mean of its channels, which counts how much was painted without caring
where — put us within half a level of both references the gate votes with, and `ghostscript`
further away than us on five of them. One number, computed from artefacts the gate had already
written, turned eight questions into one population.

**An inconsistency inside a reference's own output outranks any distance from it.** Two renderers
spacing one line of one font at two different widths cannot both be reading the document's `/W`;
no tolerance or vote was needed, only the ink columns.

**Agreement with one reference is not evidence.** `mupdf` drew what we drew on the tiling `/BBox`
page, which felt like support and was one implementation reading the clause as we had.

**"Both readers fail the same way" is agreement about a symptom, not about a cause.** `poppler`
reporting the same broken flate stream was taken as proof that `issue19484_1.pdf` was damaged;
both readers were deriving the same wrong key.

**Two references agreeing is evidence — once you can say what they agree *about*.** Trap 9 has
the three shapes where it means nothing; the converse is real, and the reading that explained
`poppler` and `mupdf` blanking a page was the one the clause states.

**Two references against two is not a tie and not a vote — it is a question with an answer.**
`Type3WordSpacing.pdf` splits them over a `d1` glyph's stroke colour and Table 111 settles it.

**An unimplemented feature has a default, and the default is usually "draw it".** That is a more
common failure of the oracle's premise than shared code: `mupdf`'s `FIXME: Calculate visibility
from array` took minutes to find and settled a page that looked like three-against-one.

**Point your own instrument at their data.** "Do `mupdf` and `ghostscript` agree because they
share a colour profile" reads like a question about two other projects and is a question about
one file on this machine, which `pdf_model::icc` answers in one run.

**Ask the reference the same question you asked yourself.** Two of three renderers were being
asked for the media box while we rendered the crop box, which put 54 documents beyond comparison.

**A test corpus has a bibliography, and nobody had read it.** Every pdf.js file is named after an
issue that says what is wrong with it in the words of whoever hit it. It costs one web fetch per
file and it corrected a written conclusion on the first afternoon of use.

**A corpus document can be a conformance test, and then it outranks every renderer.**
`issue14256.pdf` draws one picture eight ways and comments each case; eight images that must
agree with *each other* need no reference.

**A corpus document can check a decoder against itself, and it beats a second decoder.** An LZW
image must decode to exactly `width × height` bytes; 96 documents encode one image ninety-six
ways. Ask **what does this file already say about itself?** — a font's table directory states
`glyf`'s length and `loca`'s last entry states it again, which is how a byte-swapped
`indexToLocFormat` was repaired from the font's own bytes.

**Look at what a corpus file is *for* before filing it under a group** — and a page may be
contradicted for a reason other than the one its group names, which is how ADR 0012 started.

### Tests, gates and reports

**A test asserted through the accessor that normalises the thing being tested is not a test.**
§7.3.7's rule about a null entry was checked through `Document::get_key`, which answers `Null`
for an absent key — both sides of the assertion were the same function.

**A discriminating test has to discriminate; check by breaking the thing.** The first assertion
written for the `/FontFile` fix counted contours and passed under both readings.

**A test that skips silently is worse than no test.** `tests/ccitt.rs` printed "skipped" and
passed while checking nothing. A missing corpus is a skip; a present corpus that lacks what the
test needs is a **panic**.

**A test written to isolate one rule finds what a corpus cannot** (trap 8), and a corpus varying
one thing while holding another fixed is stating a testable invariant.

**A gap measured on both sides is a fact; measured on one side it is an accusation.** "22 of 106
named destinations resolve" reads as a broken reader until you check that every key which *is* in
a table is found — then it reads as five files with no destination table.

**A gate cannot ratchet what has no consumer.** Destinations were `partial` for three sessions
with `/OpenAction` as their only user, and the number that mattered could not be measured until
something asked the question.

**Fixing an instrument can be worth a feature.** One line — what `has_text` asks — moved 25 pages
into the oracle's judged set and showed one of them was drawing nothing.

**A page can leave the contradicted list without a pixel moving.** The oracle picks a tolerance
class from whether text read back, so anything improving extraction can loosen a bound. Take the
raster's digest before writing "fixed". **And the converse: a page can leave with pixels moving
and still be wrong.** `issue20232.pdf` agreed once the y flip was fixed and still draws `56`
where three references draw `⌀56`; its list is kept, empty, saying so. The gate answers "within
the bound the references set for each other", which on a 595×842 drawing one glyph never
reached.

**A page can be visibly wrong inside a verdict the gate cannot fail on.** `issue7406.pdf` drew a
JPEG cyan-on-black against four references and its verdict was `ambiguous` before and after —
46% of the judged set lives there and nothing watches it.

**A report has a price, paid in gated pages.** Ask "on how many pages can this actually be
seen?", and make the condition answer that rather than a looser question. Trap 11 has the form.

**Print what a condition matched before trusting its count.** Twice a report's first draft was
defensible from the clause and wrong about the corpus, and one `eprintln!` settled it.

**Measure the corpus before choosing between reporting a gap and closing it.** Every `/Decode`
array in all 974 documents is Table 88's default or its exact reversal, so the report that was
about to be written would have fired on nothing.

**A report that arrives with a fix is worth reading twice.** Both new reports in the fifty-third
session came from drawing something previously ignored: one names a real gap, one names a cycle
the document was built to contain. Neither is a regression and both look like one in the count.

**A "not implemented" count of zero can mean "nothing reports it".** `/FontFile` was recorded at
zero corpus documents while 57 embedded one and drew a substitute in silence. Before writing a
zero, ask what the code does when the feature is absent.

**Build the strong gate, then let its own output tell you it is wrong.** A table-attribution
checker failed fourteen of twenty-five references and all fourteen were correct writing; what
shipped asserts the weaker true thing and *prints* every cited table's title.

**A citation nothing checks is a citation that rots**, and it kept finding errors after the
obvious ones were fixed — corrected `/Mask` citations were still wrong, because §8.9.6.2 is
stencil masking and `/Mask` naming another image is §8.9.6.3.

**Read what an entry's *value* means before branching on whether it is there.** §12.5.6.7's
`/LL` was refused on presence, and the corpus document it named states `/LL 0` — Table 178's own
"no leader lines". The mirror of Table 115's presence condition, which was read as a condition on
meaning.

**A gate that reads one file format checks one file format.** The ledger is 823 notes about
ISO 32000-2 — the densest prose about the standard in the project — and the citation gate read
Rust sources, so none of it was checked. Its first run found three table numbers that are ISO
32000-1's for tables ISO 32000-2 renumbered, and two clause numbers that name nothing. Ask what
a gate's *input set* is, not only what it asserts.

**A `§` means one document, and a citation of a different one reads correctly and checks
correctly.** `RFC 3986 §5.2` is right about the RFC §12.6.4.8 defers to, and ISO 32000-2 has a
§5.2 of its own, so the citation checker found a clause and said nothing. Four spellings were in
the tree. The gate now fails on a `§` preceded by another document's name and says to write
"RFC 3986 section 5.2" instead.

**A bucket that means "we failed" must not also come to mean "you have not told us the
password".** When a ratchet fires on a change you believe in, ask whether the *category* is wrong
before you ask whether the number is.

**A gate's numerator moves when its denominator does, and only one of those is news.** Keep the
denominator beside the numerator, and say which pages moved and why.

**A count taken at one call site is not a count.** "Parsing was never the cost" was written
after instrumenting the pattern path, which runs once on that page while `sh` runs 3576 times —
so the number was right about where it was taken and wrong about the page. Instrument the
*function you are accusing*, not one of its callers.

**A number in this file is a claim, and attributing it is a second claim.** `calloc` was 4.5% of
`bug1721218_reduced.pdf` and this file said it was the transparency group's page-sized pixmap;
`Pixmap::new` is 0.14% and the rest is clip masks. The measurement that was right had never been
*attributed*, and a session was queued to change a coordinate system to save a thousandth of one
page. Ask `callgrind_annotate --tree=caller` who called it. ADR 0103.

**Three plausible optimisations, three counts, three refusals — and counting was cheaper than
any of them.** Eviction tuning, clip deduplication and a rectangular-clip fast path were 0%,
1.3% and 2.5% on the page that motivated them; each took one instrumented run to price.

**A profile ages past its conclusion, and the conclusion is what survives being read.** This
file carried one profile of `bug1721218_reduced.pdf` for nineteen sessions. Re-measured, its
largest item was *four times* the share recorded and its second largest a tenth of it — and the
sentence beside the largest item had named the fix correctly the whole time and nobody had done
it. Re-measure before working from a table, and work from the sentence rather than the number.

**A ratio has two ends, and this file has quoted the wrong one.** The median against `hayro` rose
2.12× → 2.29× while our own total *fell* 14%, because their total fell 65%. Quote the absolute
number you control.

### The ledger, and claims about this tree

**When two clauses describe one mechanism, reviewing one of them leaves the other lying.**
§14.8.5.1 was `silent` while §14.7.6's rows recorded the same attribute code as `implemented`;
§14.7.5's four rows were `silent` beside a `structure::Child` that implemented all of them. Four
instances in ten sessions, and the check is one `grep` for the *other* clause a family cites.

**A retired claim is a string, and strings are greppable.** `CLAUDE.md` principle 5 was rewritten
around the finding that §10.4.2.5 defines the `DeviceCMYK` conversion this project had recorded
as undefined — and three sessions later the retired sentence was still in §8.6.4's ledger row,
§8.6.4.4's, `colour.rs`'s own test and `content.rs`'s `device_space`. When a session disproves a
sentence this tree repeats, the work is done when the *sentence* is gone, not when the code is
right. One `grep` for the old wording. ADR 0101.

**A row whose evidence is a file can be `implemented` for something the file never touches.**
§8.7.4.5.2 is the sharpest instance: `shadings.rs` has fourteen tests and not one of them was a
`/ShadingType 1`, so the row's evidence passed on every run while asserting nothing about the
row. That is what `FILE_ONLY_EVIDENCE_CEILING` counts, stated as a failure mode rather than as a
number.

**A comment that names a refusal outlives the refusal.** `appearance.rs`'s module header listed
§12.5.6.10's four text markup subtypes among the things that "state no mark" for eighty sessions
after the same file started drawing all four, through several reviews of that module. A header is
where a reader learns what a module refuses, which is where a stale refusal does most damage; the
check is one `grep` for the subtype in the same file. ADR 0105.

**A stale row can understate as well as overstate, and only the overstatements have a gate.**
§14.9.4's note said its structure-element half was owed for four sessions after the fifty-sixth
built the parent tree and wired the fallback. Nothing fails when a row claims *less* than the
code does, so the only defence is reading the row when you touch the family. **The
eighty-second session met it at scale**: six rows in one family were understating, §14.7's
saying "none of it is read" two sessions after the tree was built, and eighteen rows left
`silent` in a session that wrote eleven rows' worth of code. A `silent` count is a *lower*
bound on what exists.

**A warning written into a ledger note before the code exists is a warning nobody reads when the
code arrives.** §7.11.2.1's row said a reader that decoded a file specification's bytes as text
"would corrupt a file name in every locale where they are not UTF-8", and three call sites in
`pdf-model` did exactly that for as long as they existed. A row is read when its *clause* is
implemented; nothing points at it when a caller needs one line. ADR 0104.

**A note that gives a reason gives a trigger, and nothing fires it.** §11.6.7's row called
inheriting a pattern's alpha per cell "the closest available approximation while §11.4.6 does not
exist"; §11.4.6 was drawn forty-six sessions later and the approximation stayed. "While X does
not exist" expires the day X lands, and no gate in this project can see that day arrive — so the
`partial` notes are worth reading for their *reasons* rather than only their claims. ADR 0107.

**A ledger note is a hypothesis the gates test, not a conclusion they inherit.** Three
`implemented` rows claimed behaviour the code never had — §8.7.3.1's `/BBox` clipping a cell,
§8.7.2's pattern space inside a form, §14.6.2's "both forms" — each written from the clause during
a review, each costing a visible defect, and each found by the oracle rather than by the ledger.
`FILE_ONLY_EVIDENCE_CEILING` counts the population where that can still hide.

**A ledger row is an entry, and an entry gets measured before it gets believed.** §12.3.2.3's row
priced it as a whole clause of clause 14 and it cost an afternoon. Same failure as
`mesh_shading_empty.pdf`'s entry, and worse: that was a note about somebody else's file.

**A ledger with a status per subclause can find a missing *component*, not only a missing
feature.** Four rows in two clauses named one absent data structure — a name or number tree —
which no clause review would have shown and no corpus document would have asked for.

**A row that names a *blocker* rather than a gap is the class no gate can watch.** It is true
when written, false the day the blocker lands, and nothing re-reads it because nothing changed in
its own clause. §12.6.4.5's `GoToDp` was refused for seventeen sessions because §14.12 was
`unreviewed`, which it had not been for sixty-two. One regular expression over the ledger's notes
— `while … does not exist`, `until …`, `needs §…` — finds them in twenty minutes, and it is worth
running whenever a family lands. ADR 0108.

**An `inapplicable` row decays exactly as a `silent` one does.** §12.7.4.2's field names were
`inapplicable` for eighteen sessions on sound reasoning — a name identifies a field for export,
scripting and the user interface, none of which decides a mark — until §12.6.4.11's hide action
was implemented and a field name decided whether an annotation is drawn. The row means nothing is
owed *by the clauses that reach it today*.

**A count of `silent` rows is a map of a project's shape**, and only as honest as the clauses
that have been read: the count was 2 for eight sessions because clause 12's interactive half was
`unreviewed`. `unreviewed` and `silent` are different admissions.

**A count taken over what you touched is not a count.** This file said clause 7 had no
`unreviewed` row left for six sessions, because the count was taken over the families a session
had *touched*. The check is one line, grouping by leading clause number.

**Reading every clause is not implementing every clause**, and the vocabulary says so: 193
silences and 30 reports are the distance left, now itemised rather than unknown.

**Read this project's own lists for the sentences that admit ignorance, not only the counts.**
"Has not been looked into" sat in `oracle.rs` next to its own answer for many sessions.

**Whatever this file asserts, run it once.** "Clippy clean" was claimed while eleven warnings sat
in the tree, because `allow-panic-in-tests` does not reach an integration test's helpers. The
same rule caught this file's own arithmetic about page counts.

**A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have no
memory-safe implementation" sat in `PLAN.md` as a reason, true when written and false for months.
Anything deferred on an external condition should carry the date it was last verified.

**Price the work before believing a reason not to do it.** `mesh_shading_empty.pdf`'s entry said
for fifteen sessions that closing it needed a Gouraud rasteriser in both backends — true, and one
shared raster satisfies that constraint *better* than two implementations, in less code.

### Measuring

**Wall-clock benchmarks lie under load; count instructions instead.** One change measured as a
24% regression and an 8.5% improvement twenty minutes apart. A/B in one sitting, and measure the
baseline on this machine rather than trusting a number in this file — **which the sixtieth
session then had to do**: `HEAD` rebuilt in a worktree measured 2 099.8 M on the page this file
records as 2 094.9 M, with no code between them. 0.23% is the floor under how finely this number
compares across sessions, and it is above several of the per-feature costs listed below.

**Attribute a regression by removing the suspect, not by reading the profile.** The profile said
the lexer, `token_to_object` and allocation had all grown, which is the *shape* of the extra work
and not its cause; one stubbed field said 96 of the 110 M.

**When a page's error has a suspiciously round size, do the arithmetic.** Seven pixels of
gradient where there should be an edge is 1800 ÷ 256, and that division named the defect before
any clause was opened.

**Profile before believing an explanation, even one whose arithmetic matches.** A 48-second page
was attributed to clip masks with `3576 × 485 kB = 1.7 GB`, which was exactly the memory held and
silent about the time: callgrind put the masks under 4% and the gradient stage at 78.9%.

**A suspiciously clean measurement is a reason to check the instrument.** Four callgrind numbers
flat to four significant figures across pages doing obviously different work meant the benchmark
was panicking and callgrind was faithfully counting the panic.

**Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's 85
seconds as the price of having an oracle; 95% of it was three programs answering a question they
had already answered.

**Measure before optimising, and delete what does not measure.** A `FontRef` cache changed a
dense page by less than run-to-run noise and was removed with the reason recorded; the same
session's real win was hoisting a string allocation, 1.37 ms to 18 µs.

**An eager lookup on a cold path is a hot-path cost when the path runs per object.** Reading
`/AcroForm` for every constructed appearance was 2.7× the whole feature's cost.

**Look at what a safe idiom compiles to in a loop that runs per pixel.** `.round()` on a clamped
float is a library call — 205 M instructions on one page, 10.7% of it. The profile said so three
times now and the reading never did.

**The exact fix is often available and is usually better than the approximate one.** A memo keyed
on the input tuple beat the obvious interpolated lookup grid: 3249 M instructions to 1075 M, and
simpler.

**A change made for correctness that is also an order of magnitude faster means the old code was
doing work that was worse than useless.** One mesh raster replaced 4096 flat pieces: 35.47 G
instructions to 3.08 G, and closer to the references.

**And when the first design of a fix is the obviously safe one, still measure it.** Refusing to
cache timeouts is unarguable in principle and left two pages accounting for 46 of 57 seconds.

### Code, bounds and dependencies

**A gap inside a feature you have implemented does not announce itself.** Every missing
*subsystem* reports, because whoever decided not to build it wrote the report. What ships is the
gap *inside* something implemented — `Tr` with four modes silently absent, `/Decode` read on four
routes out of five. **A fast path inherits none of the rules of the path it skips.**

**A "nothing here" is data, and dropping it is not the same as recording it.** §7.5's free
entries and §7.5.8.3's undefined entry types both say an object number names nothing; both were
*skipped* rather than recorded, so the question fell through to an older cross-reference section
and the reader resurrected objects their own file had deleted. Neither could be seen on a page,
because the next thing to run produced a plausible answer. Ask what a `continue`, a dropped
branch or an unmatched arm hands the question *to*. ADR 0100.

**The archetype is the `d` operator.** Every layer of dashing existed and one line read only the
*empty* array, so not one dashed line in 974 documents. When a feature looks finished, check the
operand path from the content stream to the state.

**A feature switched off in one place is switched off everywhere it is not switched on.** The
comment saying a tiling pattern "is not a paint at all" was correct and attached to the one call
site that knew what to do instead; the other call site did not exist when it was written.

**A display list that holds the right commands can still draw nothing, and no report will say
so.** A type 5 mesh was complete, correct and 180 points from where it belonged. Between "we
could not build it" and "we drew it" there is a third state that only the oracle catches.

**A representation can forbid a correct answer.** No amount of care inside an evenly spaced array
of colours can express a discontinuity. Ask what a data structure *cannot say* before asking what
the code does wrong with it.

**A parser that recognises a delimiter without parsing it will be read as parsing it.** The
comment saying only the brackets of an inline dictionary were recognised was accurate, present,
and read by three readers — including this file — as meaning the dictionary was available.

**A clause whose operators are implemented can still be unread.** `J`, `j` and `M` set the line
parameters from the first commit; Table 57's `/LC`, `/LJ` and `/ML` — the same three parameters
by §8.4.1's other route — read nothing for twenty-three sessions.

**An operator that is matched and ignored may still be a rule.** `BX` and `EX` sat with `MP` and
`DP` for thirty-one sessions; §7.8.2 makes them the one place an unrecognised operator is not an
error.

**A convention that agrees with the specification is worse than one that does not**, because it
removes the reason to write the rule down. `tiny-skia` draws a zero-width stroke as one device
pixel, which is exactly §8.4.3.2 — so the clause was never stated and every `0 w` line was
invisible on the GPU for fifteen sessions.

**Where a clause states arithmetic exactly, two independent implementations are worth more than
one shared one.** Trap 2 sends a device *decision* to the shared crate; §11.3.5.3's formulas are
the other kind, and hoisting them would have made the cross-backend scene compare one
implementation with itself.

**Two rasterisers disagreeing is information, not noise — and two agreeing is not proof.** Both
backends positioned paints in the wrong space, in the same way, because the two libraries share
the convention that was misread.

**An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
backend demultiplied Vello's output for fifteen sessions; every scene rendered onto an opaque
background, where the conversion is the identity.

**Two copies of a constant is one defect waiting.** Three `DeviceCMYK` conversions disagreed;
fixing that left the same shape one level down in a nine-constant matrix. It is now one function
with a test that recomputes all nine numbers from the published matrices.

**A constant that is a property of the state must reach every paint, including the ones that
replace the colour.** A shading replaces the current colour, and the line that returned it
dropped `ca` with the colour it did not use.

**A clamp is a decision.** `width.max(0.0)` reads as defensive hygiene and was this program's
whole answer to a value §8.4.3.2 forbids. Ask what a `max`, a `clamp` or an `unwrap_or` *decides*
before calling it a guard.

**A fallback that fills the page is worse than one that leaves it blank.** "If nothing else
matched, the code is the glyph index" drew `v 0' ' W` for `What's an interval?` — confident,
plausible, wrong and silent. **The distinction that makes one legitimate is where the answer
comes from, and it is measurable.** §9.10.2's own permission to "choose a character code of
their choosing" is taken by asking the *program* what it drew — its `post` name, its Unicode
`cmap` inverted — and the corpus readback rose 96.5% → 97.8% with **no document moving the other
way**. A fallback that invents text lowers a score somewhere.

**A shortcut that is right on the common case is worse than one that is wrong on all of them.**
The Cal-space pass-through was nearly correct for `/Gamma 2.2` and badly wrong otherwise, and
nothing distinguishes the two populations at runtime.

**Silent caps are defects, not safety.** Dropping operands past the 64th truncated any `TJ` array
holding a justified line, with `unsupported: []` beside it. Every bound now reports.

**A bound written for the pathological case can refuse a reasonable one.** `MAX_MASK_GRID` exists
because a 2×2 image with a 34862×4332 mask asks for 604 MB; the bound belongs on the *growth*.

**A panic in a dependency is a symptom, not a diagnosis** — especially where its arithmetic is
modular. `tiny-skia`'s overflow panic still fires from a blend mode that is correct to the
channel, because its `u16x16` lanes are meant to wrap. **Being right for the wrong reason is
worse than being wrong.**

**A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
`DCTDecode`, `skrifa` font parsing, `flate2` Flate, `tiny-skia` rasterisation; writing 19 400
lines of MQ coding here would have been consistent with none of it. ADR 0014.

**A dependency can implement more of a specification than the clause cites.** §9.6.5.4 cites the
Adobe Glyph List — two *lists* — and `read_fonts::ps::agl` gives you the list *and* the Adobe
Glyph List Specification's algorithm under one function, which is how `o.sc` answered `o`.

**Look in `read-fonts` before writing font-format code.** An earlier handover specified ~80 lines
of CFF charset parsing plus two 256-entry tables, all of which already existed in
`read_fonts::ps`, re-exported as `skrifa::raw`. ADR 0006.

**The interesting half of a "viewer feature" is usually a clause.** Of the click that follows a
link, the mouse is four lines and the rest is Table 176's three conditions, §12.5.2's coordinate
space and §7.7.3.3's rotation.

## Things worth knowing

- **The oracle's artefacts are the fastest diagnostic in the tree.** Every page that is not
  agreement leaves `<target>/tmp/oracle/<stem>/p<n>/` holding our render, each reference's, a
  side-by-side strip and a difference heatmap per reference. Open the side-by-side first: it is one
  image, four panels, ours leftmost, and it has explained every page it was pointed at. Pages that
  agree have theirs deleted, so what is on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text or vector
  tolerance from our own render's content, so a change that adds glyphs to a page also loosens its
  bound — and can move it from "ambiguous" to "judged". When a page appears in the
  newly-contradicted list, check whether its bound changed before concluding the render got worse.
  Since the thirty-first session the question it asks is `Interpretation::glyphs`, "did glyphs mark
  the page", rather than "did we read any text back" — which had made a page of unnameable CJK a
  vector page and a page of invisible OCR text a text page, both backwards.
- **The sandbox is a flag, and the default is the safe one.** `--no-sandbox` decodes JBIG2 and
  JPEG 2000 in the viewer's process. It can be a flag only because both decoders are memory-safe
  either way: what it trades is panic containment and a memory ceiling. There is deliberately no
  path that falls back to in-process decoding when the worker fails to start.
- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the only
  channel a font has, so a font that maps *some* of its document's codes draws those and says
  nothing about the rest. The eighth session narrowed this — a substitute reaching *none* of the
  declared codes is refused — but the general case needs a report where a glyph is *shown*, in
  `show_text`, which needs `LoadedFont` to distinguish "this code has no glyph" from "this code's
  glyph is blank", which a space legitimately is. Not hard; not done; worth measuring on the
  corpus before assuming the volume is manageable.
- **`Interpretation::text` is a readback of what was drawn**, accumulated by the same loop that
  places the glyphs, and `crates/pdf-model/tests/text_extraction.rs` compares it against
  `pdftotext` — over the 14 specification PDFs, held to 0.99, **and since the sixty-third session
  over all 974 pdf.js documents in 30 seconds**, held to 0.90 with 44 named (ADR 0066). It is
  the only check that catches a code reaching a *plausible* wrong glyph, and it is known to
  bite: reverting the operand-cap fix scores 93.2%, shifting every `/ToUnicode` entry by one
  code scores 58.7%, and the pdf.js run found a real defect on its first pass. The corpus
  number is **97.8%**, and about 31 of the 44 below the floor are §9.10.2's own "there is no way to
  determine what the character code represents" — the *first* half of that sentence, since the
  second half is now taken (ADR 0067).
- **`doc/md/` is the specification, in a form code can read.** Markdown conversions of the 14
  specification PDFs, with real tables, committed — so a test may depend on it without a skip path.
  `ISO_32000-2_sponsored_EC3.md` is 24 MB and its 860 `##` headings give a clause number, a title
  and a line range apiece, which is the whole basis of the citation checker and the ledger. Two
  caveats: it is a *conversion*, so a quotation the checker cannot find may be an artefact — check
  `doc/`'s PDF before editing the comment — and one heading number (`14.8.4.7.3`) occurs twice.
  **The conversion also drops content, which the seventieth session met for the first time**: Table
  164's `/Di` row ends "Default value: 0." in the standard's own PDF and the markdown has no such
  line, so a reading taken from `doc/md/` alone would have recorded a silence the standard does not
  have. `pdftotext -layout` over `doc/ISO_32000-2_sponsored_EC3.pdf` is the check, and a table is
  where to expect this — the conversion reflows them. **The hundredth session met the third
  instance and it was the citation checker complaining**: `Table 246 -Entries in the FDF
  dictionary` is a markdown `##` heading where every other caption in the same subclause is a
  bare line, so `table_title` found nothing and reported a table the standard *has* as one it
  does not. When a gate accuses the standard of a gap, suspect the conversion first.
  When you need spec data, extract it from there rather than writing it from memory: the
  `WinAnsiEncoding` and `MacRomanEncoding` tables came out of Table D.2 that way, and the
  extraction caught three things memory would have got wrong. The files carry base64 images
  inline, so `grep -v '^!\[Image\]'` before reading a range.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's
  application note on black point compensation, written by ISO 32000's own co-project-leader, and
  it settled a design question the base specification leaves to ISO 18619. It had been sitting
  unread while the same question was being answered by looking at other renderers. The sharper
  form of the same lesson: image reduction was recorded as unspecified in two places, and §10.7.4
  specifies it four clauses from one the tree cites constantly. `grep -n '^## '` over the
  conversion and read the *titles* around your subject; it takes a minute.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding` must be
  one of three names; it does not say what those encodings contain. Do not expect glyph data,
  operator semantics or rendering rules from it.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `crates/render-cpu/src/lib.rs`, and ADR 0010 for why rows rather than a rectangle. Two
  consequences: the device transform handed to a command already carries the band's row offset, so
  anything new that composes a transform must use *that* one; and the clip mask is band-tall and
  page-wide, because `tiny-skia` needs it to share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello wants a layer
  stack; both translate. That neither library's model is native is the evidence the neutral form is
  right, and it is what lets the CPU backend validate the GPU one on byte-identical input.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter. A test
  pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second kind of metric.** The reference
  renderers disagree with each other at worst-tile 26–28 on text pages — glyph hinting, not error
  — and no threshold fixes that, because the noise floor is above the signal.
  `raster_compare::Comparison::structural_similarity` measures whether the same shapes are in the
  same places instead, and `Tolerance` bounds it: 0.99 for vector, 0.90 for text. Both were
  measured over 153 reference-against-reference pairs, and the doc comment records that the
  distribution is *continuous* — 0.8990, 0.8993, 0.8998 and 0.9009 all occur — so 0.90 is a choice
  about which population to exclude, not a discovered boundary.
- **Reference renderers are given 30 seconds and then killed.** A corpus holds files written to
  make a reader loop, and `Command::output` waits forever. `Reference::render_within` polls and
  kills; there is deliberately no unbounded variant.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That pairing is
  what let the harness work before a parser existed, and a test renders both and demands identical
  pixels.
- **Debug builds are ~15× slower here, and it changes what a test can assert.** The corpus gate is
  2 s in release and minutes in debug. Any test with a timing assertion is meaningless at debug
  speed; run those in release and say so. The oracle gate is the exception that proves it: about
  95% of its processor time was three external renderers, whose speed does not depend on how we
  were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather than
  finding out from a red pipeline.
