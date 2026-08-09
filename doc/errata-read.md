# Errata Collection 3, read against this tree

What `tools/spec-errata check` named, and what each passage turned out to be. Written in the
four-hundred-and-seventeenth session so that nobody reads them twice. ADR 0252 built the tool;
ADR 0253 is this reading.

**The list read here is `check`'s output as it stood before this session**: 79 lines, 65 distinct
passages, of which the four-hundred-and-sixteenth session had already read three. So 62 distinct
passages — 76 of the 79 lines — are new below, and they are the whole of the round's task.

**The same session widened the instrument and the list is now 151 lines.** Two of the four findings below, and one further stale quotation, came from passages that only became visible then — they are in the *second* table rather than the first. The comparison was
whitespace-sensitive and both sides are extractions of the same glyphs by different programs, so a
passage one of them writes `inthe` and the other writes `in the` was called absent. 55 further
distinct passages became visible, and they are **not** read below — they are the next step, listed
at the foot of this file.

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

## The 79, by clause

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
| §8.9.5.4 Alternate images | 279 | #79 | **implements** | The algorithm is rewritten and three of its steps contradict `alternate_image`. See the finding below. |
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

## Not on the 79: three the corrected comparison found, and all three are findings

Each is a passage whose two extractions space a word differently, so the whitespace-sensitive
containment test called it absent. Each also lands on a page whose *outline* section is the next
clause along, so `Landing::in_clause` filed all three as coincidences — the bucket is a sort order
and not a verdict.

| clause | p. | outline says | issue | verdict | what it turned out to be |
|---|---|---|---|---|---|
| §12.5.2 Annotation dictionaries | 485 | §12.5.3 | #23, #34, #56 | **implements** | `BM` struck out of the list of entries a reader ignores, `MK` inserted, and the blanket "without regard to any other keys" removed. `/BM` was being ignored on every stored appearance stream. **Fixed.** |
| §9.6.2.2 Standard 14 fonts | 330 | §9.6.2.3 | #47, #48 | **quotes** | The clause's `shall` struck and its neighbour demoted to an informative NOTE. Three doc comments called it this program's warrant for the compiled-in fourteen. **Fixed.** |
| §14.8.6.1 Namespaces | 809 | §14.8.6.2 | #151 | quotes | The default-namespace sentence is replaced by one that states the order — the default applies *after* the role map has been applied transitively. `Tree::role` is that walk already. **Quotation annotated.** |

## The findings, and what was done

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
4. **§8.9.5.4, Issue #79 — the alternate-image algorithm is rewritten and this tree implements the
   retired one.** Three divergences, quoted in `content.rs::alternate_image` and in the ledger row.
   **Not fixed, deliberately**: the amended step a) ends "then nothing shall be shown", which reads
   as terminal and would leave the amended d) unreachable for a hidden base, so a rewrite would
   trade one contradiction for another. No corpus document states `/Alternates`.
5. **The instrument under-reported by 72.** See ADR 0253.

## Owed

- **The 55 newly visible distinct passages**, which no session has read. `spec-errata check` names
  them; among them are §7.2.3, §7.8.3, §8.4.5, §8.6.6.5, §8.9.6.3, §12.5.6.19, §12.7.4.3, §14.5,
  §14.8.6.2 and Annex F.3.5.
- **§8.9.5.4** (finding 4), with the carets' own words in the ledger row and the doc comment.
- **§7.3.10's grammar**: the lexer accepts `+5 0 obj` and `007 0 obj`, which EC3 makes malformed
  rather than merely undescribed. A reader's tolerance, but an undocumented one.
- **§7.5.6's multi-update case**: `version::document` reads the newest catalog's `/Version` only,
  so an update that *lowers* it lowers the document's version. EC3 forbids that outright.
- **§14.8.4.7.2's framing** in `structure.rs` and the ledger: `Annot` and `Form` *enclose* rather
  than *associate*, and a `Form` may enclose the widget with no content at all.
- **§14.6.1's Figure 9** in `variable_text.rs`'s `PERMITTED` list: marked-content operators are now
  admitted inside a text object, so a `/DA` carrying one is dropped on a stale reading. Harmless —
  a bracket round nothing marks nothing — but silent.
- **The ledger's 977 quoted spans**, which `spec-errata` still does not scan: it reads rustdoc
  blockquotes, and this session found two of the ledger's stale quotations by hand.
