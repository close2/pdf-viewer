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

**One row below was never `check`'s to name, and it is the most important thing this file learned
after it was finished.** ISO/TS 32001 §5.1.3's deletion (#236) is in the first table because the
five-hundred-and-fifty-fifth session went looking for it, not because the tool printed it: `check`
compares *the tree's quotations* against struck passages, so an erratum over text nobody has
written yet is invisible until somebody writes it. That session wrote it — three quotations of a
deleted subclause — and `check` caught all three before the commit, which is the tool working in
the direction it was built for. **The rule that follows: a round implementing a clause runs
`spec-errata emit` on that document before it writes, and not `check` afterwards alone.**

**It is no longer one row, and the later two show that `check` has two blind spots rather than
one.** §7.4.3's #293 is #236's, a clause away: a pure addition, over text nobody had quoted.
§10.3.1's #181 is a different one — text this tree *had* quoted, in two places, over a struck run
of **two words**, under the four-word floor `check` filters on. So a quotation can sit on retired
text and pass the check that exists to find exactly that, and the only instrument that sees it is
`emit` read against the caret's `/Rect`. The rule above is unchanged and now has a second reason.

**And a collection of errata is not a corrected standard**, which the seven-hundred-and-twentieth
session found by meeting the first place where it is inconsistent: Table 161's alphabetic page
labels carry two annotations, both `Review`/`Accepted`, that cannot both be applied — one rewriting
*AA to ZZ* as *AA to AZ*, the other inserting *AAA to ZZZ for the next 26* after it. Every round
before that had treated an accepted erratum as settling a question, which works exactly while the
collection agrees with itself. **So an erratum is evidence about the standard, in the way another
renderer is evidence about our reading** (`CLAUDE.md` principle 5): where two disagree, the
published clause and its own arithmetic decide, and the disagreement is recorded rather than
resolved. ADR 0601, and the section at the end of this file.

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
| §7.4.3 ASCII85Decode | 52 | #293 | **applies** | A whole sentence added: "If the ASCII85Decode filter encounters the character ~ in its input, the next character shall be > and the filter will reach EOD. Any other characters shall cause an error.", with a NOTE crediting the PostScript Language Reference Manual's clause 3.13.3. **`check` never named it, for #236's reason one clause family along**: it is a pure addition over text nobody had quoted. `filter.rs` already fails a `~` not followed by `>`; what the sentence newly buys is *outside* the filter — §8.9.7's inline image extent is derived from the marker since the six-hundred-and-thirty-first session, and that needs the marker to be a rule rather than a convention. ADR 0464. |
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
| §10.3.1 CIE-based to device | 376 | #181 | **quotes** | The dated *ISO 15076-1:2010 (ICC.1:2010)* struck from the clause's last sentence, replaced by "the appropriate ICC specification (see "Table 66 - ICC profile versions supported by ICCBased colour spaces")" — the same erratum as §3's, §8.6.5.5's and §0.4's, now in the one place it states a requirement. **`check` never named it and `emit` did**: the struck run is two words, under `check`'s four-word floor, and this is the third row here found by the rule that a round runs `emit` before it writes. `emit` files it under §10.4.1's heading — page 376 carries §10.2 through §10.4.1 — and the StrikeOut's `/Rect [83.128 333.629 238.273 346.049]` is over §10.3.1's last line, which `pdftotext -bbox` puts at (86.42, 494.66)–(237.24, 507.60) on a 841.92-point page. Two quotations of the retired words, in `colour.rs`'s `BRADFORD` and §10.3.1's ledger row, are prose now. The erratum *strengthens* both: `icc.rs` reads the profile header's version byte and accepts 2.x and 4.x alike, which is Table 66's amended shape rather than one dated edition's. Found in the six-hundred-and-fifty-sixth session. |
| §10.6.5.4, §10.6.5.6 | 391, 395 | #310, #12 | cites | ×2. Halftone dictionaries. Table 130's `TransferFunction` gains the "shall be present if the dictionary is a component of a Type 5 halftone" sentence, and Table 132's "The value shall not be 5" becomes "The halftone shall not be a Type 5 halftone" — both producer rules, and both already the shape `doc/md/` carries. Neither changes what `ext_gstate::halftone_transfer` reads. **These rows said "untouched — §10.6 is inapplicable" until the six-hundred-and-seventy-seventh session**, which found that the condition covers a halftone *screen* and not the `TransferFunction` §10.5's second bullet reads out of a halftone dictionary; the seven §10.6.5 rows are `implemented` now (ADR 0505). |
| §12.5.6.9 Polygon/polyline | 506 | #444 | cites | `/Vertices` "shall be ignored if Path is present". `appearance::path` tries `/Path` first and structurally never reads `/Vertices` after it. |
| Table 234 `/I` (p.558) | 558 | #468 | **quotes** | `/I` is no longer restricted to `MultiSelect` fields and the "value is an array" trigger is deleted. The code was already right; **the erratum vindicates it** — writing `/I` beside a single selection was a stretch of the retired wording and is the plain sense of the amended one. Two quotations corrected. |
| §12.7.5.5 Signature fields | 561 | #680 | untouched | The seed value dictionary's `/Ver`. `/SV` is unread; the ledger says why. |
| §12.7.8.3.1 General | 576 | #173 | cites | "Although FDF file encryption is deprecated" — the disambiguation `forms_data.rs` already made in prose. |
| §12.8.1 General | 582 | #685 | cites | Table 254's `/Page` loses "for annotations in FDF files", which restated the table's own title. |
| §13.2.4.2, §13.2.7.2.2, §13.5, §13.6.4.1 | 638–672 | #414, #449, #481, #150 | untouched | ×4. Clause 13 is out of scope by `CLAUDE.md`'s closed list. |
| §14.6.1 Marked content | 736 | #335 | cites | **"They may not occur within a graphics object" is struck**, and marked content may appear "within a text object". The interpreter's marked-content stack was never coupled to `BT`/`ET` — it was written from the *surviving* nesting paragraph, which already gives `BT BMC … EMC ET` as valid, and existing tests across `accessibility.rs`, `logical_order.rs` and `logical_structure_example.rs` use the now-explicitly-legal form. One stale comment about Figure 9 in `variable_text.rs` was owed and is settled below, in `PERMITTED`'s own doc comment, as a documented choice. |
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
| ISO/TS 32001 §5.1.4 | 10 | #404 | **implements** | SHAKE256's fixed OID gives way to RFC 8702's algorithm identifiers. **This row read "untouched — no SHA-3 or SHAKE anywhere in the tree" until the five-hundred-and-fifty-fifth session, when `cms::Digest` gained all four** (ADR 0390), which makes it the errata table's own decay: a verdict of *untouched* is a claim about the tree and expires the moment the tree implements the clause. Read now: the struck sentence pinned `id-shake256`, the caret defers to "RFC 8702, 3.1 and RFC 8419, 3.1, 3.2", and the NOTE fixing the output at 512 bits was **not** struck — so the length this program squeezes to is a documented choice standing on an unstruck NOTE, not a requirement. `Digest::Shake256` carries it. |
| ISO/TS 32001 §5.1.3 | 5, 10 | #236 | **implements** | ×2 (the contents line and the clause). "Delete all of clause 5.1.3" — so Table 256's `/DigestMethod` is **not** extended with the SHA-3 family, and the four are added to Table 237 (§5.1.2) and Table 260 (§5.1.4) only. **`check` never named this one, and the reason is the shape to remember**: the tool compares the tree's quotations against struck passages, and nothing here quoted §5.1.3 until a round set out to implement it — at which point it quoted the retired text in three places and `check` caught all three before the commit. An erratum over text nobody has written yet is invisible until somebody writes it. |

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
| §8.4.3.5 Mitre limit | **177** | #154 | **cites** | **An erratum this collection had never recorded, and the fourth `Caret` with no `StrikeOut` — after #293, #34 and #536.** A bare Caret, `Review/Completed`, `/Rect [87.78 387.05 96.80 394.40]` — which lands between "limit" and "shall" on the line `pdftotext -bbox` puts at 441.5 — inserting "shall be a number greater than or equal to 1.0 and", so the clause's second sentence becomes *The miter limit shall be a number greater than or equal to 1.0 and shall impose a maximum on the ratio of the miter length to the line width.* `check` cannot see it: nothing is struck, so there is no retired text for a quotation to land on. **It vindicates the code and replaces its reasoning.** `content.rs` clips the limit below at 1 and §8.4.3.5's ledger row justified that by *inferring* the floor from the clause's own ratio `1 / sin(φ/2)`, which never goes below 1; the erratum states the floor outright, and a clause that says a thing is a stronger answer than one that implies it (`CLAUDE.md` principle 5). `a_miter_limit_below_one_is_clipped_into_range` is now a test of a repair to a file the standard calls non-conforming rather than of a derived bound. |
| §8.4.5 ExtGState | 180 | #371 | cites | Table 57's `/FL` loses the 0-to-100 range, which moves into §10.7.2 (below). The permission this tree exercises is in neither half. **These two rows said Table 58 until the seven-hundred-and-twenty-fifth session** — the path construction operators — where the graphics state parameter dictionary is Table 57, which §10.7.2's own clause text names. It is the same two numbers §10.7.5's ledger row records having confused for the whole of its life, and the ninth sweep could print neither, for two different reasons: this row attributes `/FL` to Table 58, which states *operators* rather than entries, so the citation lands in the sweep's keyless count instead of among its absences; §10.7.2's row below writes the number with no key beside it, so it is not an attribution at all. Calibrated per trap 13, one instrument over three states of this cell: with `Table 58` the sweep prints nothing and its keyless count carries the citation, with `Table 166` — a table that does state entries — it prints the citation and offers Table 57 as the table stating the key, and with `Table 57` it agrees. |
| §8.4.5 | 182 | #360 | cites | `/UseBlackPtComp` loses "The default value is: Default." — an entry whose stated default already said "up to the PDF processor". `BlackPoint::Default` compensating is this processor's determination either way, and `content.rs` says so. |
| §8.5.3.1 General | 187 | #549 | **implements** | "Attempting to execute a painting operator when the current path is undefined … shall **generate an error**" becomes "shall **be ignored**". §8.5.3.1's row carried that as its one departure — "here it paints nothing, which is the recovery a viewer owes a malformed file" — and the amended clause states the recovery. Corrected. |
| §8.5.3.2 Stroking | 188 | #103 | quotes | "**This** rule shall apply only to zero-length subpaths of the path being stroked" becomes "**In the opaque imaging model, this** rule …". `pdf-render`'s `degenerate.rs` and both backends quote the unqualified sentence, which is what `doc/md/` carries and what the quotation gate reads; the erratum adds a scope rather than an answer, and §11.6.2 already makes a stroked path one object in the transparent model. Annotated in all three. |
| §8.5.3.2 | 188 | #434 | untouched | A NOTE that the path-painting operators "also serve a purpose in path construction as they may affect the current graphics path". States no requirement. |
| §8.6.5.5 ICCBased | 206 | #181 | cites | The other half of the erratum read at p.207: the table becomes one of ICC *profile header* versions. |
| §8.6.6.5 DeviceN | 221 | #309 | untouched | The `/Colorants` restriction to non-`NChannel` spaces is struck. `/Attributes` is unread; nothing in the tree names `NChannel`. |
| §8.9.6.1 | 280 | #79 | **implements** | The second mark of §8.9.5.4's rewrite, carrying the amended step c) and the new step e). Finding 4 above, still declined for the reason stated there. |
| §8.9.6.3 Explicit masking | 281 | #333 | untouched | A cross-reference repointed from §9.6.5.3 to §9.6.4, and "need almost always be used" softened to "is normally used", in a NOTE about stencil masks and glyph bitmaps. |
| Table 74 `CS`/`cs` | 285 | #19 | cites | **The erratum vindicates the code.** "The names DeviceGray, DeviceRGB, DeviceCMYK and Pattern always identify the corresponding colour spaces directly" becomes "either directly **or via a default colour space** (see 8.6.5.6)" — which `colour.rs` has done since the twenty-fifth session, remapping through `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK`. |
| §9.6.2.1 General | 329 | #106 | cites | "; shall be an indirect reference" again. |
| §9.6.4 Type 3 fonts | 333 | #128 | **implements** | Table 110's `/Resources` row is rewritten and loses the page fallback it stated. Same finding as §7.8.3's. |
| §9.6.4 | 334 | #128 | **implements**, quotes | Step d) is replaced by a pointer to §7.8.3. `type3.rs::resources` quoted the retired two-place rule and implemented it. **Fixed.** |
| §9.8.1 General | 358 | #11 | cites | Table 120's `/FontName` gains a Type 3 case: it matches the font dictionary's `/Name` for a Type 3 font and `/BaseFont` for every other. A writer's rule; `collection.rs` matches `/FontName` against a TrueType collection's PostScript names, which the surviving half describes. |
| §10.6.5.6 Type 5 halftones | 396 | #311 | **implements** | **A StrikeOut with no replacement**, over the whole paragraph beginning "When a halftone dictionary of some other Type appears as the value of an entry in a Type 5 halftone dictionary" — which takes with it the sentence a round would reach for first, "[t]his is in contrast to such a dictionary's being used as the current halftone parameter in the graphics state, which shall apply to all colour components", and the nonprimary-colourant fallback to the gray screen. The six-hundred-and-seventy-seventh session was about to quote the first of those and found the strike by running `emit` before writing, which is the whole of that rule's purpose: **`check` does not catch this one**, because the extracted words run together ("graphicsstate", "halftonedictionary") and match nothing. What the code rests on instead is live text — Table 52's "[a] halftone screen for gray and colour rendering", one per graphics state, and this clause's own opening about why Type 5 exists — which reaches the same answer. ADR 0505. |
| §10.7.2 Flatness | 397 | #371 | cites | "It shall be a positive number" gains the 0-to-100 range and the meaning of 0, moved here from Table 57. The permission this row rests on — "PDF processors may choose to ignore any flatness tolerance" — is untouched, and `i` is still matched and discarded. |
| §12.5.2 Annotation dictionaries | 483 | #287 | quotes | "[i]f an annotation dictionary includes the BS entry, then the Border entry **is** ignored" becomes "**shall be** ignored". `appearance.rs` quotes it in two places; the precedence it implements is the same one. Annotated. |
| §12.5.6.21 Screen | 513 | #42 | cites | "If AP is not present, the screen annotation shall not have a default visual appearance and shall not be printed" struck. §12.5.6.18's row already refuses an appearance-less screen annotation *and reports it*, which is a stated choice rather than a rule this clause supplied. |
| §12.5.6.24 Projection | 520 | #42 | untouched | The rule forbidding an `/AP` on a zero-area projection annotation struck. Nothing enforced it. |
| §12.7.4.1 General | 546 | #313 | cites | **The erratum vindicates the code.** "a field dictionary may also be an annotation dictionary" becomes "a **Widget** annotation dictionary (see 12.5.6.19)", which is the only merge `appearance.rs` performs. |
| §12.7.4.3 Variable text | 549 | #393 | untouched | An EXAMPLE cross-reference repointed at the example above it. |
| §12.7.5.5 Signature fields | 561 | #158 | cites | `/DigestMethod`'s DSA-and-SHA-1 sentence becomes "[s]ome signature mechanisms require a specific digest function … the value of this entry shall be ignored". `/SV` is unread; the ledger says why. |
| §12.7.5.5 Signature fields | **560** | #131 | **cites** | **An erratum this collection had never recorded, and the reason it had not is the finding.** It adds to Table 236's `/P` the carve-out §12.8.2.2.1 already states for `/DocMDP`: an incremental update carrying only a DSS (§12.8.4.3) or a document timestamp (§12.8.5) is not a change to the document as the entry's own choices define one. It moves nothing here, because nothing here reads that `/P` — and *that* is why the erratum went unread: the ledger row disposed of the entry as being about signature validation, so the collection had no reason to look at the page. An erratum that carves an exception out of a permission is evidence the entry states a permission. §12.7.5.5's row now carries the question with its population (28 crawled witnesses), and this row is what says the standard has been amended there since. ADR 0502. |
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

## The errata `check` cannot see, swept in the five-hundred-and-sixty-second

The five-hundred-and-fifty-fifth session found ISO/TS 32001 §5.1.3 deleted by Issue #236 while this
tree had been asserting its content since ADR 0314, and wrote the rule at the top of this file: **a
round implementing a clause runs `spec-errata emit` on that document before it writes, and not
`check` afterwards alone.** This round ran `emit` over all fourteen PDFs and asked the output a
different question from `check`'s — not *does a quotation land on struck text*, but **does an
erratum move ground the ledger is standing on**, whether or not anybody quoted a word of it.

`emit` prints **1097 annotations over the three documents that carry any**. Twenty of them are
structural — a clause deleted, moved or renumbered, a subclause inserted — and those are the ones
`check` is blind to by construction: a heading is not a sentence, so no quotation can land on it.

| issue | what it does | what it stood under |
|---|---|---|
| **#452** `Completed` | "Move entire subclause 14.7.5.1.1 up one heading level to become 14.7.5.2 and renumber later subclauses of 14.7.5 appropriately. Subclause text is otherwise unchanged" | five ledger rows and some twenty source citations, all of them the 2020 numbers. **Recorded in §14.7.5.1.1's note.** |
| **#196** `Completed` | inserts "7.6.5.3 Public-key security permissions" below Table 23's NOTE, "current text and Table 24 remain unchanged" | §7.6.5.3's existing row holds the 2020 number for what becomes the clause after it. **Recorded in §7.6.5.2's note.** |
| #133 `Completed` | inserts §14.8.4.7.3 (link elements) and renumbers ruby to §14.8.4.7.4 | already read — ADR 0273, the four-hundred-and-thirty-seventh. The instrument agreeing with a known finding is what says it works. |
| #236 `Accepted` | "Delete all of clause 5.1.3" (ISO/TS 32001) | already read — the five-hundred-and-fifty-fifth. |

**The numbers are not changed anywhere.** `doc/md/` is the published text, the citation gate resolves
against it, and `the_ledgers_own_prose_names_clauses_and_tables_that_exist` refuses a post-erratum
number outright — which this round confirmed by writing two of them and watching the gate fail. What
changes is that the three affected families now say so in their own notes.

### And one erratum that changes a requirement rather than a number

Filtering the same output for a strikeout whose text is a table's *requirement* word — `Optional`,
`Required`, `Deprecated` — printed nine pairs. Eight are already read or fall outside scope; the
ninth is **Issue #22** (`Completed`), which replaces Table 166's `/AP` requirement "Optional; PDF
1.2" with "Required except for conditions listed below (PDF 2.0); optional in PDF 1.2 through PDF
1.7", the conditions being a degenerate `/Rect` and a `/Subtype` of `Popup`, `Projection` or `Link`.

**The 2020 text already said it in prose**, which is the part worth keeping: "[a] PDF writer shall
include an appearance dictionary when writing or updating the PDF file except for the two cases
listed below". So the erratum moves a requirement into the column it belonged in, and two doc
comments in `pdf-model/src/view.rs` — the file that writes annotations under §7.5.6 — said "an
annotation with no `/AP` is legal", which was false before the erratum as well as after. Corrected,
with the one place this program departs from the `shall` argued in place and in §12.5.2's row:
`write_retypings` removes a producer's appearance for a free text annotation whose new text it could
not lay out, because Table 177 makes `/AP` decisive over `/DA` and the alternative is drawing words
the file no longer states. It is reported — `Written::unappeared` — which is what makes it a
departure rather than an oversight.

**What this adds to the method.** `check` asks whether this tree quotes something struck; that is
one direction of one question. The other direction is whether an erratum has moved something this
tree *claims*, and a claim needs no quotation. `emit` plus the ledger is the instrument for it, and
its three signals are cheap to filter for: the words *delete*, *move* and *renumber*, and a
strikeout whose whole text is a requirement word.

## The erratum a row records, against the words it quotes — the five-hundred-and-ninety-first

The five-hundred-and-ninetieth session found the §14.8.4.7.2 row naming Issue #437 since the
four-hundred-and-eighteenth and quoting the sentence it struck out two sentences later, with four
places in `crates/` quoting the same sentence as current text. **A row that records an erratum is
not a row that has applied it**, and this file's whole first table is a list of rows that record
one. So the question became a command:

```sh
cargo run --release -p spec-errata -- applied doc/*.pdf
```

It reads every place that *names* an erratum — a ledger note, a run of comment lines, a Markdown
block — and asks whether the quotations inside it match what that erratum struck out or what it put
there instead. **Nothing is inferred**: `check` has to guess which clause a quotation belongs to
from the nearest citation above it, and here the erratum is named as data by the writer, with the
`StrikeOut` and the `Caret` supplying both sides. ADR 0426; `doc/todo/01` has the noise shapes and
`doc/todo/02` §4 the invocation.

**Its first run put 26 hits on the read-first list and two of them were the §14.8.4.7.2 shape one
clause family over**, both in §9.6.2.2's row:

| where | issue | what it was |
|---|---|---|
| §9.6.2.2's row, its opening sentence | #47, #48 | "[t]he clause asks for '[t]hese fonts, or their font metrics and suitable substitution fonts'" — the sentence the erratum strikes outright, in the present tense, three thousand characters above the same note's own record that it does. **Corrected.** |
| §9.6.2.2's row, beside ADR 0358 | #47, #48 | The five-hundred-and-twenty-third session quoted the same struck sentence as "the clause's substitution `shall`", four sentences from that record, while reading the clause for a substituted glyph's shape. **Corrected, and the conclusion is stronger without it**: with the availability `shall` retired the clause states no requirement about a substituted face at all. |

The other twenty-four are the annotations sessions 417 to 419 wrote in place, where the retired
words are kept deliberately because `doc/md/` is what the gate verifies against, plus four dated
ADR records.

### And the twelfth sweep's comparison was blind to two spellings this project's own rules produce

`squeezed` was `normalise` plus the spaces and the case. It kept **square brackets** — so
`CLAUDE.md`'s own `"[e]ncloses one or more PDF annotations"` spelling of an altered first letter
made a passage unfindable — and it kept **dash shapes**, so a quotation carrying a table caption
could not match `doc/md/`'s `Table 118 -Additional entries`. `conformance::prose::folded` had
answered both for the Markdown sweep (ADR 0375) and is what `squeezed` is now: one comparison in
the crate rather than two. Every folding is applied to both sides, so it can only hide a finding
and never invent one.

**It moved `check`'s three levels** — 151 struck passages still in the conversion to **178**, 73
in-clause landings to **86**, 272 elsewhere to **293** — and the thirteen new landings, read one by
one, held **three more defects**, none of which `applied` could see because none of the three named
the erratum:

| where | issue | what it was |
|---|---|---|
| §9.6.2.1's ledger row | #47, #48 | Quotes **both** struck sentences — the closing paragraph requiring a processor to supply glyph widths and font descriptor data for the standard 14, and the half of Table 109's `/FontDescriptor` row saying that stating the entries overrides a standard font — and names the erratum nowhere. Session 418 corrected this sentence in three doc comments and the 419th in two more; the row the code's own entry points at was never swept with them. **Corrected**, onto Table 109's permission and §6.3.2.2, which is the warrant `pdf_font::standard` already carries. |
| `pdf-font/src/loading.rs`, `pdf-model/tests/composite_fonts.rs` | #462 | Two rustdoc **blockquotes** — the one population this project gates — quoting §9.10.3's "[t]he only pertinent entry in the CMap stream dictionary … is UseCMap, which", struck by Issue #462, which also inserts a table of the `/ToUnicode` stream's own entries. §9.10.3's row recorded the erratum in the five-hundred-and-eighty-seventh and closed by warning that a later round quoting it as current would be quoting removed text; the quotations were already there. **Both corrected**, and neither behaviour moves — `/UseCMap` is the entry under both readings. |

### What the repair leaves owed, with its number

The struck-passages list grew by **27 lines** and every one of them is a passage nobody has read:
three in Annex A, which says *informative* on its own title line, four in clause 13, which
`CLAUDE.md` excludes, and the remaining twenty across §7.5.8.2, §7.6.3.1, §7.10.4, §8.2, §9.6.2.3,
§9.8.3.3, §9.10.3, §12.3.3, §12.5.6.3, §12.5.6.5, §12.7.5.2.3, §12.11.1, §12.11.2, §14.8.2.2.2,
§14.10.5.3, ISO/TS 32001 §5.1.4 and §0.4. **They are visible for the first time and they are a
round's reading**, at the rates this file already records — 66 passages gave two findings and the
next 54 gave one. Recorded here rather than left to be rediscovered; `doc/todo/48` carries it as
owed.

## The filter becomes a command, and it found two the hand-run missed

The paragraph above ends by naming three words to filter for. **A filter written down is a filter
somebody re-invents**, which is `CLAUDE.md`'s own rule and the reason the fifteenth sweep went unrun
for twenty-four rounds (ADR 0319), so since the five-hundred-and-sixty-fifth session it is a command:

```sh
cargo run --release -p spec-errata -- moved doc/*.pdf
```

It takes every annotation whose own instruction uses one of `move`, `renumber`, `delete` or `insert`
**and names a clause number** — a number with a full stop in it whose first component is one of the
technical clauses, which is what keeps `PDF 1.7`, `Table 24` and `ISO 32000-2` out — and prints
beside each one what this tree has standing on that number: its ledger rows, its `§` citations under
`crates/`, `tools/` and `fuzz/`, and its mentions in this project's own Markdown. That last column is
the reader's hazard as a count.

**Its first run printed 15 of 2 865 annotations and two of them were unrecorded**, both of which the
five-hundred-and-sixty-second's hand-filter for *Move*, *delete* and *renumber* had walked past:

| issue | what it does | what stands on it |
|---|---|---|
| **#477** `Completed` | "all of subclause 12.3.6 Navigators was moved and demoted one level to subclause 12.3.5.3 Collection navigators" | §12.3.6's ledger row, 7 source citations, 10 mentions in these documents. **Recorded in §12.3.6's note.** |
| **#256** `Completed` | "[t]he remaining text in subclause 12.6.4.8 about Base and the URI dictionary … applies to all relative URIs in a PDF document and is not limited to only URI actions as is currently implied. A future edition of ISO 32000 will move this text into a new subclause" | §12.6.4.8's row and 34 source citations. **Recorded in §12.6.4.8's note.** |

**#477 is the same shape as #452 and it is invisible to a grep for the verb**, because this collection
writes an instruction in two voices: #452 says "Move entire subclause 14.7.5.1.1", an imperative, and
#477 says "was moved and demoted", a past passive. A command that takes the verb *and* the number
finds both; a person filtering by eye finds the one that reads like an instruction. That is the whole
argument for the program.

**#256 is a different kind and is the more interesting of the two**: it changes no number and no
sentence, and says that a clause's text has a wider *scope* than its placement implies. Nothing here
is owed by Errata Collection 3 — the move is deferred to "a future edition" — but the reading is
owed now, and it is taken: `uri::resolve` is RFC 3986's reference transformation over a base and a
reference and knows nothing about actions, so the arithmetic is already general. Its one caller is
§12.6.4.8's action; the other relative-URI site this tree reads is §7.11.2.2's URL-based file
specification, which `file_spec::is_valid_relative_url` validates and does **not** resolve, because
nothing here fetches a URL and there is no target for a resolved one to name. A documented choice
with the erratum beside it, and a second caller the day a host opens one.

### What this tree does about a clause number the errata have moved

Asked and answered in the five-hundred-and-sixty-fifth, because a citation that is right against the
published text and wrong against the amended one is a real hazard for a reader. Three parts, and the
first is not a preference:

- **The published numbers stay, and the gate enforces it.** `doc/md/` is the text every `§` is
  resolved against, and `the_ledgers_own_prose_names_clauses_and_tables_that_exist` refuses a number
  ISO 32000-2 does not have. §12.3.6's note above was written with a section sign in front of the
  amended number and **failed that gate**, which is the second round running to confirm it by walking
  into it. So the amended number is written as the erratum writes it — *subclause 12.3.5.3* — and a
  renumbering can never quietly enter the tree as a citation.
- **The row is where the amendment is recorded**, because a row is what a round reading a clause
  opens. Four families say so in their own notes now: §14.7.5.1.1, §7.6.5.2, §12.3.6 and §12.6.4.8.
- **And the command is what makes it findable at all**, because a note in one row is not read by the
  round that writes the twentieth citation in `crates/`. Nothing is renamed, nothing is deprecated and
  no comment is annotated: a citation of §12.3.6 is correct about the standard this project is checked
  against and *incomplete* about a reader holding Errata Collection 3, and one command closes that gap
  in a second.

**The noise it prints rather than filters**: four of the fifteen renumber a *NOTE* rather than a
clause (#598, #649, #655, #303/#334) and four insert a NOTE or an EXAMPLE whose replacement text
happens to cite a clause (#53 twice, #151, #65, #62). A NOTE's number moves nothing this tree cites,
and the instruction says which it is in its own words — which is cheaper to read than a predicate that
would have to tell an inserted heading from an inserted note.

## The twenty the repaired comparison uncovered, and there were nineteen — the five-hundred-and-ninety-fourth

The section above ends by naming the debt with a number: *twenty* unread struck passages, over
seventeen clauses. **The number was nineteen and the clause list was right.** It was arrived at by
subtracting the Annex A and clause 13 lines from the 27 the repair added, and clause 13 gained
**five** lines rather than four — #179 in §13.2.4.2, a third #156 in §13.6.4.7, #283 in §13.6.7.4, and
#127 in §13.7.2.2.4 and again in §13.7.2.3.2. So the split is **3 + 5 + 19** and
not 3 + 4 + 20, and it is recorded that way rather than quietly corrected: the seventeen clause
names the previous section printed name nineteen lines, because §7.5.8.2 carries two and §12.11.2
two.

**How the nineteen were identified, which is the part worth keeping.** The tables in this file
count lines unevenly — some rows carry an explicit `×N` and most do not — so they cannot be
subtracted from a fresh run. What can is the instrument itself: `squeezed`'s pre-repair body is
four lines in the commit that replaced it, so a copy of the tree with those four lines back prints
**exactly 151**, and `diff` against the 178 names the 27 with nothing inferred. A comparison that
is a function of the source is a comparison a later round can re-derive, which a table of
verdicts is not.

Every line is filed under the clause the *outline* puts its page in, and eight of the nineteen are
a page-straddle away from the clause the sentence is in — the bucket is a sort order and not a
verdict, for the fifth round running. Both are given below.

| outline says | the sentence is in | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|---|
| §0.4 | §0.4 | 15 | #181 | untouched | The ICC.1 bullet in the Introduction's list of changes, deleted with no replacement: the dated ISO 15076-1 reference "can be supplemented by the Errata list and approved revisions available from the ICC website". The same erratum as §3's and §8.6.5.5's, in the Introduction, which states no requirement. |
| §7.5.8.2 | §7.5.8.2 | 81 | #246 | **cites — the erratum vindicates the code** | The directness rule a cross-reference stream's reader stands on is widened from a condition to a rule. 2020: "If the stream is encoded, the Filter and DecodeParms entries in" Table 5 "shall also be direct objects." Amended: "The values of all entries shown in" it "shall also be direct objects. For arrays, all array elements shall be direct objects and for dictionaries, all key values shall be direct objects as well. The F entry defined in Table 5 shall not be used." `xref::decode_direct` has read both entries straight from the dictionary unconditionally since it was written, and refuses an indirect `/Filter` outright; the new prohibition on `/F` is met by construction, since nothing here opens §7.3.8.1's external file. **Recorded in §7.5.8.2's note.** |
| §7.5.8.2 | §7.5.8.2 | 81 | #720 | cites | The same change from the other side: "not listed in" Table 17 struck from the bullet that says which entries *may* be indirect, because #246 makes the exception set two tables rather than one. `Root` stays the example of one that "shall be indirect". |
| §7.6.3.1 | §7.6.3.1 | 89 | #95 | untouched | RSA Security's postal address struck from the availability note for PKCS#5. Editorial. |
| §7.10.4 | §7.10.3 | 143 | #30 | cites | Type 2's `/Domain` constraint becomes two bullets and not one sentence: "if N is not an integer, all values of x will be non-negative; and … if N is negative, no value of x will be zero." The rule is unchanged and it binds the *file*; `function.rs`'s exponential arm is `x.powf(N)` and rests on it. |
| §8.2 | §8.2 | 162 | #85 | untouched | Table 32's marked-content row cited "Table 351 -Entries in a data dictionary" for the five operators; the erratum points it at Table 352. A pointer in a summary table, and all five operators are interpreted. |
| §9.6.2.3 | §9.6.2.2 | 330 | #384 | **a finding** | The fourteen names become a bulleted list in alphabetical order — and the erratum's own text spells the fourteenth `Courier-BoldOblique`. See the findings below. |
| §9.8.3.3 | §9.8.3.3 | 364 | #5 | **a finding** | The sentence this row calls a self-contradiction is repaired. See the findings below. |
| §9.10.3 | §9.10.3 | 372 | #462 | quotes | Already read, in the five-hundred-and-ninety-first, by the *other* half of `check`: two rustdoc blockquotes quoted "[t]he only pertinent entry in the CMap stream dictionary … is UseCMap, which", and both were corrected. The struck passage itself becomes visible only now, which is the two halves of `check` having different populations again. |
| §12.3.3 | §12.3.2.4 | 459 | #162 and #288 | **a finding** | A named destination's dictionary form and its `/SD`. See the findings below. |
| §12.5.6.3 | §12.5.6.3 | 496 | #479 | cites — the erratum vindicates the code | The sentence defining a state annotation cites "Table 176 -Additional entries specific to a link annotation" for the `/IRT` entry, which is a link's table; the erratum cites Table 172, the markup annotations' one. `conformance`'s own table claims already put `/IRT` under Table 172, and `markup::group_source` cites §12.5.6.2 rather than any table. **Recorded in §12.5.6.3's note.** |
| §12.5.6.5 | §12.5.6.3 | 497 | #479 | cites | The same correction on the bullet two paragraphs down, where the entry that "shall refer to the original annotation" cites Table 176 as well, and which the page break files under the next clause. |
| §12.7.5.2.3 | §12.7.5.2.2 | 551 | #386 | quotes | Already read, in the four-hundred-and-eighteenth and -nineteenth: "Because this type of retains no permanent value, it shall not use the V and DV entries in the field dictionary", struck with no replacement, corrected in `form.rs`, `appearance.rs`, `pdf-viewer.rs` and §12.7.5.2.2's row. This file recorded then that #386 was **not** on the 151; it is on the 178. |
| §12.11.1 | §12.11.1 | 621 | #187 | cites — the erratum vindicates the code | Table 273's `/S` row says "See" Table 276 "for valid values", which is the requirement *handler* dictionary; the erratum says Table 275, the requirement types. `requirements::Kind::read` has matched Table 275's names since it was written. **Recorded in §12.11.1's note.** |
| §12.11.2 | §12.11.1 | 622 | #187 | cites | The paragraph above Table 274 is rewritten to name Table 273 as the table its two type-specific keys are additional to, and the table is retitled "Additional entries for specific types of requirements". `/Encrypt` and `/DigSig` are unread and §12.11.1's row says why. |
| §12.11.2 | §12.11.1 | 625 | #187 | cites | The same retitling on the table's own caption, four pages later, because Table 274 spans them. |
| §14.8.2.2.2 | §14.8.2.2.2 | 762 | #484 | quotes | Already read, in the four-hundred-and-eighteenth: the paragraph carrying two `shall`s about the marked-content artifact forms is split into a NOTE 2 with both softened to "is", and `tests/accessibility.rs`'s blockquote stays verbatim because `doc/md/` is what the gate verifies against. Like #386, it was "not on the 151" and is on the 178. |
| §14.10.5.3 | §14.10.5.3 | 827 | #222 | untouched | "Table 393 -Entries in a Web Capture command dictionary" repointed at Table 394, the command *flags*. The row is `inapplicable` and reads neither. |
| ISO/TS 32001 §5.1.4 | ISO/TS 32001 §5.1.3 | 10 | #236 | implements | Already read, in the five-hundred-and-fifty-fifth: "Delete all of clause 5.1.3", so Table 256's `/DigestMethod` is not extended with the SHA-3 family. The deleted clause's body sits on the page the outline files under §5.1.4, which is why it appears under that heading here. The first table's own row for it says `check` never named it; `check` names it now, and the row's lesson stands unchanged — the session that implemented the clause found it by looking, four rounds before the instrument could. |

### The three findings

1. **§9.8.3.3, Issue #5 — the clause does not contradict itself; the 2020 printing does.** This
   file's neighbour in the ledger has said since it was written that §9.8.3.3 "contradicts itself,
   and the corpus's one witness proves it": an `/FD` dictionary's value "shall contain entries for
   metric information only; it shall not include FontFile , FontFile2 , FontFile3 , or any of the
   entries listed in ' Table 120 -Entries common to all font descriptors '", and every metric a
   descriptor can state is in Table 120 — so `issue13147.pdf`'s `/FD << /Proportional … >>`, holding
   ten Table 120 entries, breaks a rule it has no way to keep. The erratum makes three changes to
   that one sentence: it inserts "be a subset of the keys defined in "Table 120 - Entries common to
   all font descriptors" that", it softens "shall contain" to "contains", and it repoints the
   prohibition at "Table 122 - Additional font descriptor entries for CIDFonts". Read whole, the
   amended sentence says an `/FD` descriptor's keys are drawn *from* Table 120 and may not be
   Table 122's CIDFont-specific ones — `/Style`, `/Lang`, `/FD` itself, `/CIDSet` — which is
   consistent, checkable, and exactly what the corpus's one witness does. **No behaviour moves**:
   `/FD` is still read and applied to nothing for the licensing reason the row gives, and
   enforcement was and remains a validator's job. What moves is the reason, and a reason recorded
   as a defect in the standard is worth correcting when it turns out to be a defect in one printing
   of it. **Corrected in §9.8.3.3's note.**

2. **§12.3.2.4, Issue #162 and #288 — a named destination's `/SD` is unread, and §12.3.2.3's row
   claimed there was no such entry.** The clause's sentence about the `/Dests` dictionary's values
   ends "or a dictionary with a D entry whose value is such an array and may optionally contain an
   SD entry as defined in "Table 201 -Action types"" — and Table 201 is the list of action *types*,
   which defines no `/SD` at all. The erratum splits the sentence into bullets and replaces the
   dead reference: "In PDF 2.0, this dictionary may also optionally contain an SD entry. See "Table
   202 — Additional entries specific to a go-to action", "Table 203 - Additional entries specific to
   a remote go-to action" and "Table 204 - Additional entries specific to an embedded go-to
   action"." So the rule a named destination's `/SD` obeys is reachable from §12.3.2.4 for the first
   time, and it is Table 202's: "If present, the structure destination should take precedence over
   destination in the D entry."

   Two things follow. `Destination::of_go_to` applies that precedence and has since the
   four-hundred-and-eighty-fourth session; `Destination::read_within`, which is what reads a
   *named* destination's value, has a dictionary arm that takes `/D` and never looks for `/SD` — so
   a document stating both gets the weaker of the two, silently, and §12.3.2.3's algorithm is not
   reached although this tree has had it since that session. And §12.3.2.3's own row
   called Table 202's entry "the only entry in the standard that states a structure destination in
   *this* document", which was false when it was written: §12.3.2.4 stated a second one in 2020 as
   well, behind a reference that named nothing. **The erratum did not move the requirement; it made
   the requirement legible, and the false claim was found by reading it.** Both rows are corrected,
   and §12.3.2.4's carries the price: `read_within`'s dictionary arm needs `of_go_to`'s two lines
   with the same fallback argument — an `/SD` that does not resolve falls back to `/D`, because the
   precedence is a `should` and `/D` is the required entry — a test only the new step can answer, a
   count of the corpus named destinations that state one, and the row moved to `partial` until it
   lands.

3. **§9.6.2.2, Issue #384 — the fourteenth standard font's name has a hyphen in it, and this tree
   argued from a conversion artefact that it does not.** The erratum replaces the clause's inline
   list of the fourteen names with a bulleted one in alphabetical order, and the change is
   typographic — except that the erratum's own text spells the last of them `Courier-BoldOblique`.
   `doc/md/` writes `CourierBoldOblique`. The standard sets the name broken across a line, and the
   conversion drops the hyphen of a word it breaks — the failure `conformance::prose::folded`'s own
   doc comment describes with `text-tospeech` and `markedcontent`, here producing a **name** rather
   than a mangled word. A second extractor over the same page prints `Courier-` and `BoldOblique`
   on two lines, which is what the erratum's extraction says as well.

   `pdf_font::standard::STANDARD_NAMES` carries fifteen entries for the fourteen names, and its
   comment argues that the unhyphenated spelling "reads as the standard's own typography rather
   than as a distinct name". It reads as the conversion's. Nothing this program accepts changes —
   a resource named `/CourierBoldOblique` is still matched — but the warrant does: it is a
   documented tolerance for a producer who copied the same broken line, not one of the clause's
   own names. The blockquote of the retired inline list stays verbatim, because `doc/md/` is what
   `cargo test -p conformance` verifies against; what is owed is the annotation beside it and the
   corpus count that says whether any file writes the name at all. **Recorded in §9.6.2.2's note.**

   **This is a witness for `doc/todo/48`'s step 4** — the disagreement sweep, which compares this
   project's own extraction against `doc/md/` and has never been run — and it is the third of its
   kind after the four-hundred-and-seventy-fourth session's truncated `/OpenAction` row and shifted
   Table 179. The first two cost a suspect each; this one cost an argument in a doc comment and an
   entry in a constant.

### What the nineteen say about the rates

Sixty-six passages gave two findings, the next 54 gave one, and these nineteen gave three — the
highest rate this file records, and the reason is not that the passages are richer. Four of the
nineteen were **already read**, by one of `check`'s other halves or by a round that went looking
(#462, #386, #484, #236), so the nineteen are fifteen genuinely new readings. And **seven of the
fifteen are cross-reference corrections** — a wrong table number in the standard, repaired — which
is a shape this file had not met in quantity before: #85, #222, #479 twice and #187 three times.
Six of the seven are in clauses this tree implements, and each one is a place where a wrong pointer
in the standard meets a right one in the code, so each is cheap to read and says something about a
claim either way. Two of the three findings came out of that shape rather than out of a changed
`shall`: §12.3.2.4's `/SD` was invisible behind a reference to a table that defines no such entry,
and §9.8.3.3's contradiction was a prohibition pointing at the wrong table. **A struck pointer is
worth reading, and this is the evidence for it.**

## The addition a NOTE makes to §12.5.3, found in the six-hundred-and-fortieth

`emit` over all fourteen documents, before writing, on the annotation family — which is
`doc/todo/02` §4's rule and the one that keeps finding things `check` cannot. §12.5.6.4 carries no
annotation at all and neither does §12.5.5; §12.5.3 carries **Issue #34, `Review/Completed`**, and
one of its two halves had never been read here:

> When an appearance dictionary is not present, the rendered appearance will be implementation
> dependent.

A **pure addition** — a new NOTE 2 rather than a replacement — so `check` is blind to it by
construction, the same shape as Issue #293 in the six-hundred-and-thirty-first session. The row in
the table above records the *other* half of the same issue, the struck "without regard to any other
keys" that had `/BM` ignored on every stored appearance stream, and that is why this one went
unnoticed for two hundred and twenty sessions: an issue number already had a verdict.

**What it settles is a direction of inference rather than a behaviour.** Everything this tree
constructs for an annotation with no `/AP` — §12.5.6.4's seven icons, their size, §12.5.6.8's
inscribed rectangle, `crate::icon`'s whole artwork — is a place where `CLAUDE.md` principle 5 says
to make a documented choice because the standard states nothing. This sentence says so in the
standard's own words, for the whole population at once, which turns an inference from silence into
a citation. It is recorded on §12.5.3's ledger row, and §12.5.6.4's rests on it.

## Table 149's imperative becomes a `should`, found in the six-hundred-and-eighty-second

`emit` over all fourteen documents while re-deriving §12.3.2.2's negative — the same rule as the
section above, and the same shape of finding. §12.3.2.2 carries exactly one annotation, **Issue
#536, `Review/Accepted`**, and it had never been recorded here:

> interactive processors should

A **Caret with no `StrikeOut`**, so `check` is blind to it by construction, which is now the third
instance of that shape in this file after #293 and #34. Its `/Rect` is `[480.56 266.68 489.64
274.08]` on page 457, which `mutool draw -F stext` puts immediately before the word `use` in
Table 149's `/FitR` row — "If the required horizontal and vertical magnification factors are
different, use the smaller of the two". So the table's bare imperative becomes "…are different,
interactive processors should use the smaller of the two".

**Nothing here changes and the reading is still worth having.** `viewer_core::Open::apply_view`
takes the smaller of the two factors for `/Fit`, `/FitR` and `/FitB` already, so the amended
sentence is met; what moves is its *strength* — Table 149 stated the rule as an instruction with no
modal verb, and it now states it as a `should` addressed to an interactive processor, which is a
recommendation this tree follows rather than a requirement it meets. The distinction matters for
anybody arguing later about a window that would rather fit the other dimension. §12.3.2.2's row is
not rewritten for it, because the row quotes the null-parameter rule and the crop-box `shall`, and
this erratum touches neither.

## Two carets under §7.6.6, found in the six-hundred-and-ninety-first

`emit` over all fourteen documents while reading §7.6's `partial` rows, and both findings are the
same shape as the three above: a **`Caret` with no `StrikeOut`**, which `check` cannot see because
it has no struck text to compare a quotation against — the shape #293, #34 and #536 already are
above. It is worth stating as a rule rather than as a run of coincidences: **an erratum that only
*adds* is invisible to `check` by construction, and `emit` is the only instrument that reads it.**
The population is a command rather than a tally here, because this file records what a round read
rather than what the collection holds: `cargo run --release -p spec-errata -- emit doc/*.pdf`, and
a `Caret` printed with no `StrikeOut` above it under the same issue number is one of them.

**Issue #74, `Review/Completed`, licenses what this reader does at `/V` 5.** Its whole content is

> or 5

with `/Rect [215.245 388.253 224.21 395.557]` on page 105, which `mutool draw -F stext` puts
immediately after the `4` in §7.6.6's first bullet — "the value of the V entry shall be 4 to use
crypt filters". So the amended sentence is "shall be 4 or 5", and the reason it matters here is
that the 2020 text makes an AES-256 file's crypt filters non-conforming: Table 25 gives `AESV3` no
home other than a `/CF` entry, `/V` 5 is what Table 20 requires for it, and eleven of `doc/pdf.js`'s
twenty-five encrypted documents are exactly that. `crypt::crypt_filters` has read `/CF` at `/V` 4
**or greater** since it was written, on nobody's stated authority; it now has the clause's. Recorded
in §7.6.6's row and in the function's own comment.

**Issue #184, `Review/Completed`, settles a `/Length` this tree had disambiguated by arithmetic.**
Two carets on page 107, `/Rect [447.662 627.997 …]` and `[411.662 616.215 …]`, landing at the ends
of Table 25's last two sentences:

> for public-key security handlers, and 16 for the standard security handler

> for public-key security handlers, and 32 for the standard security handler

which amend "When CFM is AESV2 , the Length key shall have the value of 128" and the `AESV3`
sentence beside it. The table already said, higher up, that "The standard security handler
expresses the Length entry in bytes (e.g., 32 means a length of 256 bits) and public-key security
handlers express it as is", and those two closing sentences were unit-less and read as
contradicting it. `crypt::key_length` resolved that by Table 25's *range* — below 40 can only be
bytes, 40 or more can only be bits — and its comment called the entry "famously ambiguous". The
arithmetic is unchanged and still right; what was corrected is the comment, because the standard
now states the byte reading in the same two sentences that had seemed to deny it. The corpus's
witness is `doc/corpora/format-corpus/pdfCabinetOfHorrors/encryption_openpassword.pdf`, which
writes `/CFM /AESV2 /Length 16`.

## A one-word substitution under §9.4.2, found in the six-hundred-and-ninety-sixth

`emit` over ISO 32000-2 while re-deriving §9.4.2's negative, which is the same rule as the four
sections above and a different blind spot in the same sweep. **Issue #373, `Review/Completed`**, is
a `StrikeOut` **with** a `Caret` — so the shape `check` was built for — and `check` still cannot see
it, because what it strikes is one word:

> TD

replaced by

> Td

Its `/QuadPoints` are `[238.85 267.809 252.232 267.809 238.85 257.849 252.232 257.849]` on page 323,
and `pdftotext -bbox` puts a word `TD` at exactly `238.850`–`252.232`, `572.398`–`584.071` from the
top of an 841.92-point page — the same box, and it is the `TD` in **Table 106's `T*` row**, whose
description gives the operator as the code `0 –Tl TD`. So the table now reads `0 –Tl Td`.

**So the rule generalises past the additions.** The last four sections each found a `Caret` with no
`StrikeOut` and concluded that an erratum which only *adds* is invisible to `check` by construction.
This one strikes and replaces, and is invisible for a second reason: `check` compares struck
passages **of four words or more**, so every erratum that corrects a single token is below its floor
whatever its shape. `emit` is the instrument for both.

**It costs no arithmetic and it corrected two of this tree's own sentences.** `TD` sets the leading
to the negation of its own `ty`, so `0 –Tl TD` sets the leading to `Tl` — where it already was —
and the two codes move the line identically. What the erratum fixes is a reader. `run.rs`
implements `T*` as the amended form and always has, touching no leading; §9.4.2's ledger row and
`text_state.rs`'s `leading_moves_the_next_line_downwards` both quoted `0 -Tl TD`, and both now say
`Td` with the erratum named.

**Issue #372 on page 320 is not a finding and is recorded so that the next round need not look**:
its strikeout covers `Tm` and its caret says `Tm`, so what changed is the typography of one symbol.

**And Issue #191 is filed under §12.7.5.4 by `emit` and belongs to §12.7.5.3.** `emit` keys a note
to the clause heading its page carries, and page 556 carries the tail of §12.7.5.3 above the
§12.7.5.4 heading; the note strikes `The` for `An integer value greater than or equal to zero that
is the` at the head of **Table 232's `/MaxLen`** row, which is the text field's maximum length and
not a choice field's anything. Recorded in §12.7.5.3's row rather than acted on: the entry's
`integer` type already says as much, and `appearance.rs`'s `text_shape` takes the entry through
`u32::try_from` and then `filter(|value| *value > 0)`, so a `/MaxLen` at or below zero leaves the
field a single line rather than a comb of nonsense width. The amended sentence calls such a file
non-conforming and this reader already declines it, which is the erratum vindicating the code for
the second time in this section.
## Two carets under §11.6.6 and two mis-filed above it, found in the six-hundred-and-ninety-seventh

`emit` over all fourteen documents while reading §11.4's and §11.6's `partial` rows. Three
annotations land on the two pages §11.6.6 opens across, and all three are the shape the section
above states as a rule — a **`Caret` with no `StrikeOut`**, which `check` cannot see because it has
no struck text for a quotation to match. Two of the three are not §11.6.6's at all, and that is a
property of the instrument worth writing down before the finding.

### `emit` files an annotation by the *page* its outline puts in a clause, not by the clause

`Landing::section` is "the section §12.3.3's outline puts that page in", so every annotation on a
page is filed under the last bookmark whose destination is at or before it. §11.6.6's heading sits
at the **bottom** of page 436, and the top of that page is the tail of "Table 143 — Restrictions on
the entries in a soft-mask image dictionary", which is §11.6.5.2's. So the two **Issue #619**
carets on page 436 print under `## 11.6.6` and belong one subclause earlier. ADR 0492 read this
family's errata in the six-hundred-and-sixty-sixth session and recorded them as marking "entries of
§11.6.6"; they mark entries of Table 143. The check is arithmetic and takes a minute: a `/Rect` is
in PDF coordinates from the bottom, `mutool draw -F stext` reports from the top, and the page is
841.92 tall.

**Issue #619, `Review/Accepted`, adds a deprecation notice to Table 143's `/ID` and `/OPI` rows.**
The whole content of each caret is the four words `Deprecated in PDF 2.0.` — written as data rather
than as a quotation because `check` compares a quoted span against what an erratum *struck*, and
Issue #173 struck a sentence on p. 576 that opens with those same words, so quoting them here puts
a false positive in that instrument for nothing. Both carets say the same; their rects are
`[278.143127 687.482361 287.219238 694.877686]` and
`[278.143127 663.482361 287.219238 670.877686]`, which are 147.0 and 171.0 from the
top, and `stext` puts the `/ID` line at 144 and the `/OPI` line at 168 with `Ignored.` ending at
x 280 — so each caret sits at the end of its row's `Ignored.` **Nothing this tree does moves**: both
entries are ignored in a soft-mask image dictionary under either printing, and §8.9.5.1's row
already records them unread. What the erratum repairs is an inconsistency between two tables —
Table 87's `/OPI` row already carries the same notice and its `/ID` row does not, while §14.10.1
opens by saying that the features of Web capture — the feature `/ID` belongs to — are deprecated
with PDF 2.0. Table 143 states it for both entries now and Table 87 still states it for one.

### The finding: Issue #134 names which resource dictionary remaps a group's device space

**Issue #134, `Review/Completed`, states the authority for what this reader already did.** Its whole
content is

> of the transparency group XObject

with `/Rect [179.297 428.36 187.25 434.84]` on page 437 — 407.1 to 413.6 from the top, where `stext`
puts the line `dictionary (see 8.6.5.6, "Default colour spaces").` at 404.1 with `dictionary` ending
at x 181 and `(see` starting at x 184. The caret is between them, so Table 145's `/CS` sentence

> Device colour spaces shall be subject to remapping according to the DefaultGray , DefaultRGB ,
> and DefaultCMYK entries in the ColorSpace subdictionary of the current resource dictionary

becomes "…of the current resource dictionary **of the transparency group XObject** (see 8.6.5.6,
"Default colour spaces")".

That matters because *current* is the ambiguous word. A group's `/CS` is read at the `Do`, where the
resource dictionary in force is the **parent's** — the group's content stream has not started — so
the published sentence can be read either way, and the two readings pick different `/DefaultCMYK`
entries whenever a form states one its page does not. `content/xobject.rs` resolves a form's
`/Resources` and passes it as `form_resources`, falling back to the parent's only where the entry is
absent, and `content/transparency.rs`'s `press_for_entry` and `named_press` take that dictionary —
so this tree has read the group's own since the construction was built, on nobody's stated
authority. It now has the clause's. Recorded in §11.6.6's row and in `named_press`'s own comment.

## Two pairs the nesting rule gained under §14.6.1, found in the seven-hundred-and-first

`emit` over all fourteen documents while reading §14.6's `partial` rows. Six errata land on the two
pages §14.6.1 spans and four of them were already recorded here — #126's *rolemapped*, #303's
deleted NOTE 1, #334's deleted NOTE 3, #335's marked content inside a text object. **The two that
were recorded nowhere are #302 and #301**, and the first changes a requirement.

**Issue #302, `Review/Completed`, adds two pairs of operators to the properly-nested rule.** The
2020 sentence names three pairs — BMC…EMC, BDC…EMC and BT…ET — and the erratum's two carets insert,
into the list of operators being combined, the compatibility operators BX and EX and the graphics
state operators q and Q, and, into the parenthesis that enumerates the pairs, those same two. Its
whole strikeout is the single word *or*, one word under `check`'s four-word floor, so that
instrument is blind to it by construction: the seventh consecutive round in which a bare or nearly
bare caret has been the finding, and the second in which the caret licenses something this tree was
already doing.

**It is a licence rather than a debt.** §12.7.4.3 requires a processor to replace an appearance
stream's contents "from / Tx BMC to the matching EMC", which `appearance::spliced` does by counting
depth over `BMC`, `BDC` and `EMC` and cutting at the balancing `EMC`. Under the 2020 text a
conforming file could open a `q` before that `BMC` and close it inside the sequence, and the splice
would have removed the `Q` and left the rest of the page in the saved state; under the amended text
it cannot, because q…Q is now one of the pairs that shall be properly (separately) nested with a
marked-content sequence. The algorithm is unchanged and its warrant is not. Recorded in §14.6.1's
row, which is `implemented` on that reading and on the one requirement the clause puts on a reader.

**Issue #301, `Review/Completed`, is capitalisation**: Table 352's `BMC` row states its operand as
*Tag* where every other row of the table states *tag*. The tag is compared as bytes here — §7.3.5
makes a name's comparison "an exact binary match" — and the table's own prose is what names the
operand, so nothing moves.

## The two carets Table 259's `/Fields` row gained, found in the seven-hundred-and-fifth

`emit` over all fourteen documents while reading §12.8.3's fifteen `partial` and `reported` rows.
Five annotations land inside that subclause. Three of them ask nothing of a reader: Issue #4's
footnote marker on Table 260, an editor note about that table's inconsistent italics, and Issue
#649's placeholder NOTE 1 under §12.8.3.4.4, which exists so that the subclause's NOTE 2 and NOTE 3
keep the numbers they are printed with. **The other two are a requirement, and they are filed under
§12.8.3.1 while belonging one subclause back** — the attribution shape the six-hundred-and-ninety-
seventh session recorded above: `emit` files an annotation by the page the outline puts in a clause,
and §12.8.3's heading sits at the **foot** of page 592 while the top of that page is Table 259,
which is §12.8.2.4's.

**Issue #33, `Review/Completed`, requires the `FieldMDP` transform's field names to be fully
qualified.** Two carets, both an insertion with nothing struck out — the shape `check` cannot see —
with `/Rect [249.018 594.369 256.952 600.833]` and `[298.23 594.369 306.163 600.833]`, which are
241.1 to 247.6 from the top of an 841.92-tall page. `pdftotext -bbox` puts the line `containing
field names.` at yMin 235.4 and yMax 247.1 with `field` starting at x 253.0 and `names.` ending at
x 304.1, so the first caret is immediately before `field` and the second immediately after `names`.
Their contents are

> fully qualified

and

>  (see 12.7.4.2 "Field names")

so Table 259's `/Fields` row — "(Required if Action is Include or Exclude) An array of text strings
containing field names." — requires an array of text strings containing **fully qualified** field
names, with §12.7.4.2 named as where that term is defined.

**This tree has read it that way since `FieldSelection` was written, on an argument and a producer's
file rather than on the standard.** §12.8.2.4's row says so in as many words: the transform in
`xfa_filled_imm1344e.pdf` names `form1[0].SignatureField3[0]`, which `form::fields` independently
derives as §12.7.4.2's fully qualified name for that document's one field, "so the fully-qualified
reading in `FieldSelection::covers` has a producer's file behind it and not only an argument". It
has the clause's now, which is again what a bare caret has turned out to be worth: an erratum that
only inserts states the authority for something this reader was already doing, and the instrument
that would have found it is blind to insertions by construction.

**One half of it stays an argument, and the difference is worth keeping.** `FieldSelection` serves
two tables: §12.8.2.4's Table 259 and §12.7.5.5's Table 236, whose `/Fields` row is worded
identically. Only Table 259 gains the insertion — `emit` files eighteen annotations under
§12.7.5.5, over five pages and seven issue numbers, and not one of them touches that row — so the
same comparison is *required* for the FieldMDP transform and remains the reading argued for the
signature field lock. `covers`'s own comment says
which is which, because a function whose two callers stand on different footings should not let the
weaker one be forgotten.

## Table 166's `/ca` default, found in the seven-hundred-and-tenth

`emit` files seventeen annotation objects under §12.5.2, and four more under §12.5.3 that belong
to it — the page-straddle shape this file has recorded five times, and §12.5.2's ledger row has
had those four since the four-hundred-and-seventeenth session. **One of the seventeen is named
nowhere in this tree**, and it is the newest erratum any round here has met: **Issue #577**,
`D:20260521104207-05'00'`, `/State` `Review`/`Accepted`.

It is a `StrikeOut` over `1.0 ` and a `Caret` saying `the value of CA`, at
`[263.930 285.359 278.601 297.032]` and `[274.063 283.872 283.139 291.268]` on physical page 484.
`pdftotext -bbox` puts `Default value: 1.0` at 544.9–556.6 from the top of an 841.92-tall page,
which is 841.92 − 297.032 = 544.888 — the strikeout's own top edge, to three decimal places — and
the three lines above it read "nonstroking operations on all visible elements of the annotation in
its closed state (including its background and border) but not the popup window that appears when
the annotation is opened." So the row is Table 166's **`/ca`**, not the `/CA` two rows below it,
and the amended default is *the value of `CA`*.

**A four-word floor is not why `check` missed this one; there is nothing to match.** The struck
text is a bare `1.0`, so even a tree that quoted the whole `/ca` row verbatim would share no
sentence with it. That is the third distinct way `check` is blind, beside a caret with no
strikeout and a strikeout under the floor: **a strikeout whose text is a *value*.**

**The erratum vindicates the code, and does it by settling one row with another's sentence.**
`annotation::construct` reads `/ca`, falls back to `/CA`, and falls back to 1.0 — written on
Table 166's `/CA` row, which says "If a ca entry is not present in this dictionary, then the value
of this CA entry shall also be used for nonstroking operations as well". Under the 2020 printing
the `/ca` row's own "Default value: 1.0" said the opposite, and this tree had picked the
neighbouring sentence over the nearer one for its whole life. The amended `/ca` row now says what
the `/CA` row says. No arithmetic moves; the authority does, which is what a bare caret and a bare
strikeout have both turned out to be worth. Recorded in §12.5.2's note.

## Table 179's fifth fill, and a word the ISO PDF's own row height hides — the seven-hundred-and-sixteenth

`emit` over the pages §12.5.6.7's Table 179 spans files three annotation objects this tree named
nowhere, and the first of them is the one that matters.

**Issue #515**, `/State` `Review`/`Completed`, `D:20260521100754-05'00'`, on physical page 504: a
`Caret` whose `/Contents` is `filled with the annotation's interior colour, if any.` at
`[338.883 320.112 347.959 327.508]`. There is **no** `StrikeOut` beside it, so this is the first of
`check`'s three blindnesses rather than the third — nothing was struck, so there is nothing to
compare a quotation against.

The row it lands on is settled by arithmetic rather than by eye. The page is 841.92 tall, so the
caret occupies 514.41–521.81 from the top, and `pdftotext -bbox` puts exactly one line there:
`from ClosedArrow` at 508.65–520.32, which is the second line of `RClosedArrow`'s description —
`(PDF 1.5) A triangular closed arrowhead in the reverse direction from ClosedArrow`. The caret's
left edge, 338.883, is two points past that line's last word, which ends at 341.06. So the amended
row reads *…in the reverse direction from ClosedArrow, filled with the annotation's interior
colour, if any.*

**What it settles is a count this tree held in four places and got right in only two of them.**
Table 179 as published names the fill on `Square`, `Circle`, `Diamond` and `ClosedArrow`, and this
crate has always filled a fifth — `RClosedArrow` — on the reading that a shape drawn "in the
reverse direction from" a filled one is the same shape. The reading was right and the erratum now
states it. What was wrong was the prose beside it: `Ending::filled`'s doc comment said "Four of the
ten say so and the other six do not" over a `matches!` with **five** arms, and the test guarding it
was named `only_the_four_endings_table_179_fills_use_the_interior_colour` over a loop of **five**
names. §12.5.6.6's ledger row carried the same four. No pixel moves; four sentences do.

**And the same run explains a sweep hit that had been standing unread.** **Issue #513**,
`/State` `Review`/`Accepted`, on the same page, is an EDITOR NOTE rather than a change:

> EDITOR NOTE: (Issue #513) The row height in the ISO PDF file obscures the end of the sentence.
> The text is unchanged but noted here for clarity.

The annotation then reprints `OpenArrow`'s whole description, ending in the word `arrowhead`. It is
**not** quoted again here, and that is deliberate: the sentence is one `doc/md/` cannot supply, so a
copy of it in this file would be a sixth diverging span for `--bin quotations` to print and a reader
to re-diagnose. `doc/adr/0192` carries the quotation, with the explanation beside it.

`doc/md/` carries the damage the note describes. Its Table 179 ends `OpenArrow`'s cell at "Two
short lines meeting in an acute angle to form an open" and begins `ClosedArrow`'s with the word
that finishes it. `--bin quotations` has therefore printed `doc/adr/0192`'s copy of the sentence as
a diverging document span, at the head of its own output — and the ADR is right, the conversion is
short, and the standard says so itself. This is the sweep's own instruction — *suspect the
conversion before the document* — answered for once by the specification rather than by a reader.
The ADR keeps its quotation and now says why the sweep prints it.

**Issue #524** is the third, and it moves nothing here. A one-word `StrikeOut` over `rectangle`
with a `Caret` saying `array`, in the **type** column of `/RD` — struck on physical page 500
(Table 177, free text), 505 (Table 180, square and circle) and 508 (Table 187, caret), which is
three of the four tables that state the entry. `/RD` is four differences rather than a pair of
corners, so `array` is the right type name and `rectangle` was not. A one-word strike is under
`check`'s four-word floor, which is the second blindness; nothing in this tree calls `/RD` a
rectangle, and `appearance::differences` reads four numbers in the clause's own order of left,
top, right and bottom. **Establishing that is worth recording**: the entry is read by three
subtypes and a wrong type name in the table is exactly the sort of thing a reader copies.

## Table 161's letters, where two accepted errata cannot both be applied — the seven-hundred-and-twentieth

`emit` over the pages §12.4.4 spans files nothing new against that clause — **Issues #36 and #75
and no others**, both already recorded above, which is the answer the round wanted and is worth as
much as a finding. What it did file, two pages earlier, is a pair this collection had never named,
and they are about the same sentence.

Table 161's `/S` row gives the alphabetic styles as *A Uppercase letters (A to Z for the first 26
pages, AA to ZZ for the next 26, and so on)* and the same for lowercase. Two annotations amend it,
in opposite directions:

- **Issue #432**, `Review`/`Accepted`, `D:20240617182323+10'00'`: a `StrikeOut` at
  `[444.54664 540.3789 455.29353 552.052]` with a `Caret` saying `AZ` beside it, and the same pair
  at `[440.3896 513.9789 449.4532 525.652]` saying `az` one row down. The amended sentence is *AA
  to **AZ** for the next 26* — the **odometer**, where the twenty-eighth page is `AB`.
- **Issue #593**, `Review`/`Accepted`, `D:20260521092610-05'00'`: a `Caret` at
  `[519.7684 538.8924 528.8445 546.2877]` with **no** `StrikeOut`, saying `AAA to ZZZ for the next
  26, AAAA to ZZZZ for the next 26,`, and its lowercase twin at `[513.94796 512.4924 523.02407
  519.8877]`. That is the **repeat**, where the twenty-eighth page is `BB`.

**The placement is arithmetic rather than opinion.** The page is 841.92 tall, so #432's strike
occupies 289.868–301.541 from the top and 444.547–455.294 across; `pdftotext -bbox` on physical
page 474 puts the word `ZZ` at exactly `xMin="444.546640" yMin="289.868000" xMax="455.293500"
yMax="301.541120"`. It is that word and no other. #593's caret shares the line's y and sits at
x 519.77, just past `26,` which ends at 522.115 — an insertion after the clause #432 rewrites.

They are mutually exclusive: with #432 applied the sentence enumerates an odometer, and #593's
addition of *AAA to ZZZ for the next 26* is then false, since an odometer's `AAA` to `ZZZ` is
17 576 labels rather than 26.

**This reader keeps the repeat, and not because the newer erratum says so.** The published
sentence carries its own count — *for the next 26* — and `AA` to `ZZ` is 26 labels only if the
letter repeats. `page_label::letters` has produced `A…Z, AA…ZZ, AAA…` since it was written, on that
arithmetic, and `letters_repeat_rather_than_carrying` pins `BB` at 28 against base 26's `AB`.
#593 states the reading outright, which is a stronger form of the same answer; #432 denies it.
Nothing changes but what is written down, and what is written down is the *disagreement* — a claim
about the specification that a future edition may settle either way. §12.4.2's ledger row and
`letters`'s own doc comment both carry it now.

**A third property of `check` is on display and it is the first blindness twice over**: #593 is a
`Caret` with no `StrikeOut`, so there is no retired text for a quotation to fail to match, and
#432's strike is one word, under the four-word floor. Neither could have been printed by anything
but `emit`.
