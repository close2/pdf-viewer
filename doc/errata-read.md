# Errata Collection 3, read against this tree

What `tools/spec-errata check` named, and what each passage turned out to be. Begun in the
four-hundred-and-seventeenth session so that nobody reads them twice and finished in the
four-hundred-and-eighteenth. ADR 0252 built the tool; ADR 0253 and ADR 0254 are the two readings.

**All 120 distinct passages `check` names now carry a verdict**, and the four-hundred-and-twenty-ninth session re-ran it eleven rounds later and got **the same 151 lines for the same 120 passages**: nothing remains unread and the number is not moving. The three tables below are the
order they were read in rather than three kinds of thing: **the 79 lines / 65 distinct passages**
`check` printed before its comparison was corrected; **three more** that the correction made visible
and that `Landing::in_clause` had filed under a neighbouring clause; and **the other 54**, read a
round later. `check` prints 151 lines for those 120, because an erratum stated in two annotation
objects prints twice.

**The instrument was corrected once, between the first table and the third.** Its comparison was
whitespace-sensitive and both sides are extractions of the same glyphs by different programs, so a
passage one writes `inthe` and the other writes `in the` was called absent; dropping the spaces took
the count from 79 lines to 151 (ADR 0253).

## What the `/State` annotations turned out to be

Session 416 filtered on `Completed` and said so; this session asked what the alternative was worth.
Over all fourteen documents, **every note carries a Review-model state and not one of them is
`Rejected`, `Cancelled` or `None`**:

| Review state | all notes | strikeouts | strikeouts of ≥4 words |
|---|---|---|---|
| `Completed` — "[t]he change has been completed" | 827 | 331 | 142 |
| `Accepted` — "[t]he user agrees with the change" | 265 | 108 | 45 |
| `Rejected`, `Cancelled`, `None` | 0 | 0 | 0 |

So the mirror-image mistake the round was warned about — quoting a sentence a *rejected* erratum
struck — cannot occur in these files, and `Completed` was a narrower filter than the evidence
rather than a wrong one. `Accepted` and `Completed` are both binding here and are treated alike
below; the distinction is where a change sits in the PDF Association's workflow, not whether
anybody disagreed with it.

Five notes reported as "Accepted, Unmarked" and the second word is **not** a second opinion:
Table 174 states two state models and `Unmarked` is the *Marked* model's default. `spec-errata`
now prints `StateModel/State` so that the two cannot be confused again.

## The first 79 lines, by clause

Verdicts: **implements** — the tree carries out the retired rule; **quotes** — a comment or ledger
note quotes retired text; **cites** — the clause is cited and the erratum changes nothing it
requires; **untouched** — nothing here reads it. `[416]` marks the three session 416 read.

`×N` counts the lines `check` printed, not distinct passages: an erratum stated in two annotation
objects prints twice. The multiplicities below sum to the 79.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §3 Terms | 25 | #181 | untouched | ×2. The ICC.1 term becomes ISO 15076; no colour code consults a specification name. |
| §7.3.4.2 Literal strings | 41 | #494 | quotes | `\ddd` becomes "[b]yte with value ddd in octal". `lexer.rs` masks to the low byte and its comment credited *other implementations* for a rule §7.3.4.2 prints — corrected. |
| §7.3.10 Indirect objects | 48 | #379 | cites | ×2. A new lexical grammar for object and generation numbers: no leading `+`, no leading zeros. The lexer accepts both; that is a reader's tolerance and is now an *undocumented* one. Owed below. |
| §7.5.4 Cross-reference table | 70 | #149 | cites | "The cross-reference table is" → "Cross-reference sections are". Editorial. |
| §7.5.5 File trailer | 73 | #101 | quotes `[416]` | `write.rs:506`. `startxref` may name a cross-reference stream; `xref::read_at` already read both. |
| §7.5.5 File trailer | 74 | #106 | cites | "; shall be an indirect reference" struck from a trailer row. Nothing here requires indirection. |
| §7.5.6 Incremental updates | 75 | #399 | cites | `/Version` upgrades rather than overrides, and "shall not reduce". `version::document` is already `max(header, catalog)`. The *multi-update* reduction case is new and unhandled — owed below. |
| §7.5.7 Object streams | 79 | #638 | untouched | Repairs a mangled EXAMPLE gloss; the normative source is Table 18, which `xref.rs` reads. |
| §7.6.3.1 General | 89 | #542 | quotes | PKCS#7's whole-extra-block pad moves from an EXAMPLE into the normative sentence. `crypt.rs` has always added and stripped it; `AES_BLOCK`'s comment quoted the retired half — corrected. |
| §7.6.4.4.1, §7.6.5.1 | 97, 101 | #24 | quotes | ×3. "with an initialization vector of zero" struck from all three **ECB** occurrences, where it was nonsense. `perms_block`'s comment said so on its own line before the erratum could be read; the quotation marks are gone. The **CBC** occurrences keep the phrase and `crypt.rs:1075` still quotes them correctly. |
| §7.6.7 Unencrypted wrapper | 110 | #529 | untouched | "the EmbeddedFiles name tree shall contain exactly one entry" deleted. Nothing enforced it. |
| §7.7.2 Catalog | 114, 115 | #106 | cites | ×3. Indirect-reference requirements relaxed; `Document::get_key` resolves either shape. |
| §7.7.3.3 Page objects | 121 | #106 | cites | "; indirect reference preferred" struck. |
| §7.9.4 Dates | 133 | #251 | quotes `[416]` | Two sentences retired; `Date::instant` was right for the surviving reason. |
| §7.12.3 Developer extensions | 157 | #732 | quotes | `/URL` becomes **Required**. The ledger called it optional — corrected. `DeveloperExtension::read` still answers an extension without one, now as a stated choice. |
| §7.12.6 URL | 158 | #399 | cites | The `/BaseVersion` version comparison, which this reader does not perform. |
| §8.6.5.5 ICCBased | 207 | #181 | cites | ×3. The PDF-version→ICC-specification table becomes an ICC *profile header* version table, "both 2.x and 4.x". Nothing here ties a profile to a PDF version; `icc.rs` reads the header byte and accepts both. |
| §8.6.5.8 Rendering intents | 212 | #63 | cites | The NOTE's licence to support fewer than four intents is withdrawn. Nothing here leant on it; the existing `partial` gap is unchanged and slightly less excusable. |
| §8.6.8 Colour operators | 232 | #551 | cites | Table 74's `CS` row no longer says the resource entry "shall be an array"; `colour::parse_at` already accepted a name first. |
| §8.9.5.1 General | 274 | #366 | untouched | "if a predictor function is used" struck from Table 87's `/BitsPerComponent`. Predictors never reach the image dictionary here. |
| §8.9.5.4 Alternate images | 279 | #79 | **implements** | The algorithm is rewritten and three of its steps contradict `alternate_image`. See the finding below. **Amended algorithm implemented in the five-hundred-and-fortieth, ADR 0375.** |
| §8.9.7 Inline images | 283 | #20 | untouched | "one of its filters" → "its final or only filter". `inline_image` skips one white-space byte unconditionally and never applies the test in either form. |
| §8.10.2 Form dictionaries | 287 | #292 | cites | `/Resources` becomes "Sometimes required" and independence becomes required from PDF 2.0. The page-resource fallback this tree has rests on §7.8.3, which the erratum leaves standing. |
| §8.11.3.2 Optional content | 297 | #707, #686 | untouched | ×2. EXAMPLE code wrapped in object syntax. Nothing normative. |
| §8.11.4.3 Configuration | 301 | #225 | quotes | ×2. `/RBGroups` gains "None of the inner array elements shall be an empty array." — a producer's rule. One comment quoted the struck ", each of which" — corrected. |
| §8.11.4.4 Usage | 303 | #567 | cites | A cross-reference struck, a NOTE added saying the `Language` usage entry changes no content's language. The tree does not read it and never could have. |
| §9.4.4 Text space details | 325 | #550 | cites | ×3. "All literal string rules apply before the string" is interpreted. Escape processing here finishes in the lexer, before any font sees a byte — the amended rule is the tree's own construction. |
| §9.6.4 Type 3 fonts | 333 | #106 | cites | Indirect-reference relaxation. |
| §9.6.4 Type 3 fonts | 335, 336 | #44 | untouched | ×2. An EXAMPLE's glyph names; the tree's own fixture already keys on `/Encoding` positions 97 and 98. |
| §9.7.3 CIDSystemInfo | 344 | #518 | cites | An *array* of `/CIDSystemInfo` dictionaries is no longer permitted in a CMap. The tree never accepted one. |
| §9.7.4.2 Glyph selection | 346 | #106 | cites | Indirect-reference relaxation. |
| §9.8.2 Font descriptor flags | 360 | #453 | cites | `/MissingWidth`'s "predictable effect only if" becomes "to ensure predictable results … otherwise implementation dependent". No obligation on a reader either way. |
| §9.10.3 ToUnicode | 373 | #87 | untouched | A Unicode character name corrected in an EXAMPLE. |
| §10.6.5.4, §10.6.5.6 | 391, 395 | #310, #12 | untouched | ×2. Halftone dictionaries. §10.6 is inapplicable on the standard's own condition (`CLAUDE.md`). |
| §12.5.6.9 Polygon/polyline | 506 | #444 | cites | `/Vertices` "shall be ignored if Path is present". `appearance::path` tries `/Path` first and structurally never reads `/Vertices` after it. |
| Table 234 `/I` (p.558) | 558 | #468 | **quotes** | `/I` is no longer restricted to `MultiSelect` fields and the "value is an array" trigger is deleted. The code was already right; **the erratum vindicates it** — writing `/I` beside a single selection was a stretch of the retired wording and is the plain sense of the amended one. Two quotations corrected. |
| §12.7.5.5 Signature fields | 561 | #680 | untouched | The seed value dictionary's `/Ver`. `/SV` is unread; the ledger says why. |
| §12.7.8.3.1 General | 576 | #173 | cites | "Although FDF file encryption is deprecated" — the disambiguation `forms_data.rs` already made in prose. |
| §12.8.1 General | 582 | #685 | cites | Table 254's `/Page` loses "for annotations in FDF files", which restated the table's own title. |
| §13.2.4.2, §13.2.7.2.2, §13.5, §13.6.4.1 | 638–672 | #414, #449, #481, #150 | untouched | ×4. Clause 13 is out of scope by `CLAUDE.md`'s closed list. |
| §14.6.1 Marked content | 736 | #335 | cites | **"They may not occur within a graphics object" is struck**, and marked content may appear "within a text object". The interpreter's marked-content stack was never coupled to `BT`/`ET` — it was written from the *surviving* nesting paragraph, which already gives `BT BMC … EMC ET` as valid, and existing tests across `accessibility.rs`, `logical_order.rs` and `logical_structure_example.rs` use the now-explicitly-legal form. One stale comment about Figure 9 in `variable_text.rs` is owed below. |
| §14.7.5.2, §14.7.5.4 | 746, 749 | #431 | quotes | ×2. `/Pg` becomes "Sometimes required" and "overrides" becomes "takes precedence over". `destination.rs` quoted the struck sentence — corrected. `doc/adr/0054` quotes it too and stays, being a record of a decision made at the time. |
| §14.7.5.4 | 750 | #463 | untouched | "these two entries" becomes "StructParent or StructParents". Naming only. |
| §14.7.6.1 General | 752 | #354 | quotes `[416]` | "conforming product that owns" → "owner of". |
| §14.7.6.2 Attribute classes | 753 | #226 | cites | A cross-reference repointed at Table 376, which `structure.rs` already names. |
| §14.8.4.7.2 Annot/Form | 779 | #437 | quotes | ×2. Both types are redefined from an *association* to an *enclosure*. Two doc lines and one ledger note carry the retired framing as paraphrase — owed below; no code change is implied. |
| §14.8.4.8.4 Caption | 783 | #200 | cites | ×2. The implicit table-header algorithm is rewritten, including a new "conflicting `Scope` does not terminate the search" rule. `/Scope` and `/Headers` are unread; when they are implemented it must be from the amended text. |
| §14.8.4.8.6 Formula | 784 | #470 | untouched | ×2. "**Formula** shall not appear between the BT and ET operators" struck outright. Never enforced here. Note that `Figure`'s identical sentence is **not** struck. |
| §14.11.5 Output intents | 839 | #482 | cites | `/DestOutputProfileRef`'s `/URLs` may no longer be an embedded file specification. Neither entry is read; only `/DestOutputProfile`. |
| §14.12.4.1 DParts | 851 | #612 | cites | "The array shall not be empty" → "Empty arrays shall not be present". An empty `/DParts` already answers `None`. |
| §14.13.5 Associated files | 854 | #374 | **implements** | The property list's key is `/MCAF`. See the finding below. |
| §14.13.8 Associated files | 855 | #374 | cites | ×2. A DPart's `/AF` stays an array of file specifications, which is what `associated` requires. |
| Annex E.2 | 893 | #229 | untouched | The second-class-name exception becomes "where otherwise stated in this specification". A reader treats an unknown key alike whichever class it is in; the row's `writer-side` reasoning survives. |
| Annex H.3 | 922, 923 | #402, #563 | untouched | ×3 lines, two errata. Annex H says *informative* on its own title line; both errata are inside examples. |
| ISO/TS 32001 §5.1.4 | 10 | #404 | untouched | SHAKE256's fixed OID gives way to RFC 8702's algorithm identifiers. No SHA-3 or SHAKE anywhere in the tree; an unknown digest OID is reported rather than guessed. |

## Not on the first 79: three the corrected comparison found, and all three are findings

Each is a passage whose two extractions space a word differently, so the whitespace-sensitive
containment test called it absent. Each also lands on a page whose *outline* section is the next
clause along, so `Landing::in_clause` filed all three as coincidences — the bucket is a sort order
and not a verdict.

| clause | p. | outline says | issue | verdict | what it turned out to be |
|---|---|---|---|---|---|
| §12.5.2 Annotation dictionaries | 485 | §12.5.3 | #23, #34, #56 | **implements** | `BM` struck out of the list of entries a reader ignores, `MK` inserted, and the blanket "without regard to any other keys" removed. `/BM` was being ignored on every stored appearance stream. **Fixed.** |
| §9.6.2.2 Standard 14 fonts | 330 | §9.6.2.3 | #47, #48 | **quotes** | The clause's `shall` struck and its neighbour demoted to an informative NOTE. Three doc comments called it this program's warrant for the compiled-in fourteen. **Fixed.** |
| §14.8.6.1 Namespaces | 809 | §14.8.6.2 | #151 | quotes | The default-namespace sentence is replaced by one that states the order — the default applies *after* the role map has been applied transitively. `Tree::role` is that walk already. **Quotation annotated.** |

## The findings of the four-hundred-and-seventeenth session, and what was done

1. **§12.5.2, Issue #23 and #34 with #56 — `/BM` was being ignored on a stored appearance stream.**
   Not on `check`'s list at all: the strikeout's text joins two words and the whitespace-sensitive
   comparison missed it. Both `appearance.rs`'s module doc and the §12.5.2 ledger row quoted the
   retired sentence as the rule that "shapes `crate::appearance`". EC3 strikes `BM` out of the
   ignore-list and inserts `MK`, leaving §12.5.5 and Table 166's own `/BM` row — which states the
   mode for "painting the annotation onto the page" with no condition — with nothing against them.
   **Fixed**: `annotation::blend_mode` reads `/BM` on both paths. `/CA` and `/ca` stay ignored;
   they are still on the amended list and Table 166 states their condition twice.
2. **§14.13.5, Issue #374 — the marked-content property list's key is `/MCAF`.** The 2020 clause
   named no key, so `AF` was an inference from the tag operand, and a conforming PDF 2.0 file was
   read as stating no associated file, silently. **Fixed**:
   `attachment::associated_in_property_list` reads `/MCAF` first and `/AF` after it, with a test.
   Table 409a is in neither `doc/md/` nor the annotations, so whether `/AF` should now be *refused*
   there cannot be decided from this project's copy and is not decided.
3. **§9.6.2.2, Issue #47 and #48 — the standard-14 `shall` is gone.** The sentence three doc
   comments quoted is struck outright and the paragraph carrying the same requirement is demoted
   to an informative NOTE with its `shall` softened; §9.6.2.1's compatibility `shall` goes the same
   way. **Fixed**: the three quotations are replaced by the warrant that survives — Table 109's
   "optional in PDF 1.0-1.7 for the standard 14 fonts", which makes the metrics necessary to draw
   the page at all, and §6.3.2.2.
4. **§8.9.5.4, Issue #79 — the alternate-image algorithm is rewritten, and this tree implemented
   the retired one until the five-hundred-and-fortieth session.** Three divergences, quoted in `content/image.rs::alternate_image` and in the
   ledger row. (This said `content.rs::alternate_image` until the five-hundred-and-thirty-seventh
   session: the function moved into `content/` when the module was split, and the pointer's
   *symbol* half is what found it.)
   **Not fixed here, and the reason given was wrong.** It read: the amended step a) ends "then
   nothing shall be shown", which is terminal and would leave the amended d) unreachable for a
   hidden base, so a rewrite would trade one contradiction for another. a) *is* terminal and d) is
   unreachable for a hidden base — and that is what the erratum changes, not a defect in it: a) and
   b) dispose of every base image stating an `/OC`, and c) and d) open at "Otherwise", so they
   belong to a base image stating none. The five amended steps are total, disjoint and reachable.
   **Fixed in the five-hundred-and-fortieth** (ADR 0375); no corpus document states `/Alternates`
   and the corpus gate prints the same figures either way.
5. **The instrument under-reported by 72.** See ADR 0253.

## The other 54, read in the four-hundred-and-eighteenth session

`check` names **151 lines, 120 distinct passages**. Sixty-three of the 120 carry a verdict in the
table above; three more of them are in the second table (§12.5.2 at p.485, §9.6.2.2 at p.330,
§14.8.6.1 at p.809), which the page-straddle put under a neighbouring clause. **The remaining 54
are here**, same discipline, one line apiece. ADR 0254 is the round.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §2 Normative references | 21 | #719 | cites | The MathML 3.0 reference becomes **MathML Core**. `structure.rs`'s `MATHML_NAMESPACE` and §14.8.6.3's ledger row both said "MathML 3.0" — the URI does not move and the edition is now the wrong thing to name. Corrected. |
| §3 Terms | 23 | #149 | untouched | "cross-reference table" is redefined as the data derived from *all* sections and streams, with a NOTE saying the colloquial use is wrong. Nothing here names the term normatively. |
| §7.2.3 Character set | 36 | #193 | untouched | A NOTE about representing a non-encrypted file in printable ASCII. `lexer.rs`'s byte classes are the clause's other half and are untouched. |
| §7.5.4 Cross-reference table | 71 | #113 | **false positive** | "Each cross-reference subsection shall contain entries for a contiguous range of object numbers." — and `doc/md/` carries that sentence **twice on one line**, because the standard prints it twice. The erratum deletes one copy; the rule stands, and `xref.rs`'s `realigned` still rests on it. See the note on the instrument below. |
| §7.5.4 | 71 | #147 | untouched | NOTE 3's closing sentence, that an update's subsections can never have object number zero, struck; "positive" becomes "non-negative". A NOTE, and `xref.rs` records a free entry rather than reasoning about the number. |
| §7.5.5 File trailer | 73 | #149 | cites | `/Size`'s definition is inverted: the "1 greater than the highest object number" sentence becomes the rule and the entry count becomes NOTE 1. §7.5.5's row already treats `/Size` as a claim to be departed from, on 68 corpus documents. |
| §7.5.6 Incremental updates | 75 | #341 | untouched | An advantage-of-updates NOTE loses its OLE and HTTP examples. |
| §7.5.8.4 Compatibility | 83 | #146 | cites | `/XRefStm`'s offset loses "in the decoded stream" exactly as §7.5.5's `startxref` did (#101, read in the 417th). `xref::read_at` reads the offset from the file's start either way, which is what the amended sentence says. |
| §7.6.4.3.3 Algorithm 2.A | 96 | #53 | untouched | A NOTE moved and repointed at §7.6.4.4. |
| §7.6.4.4.1 Algorithm 2.B | 97 | #325 | cites | **The erratum vindicates the code.** Step a)'s "64 repetitions of the sequence: input password, K, the 48-byte user key" is replaced by an explicit K0 — password ‖ K ‖ user key for the owner, password ‖ K otherwise — then K1 = 64 × K0. `crypt.rs::hash_2b` builds exactly that, with `extra` empty on the user path. |
| §7.8.3 Resource dictionaries | 127 | #9 | cites | "(or is an element of an array that is the value of that entry)" struck from the page-`/Contents` bullet. A `/Contents` array is one stream by §7.8.2 and is read as one here. |
| §7.8.3 | 127 | #128 | **implements** | The Type 3 glyph bullet becomes a **four-step search order** whose first step this tree did not have. See the findings below. |
| §7.9.2.1 General | 130 | #322 | cites | Table 39's type names are rewritten — "string" gains "may be further qualified", `/Number` becomes "numeric object", the tree types are named as data structures. §7.9.2.1's own body sentence, which the ledger row quotes, is **not** struck. |
| §7.9.2.2.1 | 131 | #96 | untouched | An EXAMPLE's Cyrillic bytes, spelled as `\213` and `<FEFF…>` instead of as question marks the conversion produced anyway. |
| §7.9.2.4 Byte string | 132 | #96 | untouched | NOTE 5 on UTF-16BE against UCS-2 deleted. Nothing here has a `wchar_t`. |
| §7.11.4.1 General | 153 | #155 | quotes | `/Subtype`'s MIME type is narrowed to RFC 2046 §2's top-level type and description, with no parameters and no `;`, `=` or `#`. Every word of it binds a producer. `attachment.rs` quoted the retired sentence — corrected, and the value is still kept as the document wrote it. |
| §7.12.6 URL | 158 | #239 | cites | `/ExtensionRevision`'s "shall be a monotonically increasing sequence" becomes "should increase". Nothing here compares two revisions. |
| §8.4.5 ExtGState | 180 | #371 | cites | Table 58's `/FL` loses the 0-to-100 range, which moves into §10.7.2 (below). The permission this tree exercises is in neither half. |
| §8.4.5 | 182 | #360 | cites | `/UseBlackPtComp` loses "The default value is: Default." — an entry whose stated default already said "up to the PDF processor". `BlackPoint::Default` compensating is this processor's determination either way, and `content.rs` says so. |
| §8.6.5.5 ICCBased | 206 | #181 | cites | The other half of the erratum read at p.207: the table becomes one of ICC *profile header* versions. |
| §8.6.6.5 DeviceN | 221 | #309 | untouched | The `/Colorants` restriction to non-`NChannel` spaces is struck. `/Attributes` is unread; nothing in the tree names `NChannel`. |
| §8.9.6.1 | 280 | #79 | **implements** | The second mark of §8.9.5.4's rewrite, carrying the amended step c) and the new step e). Finding 4 above, still declined for the reason stated there. |
| §8.9.6.3 Explicit masking | 281 | #333 | untouched | A cross-reference repointed from §9.6.5.3 to §9.6.4, and "need almost always be used" softened to "is normally used", in a NOTE about stencil masks and glyph bitmaps. |
| Table 74 `CS`/`cs` | 285 | #19 | cites | **The erratum vindicates the code.** "The names DeviceGray, DeviceRGB, DeviceCMYK and Pattern always identify the corresponding colour spaces directly" becomes "either directly **or via a default colour space** (see 8.6.5.6)" — which `colour.rs` has done since the twenty-fifth session, remapping through `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK`. |
| §9.6.2.1 General | 329 | #106 | cites | "; shall be an indirect reference" again. |
| §9.6.4 Type 3 fonts | 333 | #128 | **implements** | Table 110's `/Resources` row is rewritten and loses the page fallback it stated. Same finding as §7.8.3's. |
| §9.6.4 | 334 | #128 | **implements**, quotes | Step d) is replaced by a pointer to §7.8.3. `type3.rs::resources` quoted the retired two-place rule and implemented it. **Fixed.** |
| §9.8.1 General | 358 | #11 | cites | Table 120's `/FontName` gains a Type 3 case: it matches the font dictionary's `/Name` for a Type 3 font and `/BaseFont` for every other. A writer's rule; `collection.rs` matches `/FontName` against a TrueType collection's PostScript names, which the surviving half describes. |
| §10.6.5.6 Type 5 halftones | 396 | #311 | untouched | §10.6 is inapplicable on the standard's own condition (`CLAUDE.md`). |
| §10.7.2 Flatness | 397 | #371 | cites | "It shall be a positive number" gains the 0-to-100 range and the meaning of 0, moved here from Table 58. The permission this row rests on — "PDF processors may choose to ignore any flatness tolerance" — is untouched, and `i` is still matched and discarded. |
| §12.5.2 Annotation dictionaries | 483 | #287 | quotes | "[i]f an annotation dictionary includes the BS entry, then the Border entry **is** ignored" becomes "**shall be** ignored". `appearance.rs` quotes it in two places; the precedence it implements is the same one. Annotated. |
| §12.5.6.21 Screen | 513 | #42 | cites | "If AP is not present, the screen annotation shall not have a default visual appearance and shall not be printed" struck. §12.5.6.18's row already refuses an appearance-less screen annotation *and reports it*, which is a stated choice rather than a rule this clause supplied. |
| §12.5.6.24 Projection | 520 | #42 | untouched | The rule forbidding an `/AP` on a zero-area projection annotation struck. Nothing enforced it. |
| §12.7.4.1 General | 546 | #313 | cites | **The erratum vindicates the code.** "a field dictionary may also be an annotation dictionary" becomes "a **Widget** annotation dictionary (see 12.5.6.19)", which is the only merge `appearance.rs` performs. |
| §12.7.4.3 Variable text | 549 | #393 | untouched | An EXAMPLE cross-reference repointed at the example above it. |
| §12.7.5.5 Signature fields | 561 | #158 | cites | `/DigestMethod`'s DSA-and-SHA-1 sentence becomes "[s]ome signature mechanisms require a specific digest function … the value of this entry shall be ignored". `/SV` is unread; the ledger says why. |
| §13.2.7.2.2, §13.5, §13.6.4.4, §13.6.4.7, §13.7.2.2.4 | 648–718 | #449, #481, #38, #156, #145 | untouched | ×5. Clause 13 is out of scope by `CLAUDE.md`'s closed list. |
| §14.5 Page-piece | 734 | #691 | untouched | "such as MD5 (described in Internet RFC 1321)" struck from a NOTE about detecting a changed page. The row is `inapplicable` and names no digest. |
| §14.6.1 General | 735 | #334 | **quotes** | **NOTE 3 is deleted outright** — the one saying a marked-content tag has "no relationship to Tagged PDF … and thus is not rolemapped" — and the paragraph beside it now expects a tag *not* defined in an ISO publication or in §14.7 to use a second-class name. §14.6's ledger row gave that NOTE as its reason for reading no tag's meaning. **Fixed**: the status stands and the reason is now the gap in §14.7 and §14.8. |
| §14.6.1 | 735 | #303 | untouched | NOTE 1, on a marked-content sequence being complete graphics objects rather than bytes, struck; an EDITOR NOTE says the notes will be renumbered. |
| §14.8.4.4 Grouping | 772 | #141 | untouched | "Part is the semantic equivalent of Div." is replaced by a NOTE saying the opposite — a `Part`'s grouping *has* semantic value where a `Div`'s does not. `structure.rs` carries both types by name and interprets neither. |
| §14.8.4.7.2 | 778 | #84 | untouched | `Strong`'s EXAMPLE 3 gloss: "the content that the user is intended to read first" becomes "is more important". |
| §14.8.4.7.2 | 779 | #133 | cites | §14.8.4.7.3's link element is relaxed — one content item rather than two, "one or more link annotations", and the condition that their `/A`, `/Dest` and `/PA` match is struck. Recorded in the §14.8.4.7.2 row beside #437's enclosure reframing, which this round also settled. Nothing here validates an element's children. |
| §14.8.6.3 Other namespaces | 810 | #72, #719 | quotes | The MathML sentence loses its version, gains a requirement that the `math` element enclose the formula under `Formula`, and requires the namespace on every MathML type *and attribute*. The row and `structure.rs` said "MathML 3.0" — corrected. **The enclosure requirement was read in the five-hundred-and-fortieth and is a `shall` on whoever *includes* the mathematics** — the sentence opens "[w]hen including mathematics structured as MathML" — so it falls under `CLAUDE.md`'s producer exclusion; what stays owed is a validator's report. ADR 0375. |
| §14.12.4.2 DPart metadata | 852 | #290 | cites | Table 409's `/Metadata` is **withdrawn**: "XMP metadata streams shall not be used in DPart dictionaries", with a NOTE that it was allowed in earlier editions of PDF 2.0. `document_part` reads `/Start` and `/DParts` and no metadata at all. |
| §14.13.2 Embedded associated files | 853 | #568 | cites | The designation is rewritten: an object's `/AF` is an array of file specifications carrying `/AFRelationship`, and a marked-content sequence "shall use an AF marked content tag (see 14.13.5)". That is the same division session 417's `/MCAF` finding rests on, from the other side. |
| Annex E.2 | 893 | #340 | cites | Third-class names: the `XX` prefix survives, "[i]t is not necessary to register" becomes "cannot be registered". The row's own sentence is unaffected. |
| Annex F.3.5 | 900 | #389 | untouched | A cross-reference repointed from F.3.4 to F.3.7. |
| Annex H.7.5 | 939 | #402 | untouched | An XMP example replaced by an ellipsis. Annex H says *informative* on its own title line. |

## What the reading found about the instrument itself

**A struck passage `doc/md/` still carries is not always a retired one.** §7.5.4's #113 is the
witness and it is the first false positive this list has produced: the conversion carries "[e]ach
cross-reference subsection shall contain entries for a contiguous range of object numbers" **twice
on one line**, because the standard prints it twice, and the erratum deletes one copy. Nothing in
the annotation says which of the two it covers, so `still_in_conversion` cannot tell a
de-duplication from a retirement and the reader has to. `xref.rs::realigned` rests on that sentence
and keeps it.

**And `check`'s two questions have different populations.** Three of the errata acted on this round
— §7.11.4.1's #481, §12.7.5.2.2's #386, §14.8.2.2.2's #484 — are **not** on the 151, because
`doc/md/` spells those passages differently enough that the containment test misses them, and they
were found by the *other* half of the check: a quotation in this tree that overlaps struck text.
Neither list contains the other.

## The ledger's 977 spans, and a third population nobody had counted

ADR 0249 established that `cargo test -p conformance` verifies rustdoc blockquotes and nothing in
`ledger.toml`, whose notes hold **977 double-quoted spans**. This round swept them — against the
errata rather than against the standard, which needs no new syntax in the ledger because the
erratum supplies the other side of the comparison — and found a **third** population on the way:
a pair of quotation marks inside ordinary rustdoc *prose*, which `CLAUDE.md` binds exactly as hard
as a blockquote and which the gate's blockquote scanner walks straight past.

| population | in-clause landings | elsewhere | stale quotations found |
|---|---|---|---|
| rustdoc blockquotes — the one population with a gate | 8 | 10 | **1** |
| rustdoc prose | 11 | 28 | **6** |
| `ledger.toml` notes | 11 | 10 | **4** |

Eleven stale quotations, and **four of them are in the "elsewhere" bucket** — the one `check` prints
under "a repeated phrase rather than a finding". `Landing::in_clause` compares the clause a
quotation cites against the clause the *outline* puts the erratum's page in, and a clause heading
that straddles a page break puts a real landing in the wrong list; ADR 0253 found three that way and
this round found four more. The bucket is a sort order, not a verdict, for the third round running.

Every in-clause landing that is not a defect is a *correction* quoting the wording it retired, which
is `doc/todo/01`'s known false-positive shape and the same one its first four sweeps produce.

## The findings of the four-hundred-and-eighteenth session

1. **§7.8.3 and §9.6.4, Issue #128 — a Type 3 glyph description's own `/Resources` was never
   read.** The 2020 clause named two places for a glyph description's resources, the Type 3 font
   dictionary and then the page, and `Type3Font::resources` implemented exactly that. EC3 replaces
   §9.6.4's step d) with a pointer to §7.8.3 and gives §7.8.3 a four-step search: "1. the stream
   dictionary of that glyph description content stream; 2. the parent Type 3 font dictionary that
   contained the CharProcs entry", then the page and what the page inherits. **The first step was
   missing**, so a glyph stream stating its own `/Resources` was read against somebody else's
   dictionary — silently, since a resource name that resolves to nothing draws nothing. **Fixed**,
   with `tests/type3.rs::a_glyph_description_finds_the_resources_its_own_stream_names` built so
   that only the new step can answer it. Table 110's `/Resources` row loses its page fallback in
   the same erratum, and that fallback survives as §7.8.3's steps 3 and 4.
2. **§14.6, Issue #334 — the ledger's reason for reading no tag's meaning was a deleted NOTE.**
   Corrected; the status does not move and the reason is now §14.7's and §14.8's unimplemented
   semantics.
3. **Six prose quotations of sentences EC3 struck**, none of which any gate can see:
   `viewer-ui/src/chrome.rs` and `pdf-font/src/lib.rs` on §9.6.2.1's and §9.6.2.2's standard-14
   `shall`s — the two the four-hundred-and-seventeenth session missed while correcting three
   others — `attachment.rs` twice on §7.11.4.1 (#481 and #155), `form.rs` on §12.7.5.2.2's struck
   "it shall not use the V and DV entries" (#386), and `type3.rs` on the rule behind finding 1.
4. **Four quotations in three ledger notes**: §7.6.3 (#542's rewritten AES sentence), §7.8.3
   (#128's fourth bullet *and* §9.6.2.2's struck `shall`, which is two), §14.6 (finding 2).
5. **One blockquote whose sentence is now informative**: `tests/accessibility.rs` on §14.8.2.2.2,
   which #484 splits into a NOTE 2 with its two `shall`s softened to "is". The blockquote stays
   verbatim, because `doc/md/` is what the gate verifies against; what the test rests on is the
   surviving normative half.

Four of session 417's five owed items are also settled — §7.3.10's grammar, §7.5.6's multi-update
version reduction, §14.8.4.7.2's enclosure reframing and §14.6.1's Figure 9 — each as a documented
choice in the row or the comment that owns it, rather than as code.

## The fourth and fifth populations, swept in the four-hundred-and-nineteenth session

Two more things `check` could not see, both found by walking into one of them rather than by
looking. ADR 0255 has the argument; this is the list.

**A quotation inside an ordinary `//` comment.** `prose_quotations` read `///` and `//!` only, and
its own doc comment gave the reason: "a `\"` in a `//` comment is not making `CLAUDE.md`'s claim".
That is a claim about `CLAUDE.md` which `CLAUDE.md` contradicts — it asks for the clause "in its
doc comment, its module comment, **or the comment above the block**" and states "[q]uotation marks
mean verbatim" of quotations rather than of doc comments. The first `//` comment read under the new
rule was stale. Thirteen landings, **two of them findings**:

1. **§7.8.3, Issue #128 — `content.rs`'s `draw_appearance` quoted the struck fourth bullet.** "All
   resources that are referenced from those forms and fonts shall be inherited from the resource
   dictionary of the page on which they are used" is retired into NOTE 3, which *reports* the rule
   of earlier versions rather than stating it — and NOTE 3 is the wider of the two, naming an
   annotation appearance stream where the bullet named only forms and Type 3 fonts. The behaviour
   does not move; its warrant becomes a documented choice about pre-2.0 and malformed files. This
   is the erratum the round walked into: it was reading §7.8.3 for `Do`.
2. **§8.9.7, Issue #19 — `inline_image.rs` quoted NOTE 3 without the half EC3 adds.** The device
   space names "never refer to resources in the ColorSpace subdictionary; they always identify the
   corresponding colour spaces either directly or via a default colour space (see 8.6.5.6
   \"Default colour spaces\")". No code moved: `ColourSpace::parse` has asked `/DefaultGray`,
   `/DefaultRGB` and `/DefaultCMYK` before answering with a device space all along, so the sentence
   was behind the code rather than in front of it.

The other eleven are already annotated in place by sessions 417 and 418 — `view.rs` on #468,
`crypt.rs` on #24, `appearance.rs` on #287, `optional_content.rs` on #225 — or are the
"; shall be an indirect reference" family.

**A quotation with an ellipsis in it.** `overlaps` compared a quotation whole, so a quotation of
*parts* of one sentence matched only where the struck passage was shorter than it. `CLAUDE.md`'s
own convention writes `…` for an elision, so this was blind to exactly the quotations a careful
writer produces. Split at the ellipsis it finds eight more landings, **four of them findings**:

3. **§9.6.2.2 and §9.6.2.1, Issue #47 and #48 — `tests/oracle.rs`, twice.** The
   four-hundred-and-eighteenth session corrected this quotation in three files and recorded that it
   had missed two others; this is one of them, and the sweep found it rather than a person.
   `CONTRADICTED_SUBSTITUTED_FONT` rested the compiled-in standard 14 on "[t]hese fonts … shall be
   available to the PDF processor", struck outright, and `CONTRADICTED_ZERO_WIDTH_SPACES` rested
   `standard_metrics` on §9.6.2.1's "PDF processors shall provide glyph widths and font descriptor
   data …", replaced by a cross-reference. Both now cite Table 109's permission and §6.3.2.2's
   requirement, which is the stronger warrant `pdf_font::standard` already carries.
4. **§12.7.5.2.2, Issue #386 — `appearance.rs` and `pdf-viewer.rs`.** The same struck sentence
   session 418 corrected in `form.rs` and in the ledger, in two more places. What survives is the
   definition — a control that responds "without retaining a permanent value".
5. **§7.6.4.3.4, Issue #325 — `crypt.rs`'s `hash_2b`.** Step (a) is rewritten into a two-case
   definition of a string `K0`. The concatenation is unchanged; the quotation was retired.
6. **§7.9.2.2.1, Issue #96 — `text_string.rs`.** The example is rewritten so that byte 8B is
   written `\213` rather than as a character the printed page could not show. The fact the test
   asserts is untouched.

**One false positive, and it is what set the rule.** Splitting at the ellipsis and asking whether
*any* segment matched reported `image.rs`'s §11.6.5.2 comment against a sentence about `/BaseFont`,
on the four words "the same as the". `overlaps` now asks for one segment quoting the passage whole
**or** every segment inside it, which keeps `structure.rs`'s long blockquote and drops this.

## Owed

- ~~**§8.9.5.4**~~ (finding 4) — **implemented in the five-hundred-and-fortieth, and the reason it
  had been declined was wrong.** The amended step a) is terminal and the amended d) *is* unreachable
  for a hidden base image; that is the amendment rather than a contradiction in it, because a) and
  b) dispose of every base image stating an `/OC` and c) and d) open at "Otherwise". Read that way
  the five steps are total, disjoint and reachable, which the 2020 four were not. ADR 0375. Nothing
  in the corpus moves: the corpus gate prints the same figures either way.
- ~~**§14.8.6.3's enclosure requirement**~~ — **read in the five-hundred-and-fortieth and declined
  by argument.** The amended sentence opens "[w]hen including mathematics structured as MathML",
  which addresses whoever writes the tagging: the enclosure under `Formula` and the namespace on
  every MathML type and attribute are `shall`s on a producer, and `CLAUDE.md`'s closed exclusion
  covers those. What a reader owes is done. The half that is a *validator's* — reporting the
  document that breaks either — is what keeps the row `partial`. And the clause turned out to carry
  the round's real finding: `doc/md/` writes its namespace name as `' … '` where the PDF sets
  `“ … ”`, which is why `conformance::quote::normalise` drops every shape of quotation mark now.
- **The 51 landings in the "struck out of another clause" bucket.** All of them were looked at this
  round and none is a finding — they are the "; shall be an indirect reference" family, four-word
  coincidences, and this round's own corrections quoting what they retired — but the bucket is a
  sort order and not a verdict, and it grows every time a correction is written.
- ~~**The ledger's single-quoted spans.**~~ **Read since the five-hundred-and-fortieth**, and there
  were 106 of them. The cause was real — an apostrophe is the same character as a closing single
  quote — and the fix is a rule about context rather than about the character:
  `conformance::quote::quoted_spans` opens a span only after a space or a bracket and closes one
  only before a space or ordinary punctuation, and stops at a double quotation mark so that
  §9.4.3's operator names cannot swallow what follows them. ADR 0375.
- **The remaining populations nothing reads at all**, now that four are read: a quotation in a
  Markdown file under `doc/`, and a quotation of a *table cell* rather than of prose. The first is
  the larger — this file, `doc/HANDOVER.md`, `doc/todo/` and the ADRs quote the standard
  constantly and no instrument compares any of it. Counting it is a round's work and is not owed
  until somebody has a reason to think it is wrong; the reason the four unchecked populations were
  each swept is that the first sweep of each found something.

## Two more copies of one struck sentence, found in the four-hundred-and-twenty-ninth

`check`'s in-clause bucket was 28 landings and every one was an annotation sessions 416–419 wrote in
place **except** `crates/viewer-host/tests/host_mappings.rs:138`, which still quoted §7.11.4.1's
"shall map name strings to file specifications" — struck outright by Issue #481 along with the two
bullets around it. `crates/viewer-host/src/panel.rs:144` quotes the same sentence and was in the
"elsewhere" bucket, because it cites §12.3.5 and `Landing::in_clause` files a landing by the clause
the quotation names rather than by the clause the sentence is in. **The bucket is a sort order and
not a verdict, for the fourth round running.**

`pdf_model::attachment` was corrected for this exact sentence in the four-hundred-and-eighteenth,
one crate away, and neither copy was swept with it — `doc/todo/01`'s fourth-sweep shape with an
erratum in place of a session's correction. Both now rest on what survives §7.11.4.1, which is its
NOTE about pre-PDF-1.6 identification, and on §7.7.4's name tree for what a key is. The corrected
comment in `panel.rs` quotes the wording it retired, so it will land every run from now on: this
file's own known false positive, and the reason the "elsewhere" bucket grows.
