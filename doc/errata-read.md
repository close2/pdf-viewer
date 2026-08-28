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
| §7.7.3.3 Page objects | 121 | #106, #619 | cites | "; indirect reference preferred" struck from Table 31's `/ID` by #106, and "; Deprecated in PDF 2.0" inserted into the same parenthetical by #619 — the two compose rather than collide, leaving "(Optional; PDF 1.3; Deprecated in PDF 2.0)". `/ID` is unread here under either printing. |
| §7.9.4 Dates | 133 | #251 | quotes `[416]` | Two sentences retired; `Date::instant` was right for the surviving reason. |
| §7.12.3 Developer extensions | 157 | #732 | quotes | `/URL` becomes **Required**. The ledger called it optional — corrected. `DeveloperExtension::read` still answers an extension without one, now as a stated choice. |
| §7.12.6 URL | 158 | #399 | cites | The `/BaseVersion` version comparison, which this reader does not perform. |
| §8.6.5.5 ICCBased | 207 | #181 | cites | ×3. The PDF-version→ICC-specification table becomes an ICC *profile header* version table, "both 2.x and 4.x". Nothing here ties a profile to a PDF version; `icc.rs` reads the header byte and accepts both. |
| §8.6.5.8 Rendering intents | 212 | #63 | cites | The NOTE's licence to support fewer than four intents is withdrawn. Nothing here leant on it; the existing `partial` gap is unchanged and slightly less excusable. |
| §8.6.8 Colour operators | 232 | #551 | cites | Table 74's `CS` row no longer says the resource entry "shall be an array"; `colour::parse_at` already accepted a name first. |
| §8.9.5.1 General | 274 | #366 | untouched | "if a predictor function is used" struck from Table 87's `/BitsPerComponent`. Predictors never reach the image dictionary here. |
| §8.9.5.1 General | 275 | #619 | untouched | "; Deprecated in PDF 2.0" inserted into Table 87's `/ID` row, which is the caret the section below had filed as Table 143's alone. `/ID` is unread here under either printing. |
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
| §14.5 Page-piece | 734 | #691 | **cites — and this verdict judged the wrong clause's row until the seven-hundred-and-seventy-fourth session.** | "such as MD5 (described in Internet RFC 1321)" struck. This row read "struck from a NOTE about detecting a changed page. The row is `inapplicable` and names no digest" — and the strike is not in a NOTE, not about detecting a changed page, and not §14.5's: it is **§14.4's** uniqueness paragraph, a `should` addressed to PDF writers, filed under §14.5 because page 734 opens §14.4 and `emit` attributes by the outline section for the page. §14.4's row is `implemented` over a writer — `write.rs::identify` — that names MD5, now this project's stated choice rather than the standard's named example. The ninth use's section below has the placement. |
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
with PDF 2.0.

**That paragraph ended "Table 143 states it for both entries now and Table 87 still states it for
one", and the seven-hundred-and-thirtieth session found the erratum repairs Table 87 as well.**
Issue #619 carries **four** carets and the two on page 436 are half of it: one sits on page 275 in
Table 87's own `/ID` row and one on page 121 in Table 31's, so the issue deprecates `/ID` wherever
the three tables state it and leaves nothing uneven behind. The arithmetic is this section's own and
took a minute apiece. Page 275's rect is `[390.62046 176.32236 399.6965 183.71765]`, which is 658.2
to 665.6 from the top of an 841.92-tall page, where `stext` puts the `/ID` row's value cell at 654.9
and `pdftotext -bbox` ends the word `preferred)` at x 399.2 — the caret's centre at x 395.2 is one
glyph short of that, which is the insertion point before the closing parenthesis. Page 121's is
`[284.3599 256.12236 293.43595 263.51765]`, 578.4 to 585.8 from the top, on the cell `stext` puts at
575.1, with `1.3;` ending at x 291.5 and its centre at 288.9 landing before that row's semicolon.

**The lesson is about the instrument rather than about `/ID`.** `emit` files an annotation by the
page it is on, so an issue whose carets are scattered across three tables prints as three separate
entries hundreds of lines apart, and a round reading one page reads a third of the issue. The
correction it left standing here was not stale — it was **wrong when written**, from facts that were
right, because the reasoning stopped at the page in front of it. A round recording what an erratum
does asks `emit` for the *issue number* across the whole document before it concludes what the issue
leaves alone. ADR 0621.

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

## The two §12.5.2 left unread, found by asking which errata this tree names nowhere — the seven-hundred-and-thirty-fourth

`emit` files **seventeen** annotation objects under §12.5.2, and the seven-hundred-and-tenth
session counted them and wrote that one of the seventeen is named nowhere in this tree. The
seventeen carry five issue numbers — **#1, #22, #124, #287 and #577** — and **three** of the five
were named nowhere. That round recorded #577; these are the other two.

This is not the page-straddle shape and it is not a floor. It is what ADR 0627's ranking is built
on: the question *which issue numbers does this tree name* has an answer a command can print, and
nobody had asked it. Reading a clause's pages is not the same as reading its errata.

**Issue #1**, `/State` `Review`/`Completed`, `D:20220615120000+10'00'`, on physical page 482 — so
`emit` files it under §12.5.2 and the sentence it changes is **§12.5.1's last**. A `StrikeOut`
whose quadrilaterals are `[519.840 658.849 553.643 647.809]` and
`[72.024 644.089 191.919 633.049]`, with a `Caret` at `[187.448 632.442 196.390 639.728]` saying
`12.5.5, "Appearance streams" and "Table 167 - Annotation flags" (bit positions 1 and 2)`.

The placement is arithmetic. The page is 841.92 tall, so the strike's two runs occupy 183.07–194.11
and 197.83–208.87 from the top; `pdftotext -bbox` on physical page 482 puts `12.5.2,` at
`xMin="522.464" xMax="553.643"` on the line at `yMin="181.174" yMax="194.113"` and
`"Annotation dictionaries".` at `xMin="72.024" xMax="194.190"` on the line below. The strike takes
both and stops two points short of the closing full stop; the caret sits at the end of the second
run. The sentence it amends is *An interactive PDF processor shall provide certain expected
behaviour for all annotation types that it does not recognise, as documented in* — and what
follows it is now §12.5.5 and Table 167's first two flags rather than §12.5.2.

**It licenses what this reader does and it found the half nothing read.** §12.5.5's own sentence —
"[i]f a PDF processor does not have native support for a particular annotation type, the PDF
processor shall render the annotation with its normal (N) appearance" — has a test. Table 167's
`Invisible` row has two sentences and only the first had a reader: the flag *set* suppresses an
annotation outside Table 171, which `annotation::decided` does; the flag *clear* asks for the
appearance stream "if any", and an unknown subtype with no appearance dictionary was reported as
unsupported, with the detail *its clause states no geometry* — a claim about a clause a subtype
outside the table does not have. Recorded in §12.5.1's and §12.5.3's rows; ADR 0628 has the reading
and `examples/unknown_subtype_census` the population.

**Issue #124**, `/State` `Review`/`Completed`, `D:20220615120000+10'00'`, nine annotation objects on
physical page 483 — four `StrikeOut`/`Caret` pairs and one lone `Caret`. The pairs strike `1`, `3`,
`2` and `4` and write `0`, `2`, `1` and `3`; the lone caret at `[506.354 535.402 513.708 541.394]`
says `Array indices were also corrected to be zero-based as described in 3.2 "array object".`

The four strikes are Table 166's `/AP` bullet, and their quadrilaterals say so exactly:
`[255.594 584.659 261.112 574.699]`, `[386.618 …]`, `[490.199 …]` and `[308.103 572.899 313.621
562.939]`. `pdftotext -bbox` on that page puts `1` at `xMin="255.594" xMax="261.112"`, `3` at
`386.618–392.136` and `2` at `490.199–495.717`, all on the line `yMin="255.554" yMax="267.221"`,
and `4.` at `308.103–315.771` on the line below — the strike stopping before the full stop. Four
digits, in the order the bullet writes them.

**It moves no rectangle**, and that is worth writing down rather than leaving to be re-derived: on
`[x1 y1 x2 y2]` the one-based pairs 1↔3 and 2↔4 and the zero-based pairs 0↔2 and 1↔3 are the same
two comparisons, x against x and y against y. What reading the bullet did settle is a justification
this tree had written twice — the bullet is an **and**, its own NOTE saying "[t]he bullet point
above was changed from 'or' to 'and' in this document to match requirements in other published ISO
PDF standards (such as PDF/A)", while `annotation::is_empty` is an **or**. Two comments claimed
Table 166 excused a writer "for exactly that shape"; the excuse is the degenerate point alone.
Nothing drawn moves — §12.5.5's scale onto no extent leaves no mark either way — and §12.5.2's row
carries the corrected reason.

**Neither could have been printed by `check`.** #1's strike is four words including a clause
number, over text no quotation in this tree carries; #124's four strikes are single digits, which
is the third blindness the seven-hundred-and-tenth named — a strikeout whose text is a *value* —
four times over in one erratum.

**And there is a fourth way an issue number goes unnoticed, which is this tree's rather than the
instrument's.** A numeric character reference in Markdown is a `#` and digits: `&#124;` is how a
table cell escapes a pipe, and it is the only one anywhere under `crates/`, `doc/` or `tools/` —
two occurrences, in ADR 0484's comparison table. A search for `#124` finds them. So the one issue
number in the collection that a plain grep cannot answer honestly is exactly the one that went
unrecorded on a page a round had opened. The rule that comes out of it is `doc/todo/01`'s:
**ask whether an issue is recorded with the `Issue #` prefix, not with the number alone.**

## The five §12.8.1 was holding, and the entry that says which part of a validation matters — the seven-hundred-and-thirty-ninth

The successor rule's second use. `doc/todo/01`'s recipe ranks a live ledger row by the errata
annotations on it whose issue number this tree names nowhere, and at this base §12.8.1 is the head:
nine annotations carrying **five** issue numbers, none recorded here. They are read whole below, and
one of them reaches across two clause headings. ADR 0637.

Every placement is the caret's or strikeout's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on a page 841.92 points tall; the rectangles are quoted with
`y` measured from the top, which is the orientation `mutool run`'s `getBounds` prints and the one
`-bbox` uses.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §12.8.1 Table 256 `/DigestMethod` | 587 | #117 | **quotes** | `(Required)` struck at `[241.76 602.05 285.17 613.25]`, over the word `-bbox` puts at `(241.0, 601.0)–(286.0, 612.6)` on the `DigestMethod` row of Table 256, and a caret at `[278.17 606.60 286.23 613.18]` writing **Optional; deprecated in PDF 2.0** in its place. §12.8.1's ledger row quoted the retired `(Required)` as the opening of that entry. Corrected. |
| §12.8.1 Table 255 `/Filter` | 584 | #121 | **cites** | `; inheritable` struck at `[251.65 199.75 302.22 209.71]`, over `(Required;` `inheritable)` on Table 255's `/Filter` row, leaving `(Required)`. **The erratum vindicates the code**: `signature::read` asks `Document::get_key`, which resolves an indirect reference and inherits nothing, so `/Filter` has never been looked for up a field's `/Parent` chain — and until this erratum the table said it should be. Inheritance in this standard is §7.7.3.4's four page attributes and §12.7.4.1's field entries; a signature dictionary is neither. |
| §12.8.1 Table 255 `/SubFilter`, §7.6.5.2 Table 20 `/SubFilter` | 584, 102 | #219 | untouched | ×7, and this is the issue that had to be reassembled. Four annotations under §12.8.1 and three under §7.6.5.2, all of them PDF version markers on `/SubFilter` values. Table 255 loses its blanket `(PDF 1.6)` (struck at `[210.41 379.18 252.29 389.14]`) and gains `(PDF 1.3)` after `adbe.x509.rsa_sha1`, `(PDF 1.3)` after `adbe.pkcs7.detached` and `(PDF 1.4)` after `adbe.pkcs7.sha1`; Table 20 gains `(PDF 1.3)`, `(PDF 1.4)` and `(PDF 1.5)` after `adbe.pkcs7.s3`, `s4` and `s5`. Each caret's centre falls within a point or two of its value's last glyph. Nothing here compares a value against a version: `Signature::must_cover_whole_file` matches the two ETSI names and `crypt.rs` refuses a public-key handler before Table 20 is read. |
| §12.8.5 (filed under §12.8.1) | 583 | #55 | cites | A lone caret at `[350.78 726.03 359.68 733.28]`, whose line `-bbox` puts at `y 720–731` — the **document timestamp** bullet's last line, `value of a signature field and shall contain a ByteRange entry.` — inserting " These shall follow the certification signature if one is present." The identical sentence already stands in the *approval signature* bullet 80 points above, at `y 619–644`, so the erratum extends an ordering rule to a third kind of signature rather than repeating one. It is a rule about how a file is put together and this program builds no file; what a validator could do with it is report a timestamp that precedes a certification signature, and §12.8.5's row records neither a `/DocTimeStamp` in the corpus nor a reason to rank one. |
| §12.7.9 (filed under §12.8.1) | 582 | #54 | untouched | A one-character strike at `[430.63 235.90 436.75 246.94]`, over the `0` in `attrib0ute` — §12.7.9's last sentence, "Non-interactive forms are defined by the PrintField attrib0ute". `doc/md/` carries the typo and nothing in this tree quotes the sentence. Filed under §12.8.1 because page 582's outline entry is §12.8.1, which is the page-straddle this file has recorded since its second table. |

### What reading them made this round look at, which is the point of the rule

**§12.8.1's row said "Table 255 entire" and named thirteen of the table's eighteen entries.** #121
is on the `/Filter` row, so the table had to be read against the row, and five entries turned out to
have no reader at all: `/R`, `/V`, `/Prop_Build`, `/Prop_AuthTime` and `/Prop_AuthType`. Four of the
five say nothing to a reader and are declined in the row with the entries' own words. The fifth was
work:

> The value is 1 if the Reference dictionary shall be considered critical to the validation of the
> signature.

That is Table 255's `/V`, and it is the one sentence of the entry addressed to whoever *validates*.
This program evaluates no transform method — §12.8.2.2.2's comparison of two revisions is what that
would take — so a file writing `/V 1` is naming the part of its own validation that this program
skips, and nothing here read the entry. `Signature::format_version` reads it,
`Signature::reference_is_critical` applies the entry's own condition, and `viewer_core::notes` says
it beside the paragraph that names the questions which went unanswered. The population is the
crawl's and `examples/signature_algorithm_census` is the command: no curated document states the
entry at all, and the two `CC-MAIN-2021-31` files that write `/V 1` each carry a `DocMDP` and a
`FieldMDP` reference dictionary, which is exactly the material the sentence is about.

**#117's strike is one word, which is why no gate could see it.** `spec-errata check` filters
strikeouts below four words, so a quotation resting on a retired `(Required)` reads as a live one —
the third blindness this file lists, met in the one population that has a gate. The correction is
more than the words: an entry that was *required* and unread is a debt, and an entry PDF 2.0
deprecates and makes optional is one a reader meets on older files alone. The strike also leaves
the NOTE beneath it — "The DigestMethod key was also corrected to be required as no default value
is defined" — reporting a correction the erratum has since undone. A NOTE is informative and a
table cell is not, so the cell decides; this is the second place in this file where the collection
does not agree with the document it amends, after Table 161's letters.

### And a correction to the rule's own second step

The recipe's grep asks for the `Issue #` prefix, for the `&#124;` reason the round before recorded.
**It under-counts what this tree names, and the shortfall is in this file.** Every row of the tables
above writes its issue number bare, in a column — `#680`, `#158`, `#685` — so the prefixed grep
answers *nowhere* for issues that carry a verdict here. Measured at this base: the prefixed grep
finds 113 of the 351 issues that carry an annotation, and this file alone records 159.

**A bare-number grep is not the repair**, and the reason is a second collision family the round
before did not have: `doc/HAYRO_ISSUES.md` and `doc/HAYRO_ISSUES_FOR_QUORRA.md` are lists of
*another project's* GitHub issues, and they name `#54`, `#55`, `#680` and `#681` — four of the five
read above. A grep for the number alone answers "recorded" on every one of them, from a document
about a different tracker. So the population is bounded on both sides and `doc/todo/01` now says how
to take it: the prefixed grep, plus this file's own numbers, minus the numeric character references.

## The four on Table 269, and the matrix a published table only counted — the seven-hundred-and-forty-sixth

The successor rule's third use. Run as `doc/todo/01` now writes it — the prefixed grep unioned with
this file's own bare column — three rows tie for the head at seven annotations apiece: §7.7.4,
§12.10.2 and §14.8.5.3. §12.10.2 was taken, and the tie-break is stated in `doc/todo/01`: the other
two rows' errata substitute words, and this one's change a requirement level and an entry's meaning.
ADR 0653.

Every placement is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on a page 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §12.10.2 Table 269 `/PCSM` | 618 | #534 | **implements** | The one that moved the row. A two-run strike at `[429.68 680.52 520.95 692.19]` and `[187.58 692.28 222.03 703.95]` takes `projected coordinate` off the end of one line and `system.` off the start of the next, and a caret at `[217.49 698.04 226.57 705.44]` writes *the projected coordinate system. This array represents a 4x4 affine transformation matrix in row order. The XObject position coordinates are represented as a 1x4 matrix, [ x y z 1 ], where the z value is non-zero only in the context of a Geospatial3D-enabled annotation.  PCSM only applies when GCS is a projected coordinate system.* The published table says how many numbers `/PCSM` holds and nothing about what they mean; this says what they are. Its last sentence is `Geospatial::matrix_has_priority` already, written from the published table's contrapositive — a second erratum vindicating code rather than correcting it. |
| §12.10.2 Table 269 `/LPTS` | 618 | #533 | cites | `Optional` struck at `[191.29 550.89 227.26 562.68]`, over the word `-bbox` puts at `187.58–229.84` as `(Optional;` on the `/LPTS` row, with a caret at `[222.72 556.65 231.80 564.05]` writing *Required*. Nothing here turns on the level: the entry's own description — the array "shall contain the same number of number pairs as the GPTS array" — is what `Geospatial::registration` reads, and a `/GPTS` with no `/LPTS` states coordinates with nothing to attach them to whatever the cell says. Recorded on the field. |
| §12.10.2 Table 269 `/PCSM` | 618 | #358 | untouched | `real` struck at `[437.79 670.47 454.39 680.43]`, in `A 12-element transformation matrix of real numbers`. The same issue strikes `real` on page 849 at `[337.63 235.30 354.23 245.26]`, in the OPI `Inks` entry's "each tint is a real number in the range 0.0 to 1.0" — filed under §14.12.1 by the page-straddle this file has recorded since its second table. Editorial: §7.3.3 calls the thing a number. |
| §12.10.2 Table 269 `/GPTS`, `/LPTS` | 618 | #284 | **declined** | ×2, and both are one word of insertion. A caret at `[494.04 479.13 503.12 486.53]` sits at the end of `/GPTS`'s `… (requirement type Geospatial3D) in a 3D` and one at `[312.43 627.12 321.51 634.52]` between `3D` and `annotation` on `/LPTS`'s line, each writing *or RichMedia*. The condition they widen is the one that makes those arrays triples rather than pairs — and Table 333, a RichMedia annotation's, states no `/GEO` entry until the same erratum adds one. `measurement.rs` reaches a geospatial measure dictionary through §12.9's `/VP` alone, so the erratum widens clause 13's excluded population and not this one. |

### What reading them made this round look at, which is the point of the rule

**§12.10's row said the transformation needs the EPSG registry, and that is two legs of a journey
charged to the second one.** The sentence was "[t]urning a page coordinate into a latitude means a
geodesy library and that registry", and the same claim stood in `Geospatial`'s own doc comment.
Table 269 gives `/PCSM` priority over `/GPTS` where a file states one, and #534 says what its twelve
numbers are — so on such a file the leg from the object's coordinates to the projected system is a
matrix multiplication with nothing outside the standard in it. What the registry owns is
projected-to-geographic alone.

**Twelve numbers for a 4×4 matrix is §8.3.4's own convention one dimension up**, which is why the
erratum is enough to implement and not merely enough to quote. There a point is "expressed in vector
form as [ x y 1]", the matrix is 3-by-3, and "[b]ecause a transformation matrix has only six elements
that can be changed, in most cases in PDF it shall be specified as the six-element array [a b c d e
f]" — three rows, their third column elided as 0, 0, 1. Four rows with a last column of 0, 0, 0, 1
leave twelve, and the erratum's "row order" and "1x4 matrix, [ x y z 1 ]" say the vector is on the
left, exactly as §8.3.4's is. `Geospatial::projected_position` is that multiplication and no corpus
document states a `/PCSM` to check it against, so its fixture is hand-built and its matrix is
asymmetric on purpose (trap 8, trap 13).

**And the entry condition the row had no reader for is `Geospatial3D`'s triples.** Table 269 requires
`/GPTS` and `/LPTS` "to hold 3D point coordinates as triples rather than pairwise" under that
requirement type, and `pairs` chunks by two unconditionally under a row claiming "Table 269 entire".
It is declined rather than owed, on where the dictionary sits: Table 309's `/GEO` puts it "within a
3D Annotation" and #284 puts the same entry in Table 333, which is a RichMedia annotation's and states none, both of which are
clause 13's and both excluded. The erratum that could have opened it is the one that closes it.

## §7.7.4's rename and its two deprecations, and where the rename really landed — the seven-hundred-and-fiftieth

The successor rule's fourth use. §12.10.2 is gone from the ranking — the round before read it — and
what is left at the head is the plateau it left behind: §7.7.4 and §14.8.5.3, seven annotations
apiece, the two rows the tie-break lost to §12.10.2. §7.7.4 wins the second round of that same
tie-break, because Issue #672 changes a requirement level in a cell and §14.8.5.3's four `Caret`s
swap *version* for *level* in the name of a referenced CSS specification. ADR 0660.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §7.7.4 Table 32, ten rows | 124, 125 | #214 | **corrects six places** | Not a table's erratum but the whole standard's, stated once as a `Text` note on page 10: *as a result of Errata #214, all occurrences of the term "name string" are replaced by just "string" throughout ISO 32000-2:2020*. Table 32 carries ten illustrative strikes over the words `name `, five per page — `[425.259 163.049 451.035 173.009]` and four more at the same width down page 124, each over the `name strings` that `-bbox` puts at x 423.6–449.4 on the `/Dests`, `/AP`, `/JavaScript`, `/Pages` and `/Templates` rows, and five more on page 125's `/EmbeddedFiles`, `/AlternatePresentations` and `/Renditions`. **The term is not one §7.9.2 defines**: that clause states a text string, a PDFDocEncoded string and a byte string, and nothing else — so the erratum withdraws a type ISO 32000-2 never had. What it corrects here is quotation rather than behaviour, and the *scope* is the finding: see below. |
| §7.7.4 Table 32 `/IDS`, `/URLS` | 124 | #672 | **cites** | Two bare `Caret`s, `Review/Accepted`, at `[319.3999 114.04235 328.47596 121.43766]` and `[321.03334 75.26235 330.10939 82.65766]`, each landing on the `1.3)` that `-bbox` puts at the end of `(Optional; PDF 1.3)` on its row, and each writing *; deprecated in PDF 2.0*. Both trees are Web Capture's (§14.10.4's content sets). §7.7.4's row had them as two of six trees *owed to a feature*; they are deprecated, which is the reason `/AlternatePresentations` already carried in the published table, so the row's debt is three trees rather than six. |
| §7.7.2 Table 29 `/SpiderInfo` | 116 | #672 | **cites** | The same issue's third caret, `[318.3199 510.2123 327.39595 517.6076]`, on the `1.3)` of the `SpiderInfo` row that `-bbox` puts at x 310.29–326.79. §7.7.2's row lists that entry under "[g]enuinely unread, and each for a reason", and the reason is now the one the row already gives `/NeedsRendering`. **All three of the catalogue's Web Capture entries are deprecated by one erratum**, which is what reassembling an issue across its clause headings is for. |
| §7.9.6 sorting, Table 36 `/Names` | 135 | #214 | **corrects** | The same rename, reaching a sentence about order rather than about type. A strike at `[415.088 433.799 456.091 444.839]` takes ` lexically` out of "arranged in key-value pairs and shall be sorted lexically in ascending order by key", and one at `[400.965 579.139 467.059 589.099]` takes ` in lexical order` out of Table 36's "The keys shall be sorted in lexical order, as described below." What is left defines the order and the struck word only named it: "Shorter keys shall appear before longer ones beginning with the same byte sequence. Any encoding of the keys may be used as long as it is self-consistent; keys shall be compared for equality on a simple byte-by-byte basis." That is `<[u8] as Ord>`, which `TreeKey::compare` has always been. `named_page.rs` said "which §7.9.6 makes lexical by key" and is prose about bytes now. |
| §7.9.6 Table 36 `/Names`; §7.9.7 Table 37 `/Nums` | 135, 137 | #307 | **tested** | ×2, and **the fifth `Caret` with no `StrikeOut` this file has recorded**, after #293, #34, #536 and #154 — nothing is retired, so `check` has no struck text to land a quotation on. `[231.006 567.124 238.959 573.604]` sits on the `below.` that ends Table 36's `/Names` cell and `[217.915 105.668 225.868 112.148]` on the `trees".` that ends Table 37's `/Nums` cell, each writing *. Keys shall not be the null object.* One sentence, stated once per clause, which is §7.9.7 defining itself as §7.9.6 with integer keys showing through the collection as well as the standard. It is a writer's `shall not`; `tree.rs` meets a file that breaks it by chunking the pairs array in twos, so a null key costs its own pair and never re-pairs the remainder against itself, and `a_null_key_yields_nothing_and_leaves_its_neighbours_paired` asserts that for both trees. |

### What reading them made this round look at, which is the point of the rule

**A quotation of §7.9.6 for words ISO 32000-2 prints nowhere.** `tree::name_pairs` said keys come
back as the bytes the file wrote, since §7.9.6 "sorts them by unsigned character code" — and that
phrase is in no clause of the standard, no annex, and none of the technical specifications in
`doc/md/`. It is the sentence #214 amends, quoted in words the clause never had. **No instrument in
this project was placed to reach it**: `check` compares a quotation against text an erratum *struck*
and this one rests on nothing struck at all; `--bin quotations` reads the documents and the ledger
rather than `crates/`; and the conformance gate verifies rustdoc **blockquotes**, where this was a
quotation inside a sentence of prose. It is §7.9.6's own words now.

**And the rename's real landing is a sentence in §12.7.5.4 that no annotation touches.** #214 is
scoped by a `Text` note — *all occurrences ... throughout ISO 32000-2:2020* — with strikes only
where the editor chose to illustrate it, so an instrument built on strikes can see the illustration
and never the rule. Six places in this tree quoted §12.7.5.4's "the name string is the second of the
two array elements", **one of them a rustdoc blockquote in the population the conformance gate
checks**, and two more quoted §7.11.4.1's NOTE about identifying an embedded file "by the name string
provided in the name dictionary". All are prose now, naming the erratum; nothing behaves
differently, because the term the erratum withdraws was never a type.

**A third copy of the sentence Issue #481 struck, hidden by being misquoted.** `viewer_core::command`
put *the tree shall map name strings to file specifications* in quotation marks against §7.11.4.1.
The clause opened that sentence with *the associated name tree*, so the quoted words were never the
standard's — and because they were not, `spec-errata check`'s whitespace-insensitive comparison
matched nothing and the landing that caught `pdf_model::attachment` in the four-hundred-and-eighteenth
session and `viewer_host::panel` in the four-hundred-and-twenty-ninth never came. **A misquotation is
invisible to the instrument that finds quotations of retired text**, which is the sharpest reason
this project has to keep quotation marks meaning verbatim: the cost of a paraphrase inside them is
not only that it is wrong, but that it stops being checkable.

## Table D.3's own cells, corrected three ways and moving nothing — the seven-hundred-and-fifty-fifth

The successor rule's fifth use, and the first run of its fourth step. Ranked over live rows alone
the head is §14.8.5.3, the plateau the fourth use left standing; ranked over **every** row, §D.3
carries fifteen unread annotations under three issues — more than twice the live head's — and is
`implemented`. `doc/todo/01` has the argument for taking the second head; ADR 0671 is the round.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.
All fifteen fall under one heading, so nothing had to be reassembled — which is itself worth
recording, because the four uses before this one each had an issue split across clauses.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| Annex D.3 Table D.3, codes 0x04 and 0x05 | 874 | #562 | **cites** | Two strikes with a caret apiece, `Review/Accepted`, over the *alias* column. `[318.650 709.414 384.651 724.111]` covers the `(END OF TEXT)` that `-bbox` puts at (318.65, 712.44)–384.65 on the 0x04 row — where the same words are already 0x03's — and writes *END OF TRANSMISSION*, which is what U+0004 is; `[318.650 733.434 431.021 748.131]` covers 0x05's `(END OF TRANSMISSION)` and writes *ENQUIRY*, which is what U+0005 is. Both codes carry the annex's `U` note, so both are `None` here and neither erratum reaches a byte this program decodes. |
| Annex D.3 Table D.3, codes 0x18 and 0x19 | 875 | #562 | **vindicates** | The same issue's other half, and it is in the `Character` column: `[88.944 533.608 94.483 548.761]` over the `u` printed for 0x18 and `[89.184 557.638 94.111 572.911]` over the `v` printed for 0x19 — two lower-case letters standing where a breve and a caron belong. **Each caret's `/Contents` is a single byte**, 0x18 and 0x19, so the erratum states its own replacement *in `PDFDocEncoding`*: `spec-errata emit` prints them as `˘` and `ˇ` only because `pdf_syntax::text_string` decoded them through the very table they correct. The annex's `Unicode` column, U+02D8 and U+02C7, is the independent side of that circle and is what `tests/pdf_doc_encoding.rs` reads. |
| Annex D.3 Table D.3, code 0x16 | 875 | #285 | **cites** | `[226.640 490.178 266.643 501.384]` over the `U+0017` that `-bbox` puts at (229.61, 489.09)–263.67 on the 0x16 row, with a caret writing *U+0016* — the published table prints U+0017 twice, on 0x16 and on 0x17. A second strike, `[290.085 486.064 350.542 500.761]`, takes the same row's `(SYNCRONOUS` and writes *SYNCHRONOUS*. 0x16 is one of the annex's `U` codes, so the corrected cell names a character `PDFDocEncoding` still does not have; the row is `None` under both printings. |
| Annex D.3 Table D.3, code 0x8a | 879 | #461 | **vindicates, and is the one that could have cost a page** | `[89.904 701.734 94.664 716.431]` over the `Š` that `-bbox` puts at (89.90, 701.73)–94.66 in the `Character` column of the **0x8a** row, whose `Unicode` cell says U+2212 and whose name cell is empty. One caret carries the byte 0x8a and a second writes *MINUS*. `Š` is 0x97's character, so the published table showed one code's glyph against another's code — and a transcription taken from that column rather than from the `Unicode` one would put U+0160 at 0x8a, which decodes a minus sign as a capital S with caron and round-trips perfectly. The table here reads U+2212 and always has. |

### What reading them made this round look at, which is the point of the rule

**The row's only gate could not see a wrong transcription, and the erratum names the exact mistake
it would have missed.** §D.3 was `implemented` on
`text_string.rs::every_text_string_survives_the_round_trip`, and a round trip is a statement about an
encoder and a decoder that share one array: swap two codes and it still passes. Planted with 0x8a as
`Š` — #461's own misprint, transcribed — all ten of the module's tests stay green.
`crates/pdf-syntax/tests/pdf_doc_encoding.rs` now compares all 256 rows against `doc/md/` in both
directions and fails that plant twice, and the parent §D row's claim that all five tables are
"transcribed from `doc/md/` and gated" is true for the first time.

**And the row named the wrong table for its whole life.** §D.3's note said the code was "the fourth
column of Table D.2" — a font encoding keyed by glyph name, in `pdf-font` — where what
`text_string.rs` holds is Table D.3's code-to-Unicode column. No instrument could print it: the ninth
sweep reads `Table NNN` citations against the entries ISO 32000-2 puts in that table, and an annex
table's number is outside its population.

## §9.6.4's NOTE 2, and the operator category a row said this tree could not run — the seven-hundred-and-sixtieth

The successor rule's sixth use, and the second run of its fourth step. Over live rows the head is
§14.8.5.3 with seven annotations — the same plateau the third, fourth and fifth uses left standing,
because none of the three took its row from that list. Over **every** row §D.3 is gone (755 read it)
and the head is **§9.6.4 with eleven annotations under four issues**, `implemented`; §7.4.1 with
eight is second. Both figures reproduce what 750 measured from outside before the step existed,
which is what said the arithmetic was right before it was trusted. ADR 0681.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §9.6.4 opening sentence, NOTE 2 | 332 | #111 | **corrects, and is the one that moved the row** | A strike at `[236.683 417.989 289.271 430.409]` over the `operators.` that `-bbox` puts at (239.98, 410.30)–288.29, ending "In Type 3 fonts, glyphs shall be defined by streams of PDF graphics operators", with a caret at `[281.508 418.072 290.45 425.358]` writing *objects*. Three `Text` notes below it insert *NOTE 2 Type 3 glyphs can use any PDF operator from any operator category (see "Table 50 - Operator categories" and "Figure 9 - Graphics objects") subject to additional restrictions described in this clause.*, a paragraph reading *Implementations also need to avoid potential infinite recursion if a Type 3 glyph description refers to itself directly or indirectly. The result in all such cases is implementation-dependent.*, and an EDITOR NOTE that the remaining NOTEs will be renumbered. `check` is blind to the strike for the second-listed reason — one word, under the four-word floor — and `crates/pdf-model/src/type3.rs`'s module comment quoted the sentence it retires. The recursion paragraph is `draw_type3_glyph`'s `MAX_FORM_DEPTH`, written from principle 3's budgets before the clause said it, and *reported* where the clause permits anything. What NOTE 2 was worth is below. |
| §9.6.4 Table 111, `d0` and `d1` | 335 | #43 | cites | Six bare `Caret`s, `Review/Completed`, and the **sixth family of `Caret` with no `StrikeOut`** this file records. `[172.136 647.931 180.166 654.475]` and `[475.327 636.173 483.358 642.716]` sit at the starts of `d0`'s "wx denotes the horizontal displacement…" and "wy shall be 0…", `[172.136 474.547 180.166 481.091]` and `[475.327 462.788 483.358 469.332]` at the same two sentences under `d1`, and `[172.136 436.274 180.166 442.817]` and `[433.365 436.274 441.395 442.817]` ahead of "llx and lly denote…" and "urx and ury denote…". Each writes *The number*, *The numbers* or *the numbers*. Grammar: the sentences named a symbol where a noun belongs. `run.rs` quotes the half of the first one the carets do not reach — "it shall be consistent with the corresponding width in the font's Widths array" — and is unaffected. |
| §9.6.4 EXAMPLE | 336 | #144 | **vindicates a fixture** | A strike at `[175.7 259.107 193.964 266.35]` over the `104` that `-bbox` puts at (177.62, 574.20)–192.04, on the EXAMPLE's `/LastChar 104`, with a caret at `[189.436 259.154 194.652 263.405]` writing *98*. The same object states `/FirstChar 97`, `/Differences [97 /square /triangle]` and `/Widths [1000 1000]`, so the published number contradicted three of its own neighbours. `crates/pdf-model/tests/type3.rs` builds its fixture from this EXAMPLE and has written `/LastChar 98` since the tenth session — the transcription was corrected on the way in, before there was an erratum to correct it. |
| §9.6.3 (filed under §9.6.4) | 332, 21 | #553 | untouched | A caret at `[515.898743 692.721497 525.958923 700.918701]`, at the end of the `system.` that `-bbox` puts at (482.78, 134.61)–518.38 — the last of §9.6.3's bullets on deriving a PostScript language name — writing *An Adobe technical note provides a specification for Postscript name generation that can be used for instance fonts derived from variable fonts. See Adobe Technical Note #5902: "PostScript Name Generation for Variation Fonts".*, with a `Text` note on page 21 adding that note to clause 2's normative references. It tells a *writer* how to name an instance of a variable font; `/BaseFont` is a substitution request here and no name is derived from anything. Filed under §9.6.4 because page 332 carries the end of one clause and the start of the next, which is this file's page-straddle for the fifth round running. |

### What reading them made this round look at, which is the point of the rule

**NOTE 2 states a permission, and §9.6.4's row denied exactly one of the categories it covers.** The
row said, from the tenth session until this one:

> A glyph description whose marks are an inline image draws nothing yet and reports, which is §8.9.7's
> gap rather than this one's: 10 corpus documents are in that position.

All three claims were false, and for all but one of those sessions. `pdf_model::inline_image` landed
in the **eleventh** (ADR 0019) and §8.9.7 has been `implemented` since, so there was no gap to
attribute a refusal to; measured, a `d0` description's inline image is drawn, at the matrix the
description's own `cm` gives it, and nothing is reported. It is the ledger section of `doc/habits.md`'s
third shape — a capability that arrived and announced nothing — and it stood for seven hundred and
fifty rounds because nothing in this project compares a row's *denial* against a sibling row's
status. The corpus figure is withdrawn rather than re-derived: it counted documents exercising a debt
that does not exist.

**A settled row's erratum finds the evidence weaker than the row**, which is the fifth use's lesson
holding for a second row and a different reason. There it was a gate that could not fail; here the
clause's own restriction on the permission — Table 111's "the glyph description shall not include an
image; however, an image mask is acceptable" and §8.6.8's "unless painting an image mask, all image
painting operators shall be ignored" — was implemented in `content/image.rs` with no test either
side of it. `an_inline_image_is_a_glyph_description_s_marks_like_any_other_operator` and
`a_d1_glyph_description_drops_an_image_and_keeps_an_image_mask` hold both now, calibrated against
three plants: images dropped inside a description, which **no pre-existing test in the file could
see**; the font matrix lost; and the image-mask exception removed.

**And trap 13 sprang on the calibration itself, in its own words.** The first transpose plant —
swapping `b` and `c` of the glyph's transform — passed, because the font matrix composed with the
text rendering matrix is diagonal and a diagonal matrix agrees with its own transpose. The fixture's
glyph description now states `750 0 200 375 0 0 cm`, whose shear makes the placed matrix disagree
with its transpose, and the same plant fails. A rectangle is not asymmetric enough.

### And an erratum's *added* text cannot be a rustdoc blockquote

Recorded because it cost this round a red gate and will cost the next one the same. `doc/errata-read.md`
has said since its second table that a corrected quotation keeps the published wording, "because
`doc/md/` is what the gate verifies against". The mirror of that rule had never been written down:
`cargo test -p conformance`'s `every_quotation_is_the_standards_own_words` reads every rustdoc
blockquote under `crates/` and asks `doc/md/` for it, and an erratum's **inserted** sentence is in no
clause of that conversion — so quoting NOTE 2 as a blockquote fails the gate, correctly, with *§9.6.4
does not contain … as written*. The convention that already exists is `measurement.rs`'s for Issue
#534: an erratum's replacement text goes in *italics*, naming the issue, never between `> ` or between
quotation marks. Both places this round wrote it now do.

## §7.4.1's two producer rules, the filter set that became this program's, and an example missing its EOD — the seven-hundred-and-sixty-fifth

The successor rule's seventh use. Over live rows the head is **§7.6.4.1 and §7.6.6 with six
annotations apiece**, and the four-use plateau above them is gone for a reason that is this round's
finding rather than the decay working — see below. Over **every** row §9.6.4 is gone (760 read it)
and the head is **§7.4.1 with eight annotations under two issues**, `implemented`, which is the
figure 750 measured from outside and 760 named as second. Both issues fall under one `emit` heading,
so nothing had to be reassembled. ADR 0691.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §7.4.1 first paragraph, and the categories sentence | 49, 50 | #216 | **corrects, and is the one that moved the row** | Three annotations, `Review/Accepted`. A strike at `[138.740005 249.809113 153.964203 262.747986]` over the `are` that `-bbox` puts at exactly (138.74, 579.17)–(153.96, 592.11) — the fourth of four on the page, and the rectangle picks it out to the hundredth of a point — in "Whether to do so and which decoding filter or filters to use are specified in the stream dictionary", with a caret at `[313.168701 248.161377 323.228882 256.358582]` writing *shall be*; a second caret on the same line, past `dictionary.` at (265.30, 579.17)–(315.75, 592.11), inserting *All stream data shall follow the appropriate format(s) as described below.*; and a strike at `[93.863998 640.609131 113.360596 653.547974]` over the `files` at (93.86, 188.37)–(113.36, 201.31) in "PDF files support a standard set of filters that fall into two main categories", with a caret writing *processors shall*. The first two are a producer's. **The third is not**, and it is what reading this issue was worth: below. |
| §7.4.1 EXAMPLE 3 | 51 | #527 | **corrects an example, and its two halves check each other** | Two annotations, `Review/Completed`. A caret at `[84.104729 372.66507 91.43129 378.634857]` — (84.10, 463.29)–(91.43, 469.25) from the top, at the end of the last base-85 line and immediately above the `endstream` at (68.54, 470.01) — writing *~>*, so the example's ASCII base-85 stream had been printed without §7.4.3's end-of-data marker; and a strike at `[154.820007 478.438507 159.643997 486.647339]` over the last glyph of the `447` at (145.22, 355.27)–(159.64, 363.48), with a caret writing *9*, so `/Length 447` becomes 449. **Two bytes of marker and two of length**: the corrections are one change and each confirms the other's arithmetic. This tree quotes the example's arrangement — `/Filter [/ASCII85Decode /FlateDecode]` — in `filter.rs` and in `nested_content_window.rs`, and its bytes nowhere, so nothing here transcribed the missing marker. It is corroboration for Issue #293's addition to §7.4.3, read in the six-hundred-and-fifty-sixth: the marker is a rule rather than a convention, and the standard's own example is now written as though it were one. |

### What reading them made this round look at, which is the point of the rule

**Issue #216's third annotation moves a sentence from describing a file to obliging a processor.**
"PDF *files support* a standard set of filters" becomes "PDF *processors shall support* a standard
set of filters", so Table 6 stops being an inventory of what documents contain and becomes a closed
set this program owes. It is owed in full and it is met — five byte-to-byte filters in
`filter.rs`, `Crypt` a pass-through because §7.6.2 has already decrypted the bytes by the time a
chain runs, and four image codecs recognised by `is_image_codec` for the image pipeline to run —
and **nothing asserted it.** Every filter in Table 6 has a test of its own *output*; not one asked
whether the table was covered, so a name dropped from `decode_reported`'s match or from
`is_image_codec` becomes `FilterRefusal::Unsupported`, which is also what a name from no table gets,
with every other test in `pdf-syntax` green. That is the fifth use's shape for a third row and by a
third mechanism: there a round trip that could not fail, then a sentence about a sibling row, and
here a set with no closure check.

`every_filter_table_6_names_is_supported_under_both_of_its_spellings` walks Table 6's ten names and
Table 92's seven abbreviations — seventeen spellings, because `decode_reported` admits the inline
form beside the full one — and asks of each whether it decodes here or is an image codec, and
whether the two spellings of one filter answer alike on the same bytes. Calibrated per trap 13
against two plants, each of which the whole crate is otherwise silent about: `JPXDecode` taken out
of `is_image_codec`, and `A85` taken off `ASCII85Decode`'s arm.

**And §7.4's own row could not add up.** It said Table 6's ten filters were "[f]our … stream filters
implemented here, one … a pass-through … and four … image codecs", which is nine, while the same
note has said since the seven-hundred-and-fourteenth that "all five of Table 6's byte-to-byte
filters can be windowed". `filter.rs` decodes five. The wrong number is corrected in the row, and
`--bin counts` is not at fault for missing it: a cardinal is a claim about a family there only where
it governs one of the ledger's own words for a row, and *stream filters* is not one.

### The eighth blindness, and it is this rule's own record naming an issue in order to disown it

**The live ranking's head did not move because a round read it. It moved because a sentence
mentioned it.** 760 recorded, correctly, that an early draft of its ADR had written two issue
numbers in full and taken §14.8.5.3 off the ranking without a verdict — and the sentence recording
that fix writes both numbers with the `Issue #` prefix, inside backticks, in
`doc/history/760-the-operator-category-a-row-said-we-could-not-run.md`. Step 2's first grep cannot
tell a **use** from a **mention**. So at this round's base both numbers count as named, the
population is 118 where restoring the two gives 120, and §14.8.5.3 — the live head for four
consecutive uses — is not in the live ranking at all.

Measured rather than inferred: with the two numbers put back, this round's script prints exactly
760's own figures — 120 named nowhere, §14.8.5.3 the live head with seven annotations under two
issues — and without them the live head is §7.6.4.1 and §7.6.6 with six. That is the calibration
that says the mention is what did it.

**Neither number is written here**, and that is not squeamishness: this file's own bare column is
step 2's *second* grep, so recording a number here is how an erratum leaves the population, and
these two have no verdict to leave it with.

**The repair is a rule about writing, as 750's was, because no third grep can see the difference.**
A sentence about the *form* of an issue number must not contain one: write "with the `Issue #`
prefix" and say how many, never which. The two this round found are left in the population on
purpose, in the form the greps already read, and they will be at the head of the live ranking again
the moment somebody looks — which is where they belong, since no round has yet given either a
verdict.

## The clause an action refuses, and the end-of-line rule a byte string made load-bearing — the seven-hundred-and-seventy-first

The successor rule's eighth use. Over live rows the head is **§7.6.4.1 and §7.6.6 with six
annotations apiece**, unmoved, because no round has taken either. Over **every** row the head is a
three-way tie at six between those two and **§12.6.4.17, `out-of-scope`**, which the fourth step's
tie-break settles in the settled row's favour. §7.4.1 is gone from both, the seven-hundred-and-
sixty-fifth having read it, which is the decay working for the fourth use in a row. ADR 0708.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §12.6.4.16, opening sentence and Table 220 | 541 | #282 | **inside the exclusion, and recorded so that the row is not read again** | Four carets, `Review/Completed`. The first at `[259.276 711.053 268.29 718.397]` — (259.28, 123.52)–(268.29, 130.87) from the top — sits exactly between the `3D` at (248.04, 117.45)–(261.46, 130.39) and the `annotation` at (263.89, …)–(315.24, …) in "identifies a 3D annotation and specifies a view for the annotation to use", and writes *or RichMedia*. The other three widen `/V`: *or Views* beside the `VA` array, and twice *, or the Views array of the RichMediaContent dictionary (see "Table 341 - Entries in a RichMediaContent dictionary"), as appropriate for the specified annotation*. §13.7's ground, which `CLAUDE.md`'s multimedia exclusion already names. |
| §12.6.4.16, Table 220's `/S` row | 541 | #265 | **corrects a self-reference, and reaches nothing here** | A strike at `[255.543 592.197 303.604 603.401]` over the `transition` that `-bbox` puts at (258.51, 237.43)–(300.63, 249.10) in "shall be GoTo3DView for a transition action", with a caret at `[296.6 592.271 304.668 598.845]` writing *Go-To-3D-View*. The published row names the *wrong action type* for the type it defines, §12.6.4.15's rather than its own. `action.rs` matches the keyword `GoTo3DView`, which neither annotation touches. |
| §7.9.2.4, the whole example paragraph | 132 | #276 | **corrects, and is the one that moved a row two clauses away** | A strike at `[68.7318 97.0593 550.841 179.219]` — (68.73, 662.70)–(550.84, 744.86) from the top, four lines — over the old "For example, byte strings are used to define a file identifier …" paragraph, with a caret at `[280.669 97.1422 289.612 104.428]` writing the replacement, and a second at `[93.9407 72.2497 102.424 79.1618]` writing *2* to renumber the NOTE below it. The replacement opens with the sentence this round was worth: *Unless otherwise stated in this document, a byte string may be either a literal string (see 7.3.4.2, "Literal strings") or a hexadecimal string (see 7.3.4.3, "Hexadecimal strings").* Then the identifier as an EXAMPLE, and *NOTE 1 The Contents entry of a Signature dictionary can be required to be a hexadecimal string (see "Table 255 - Entries in a signature dictionary").* |
| §7.9.2.2.1, NOTE 5 | 132 | #96 | **deletes an informative NOTE, and a ledger citation of it retired with it** | A strike at `[72.024 644.076 535.452 666.996]` — (72.02, 174.92)–(535.45, 197.84) from the top — over the whole of 'NOTE 5 It is important not to confuse UTF-16BE with UCS2 (i.e. wchar_t). UTF-16 is not a fixed width encoding scheme.' No replacement. §7.9.2.2.1's row cited that NOTE as what the surrogate-pairing decoder 'is there to warn against'; the reason the decoder pairs surrogates is the clause's own normative sentence about supplementary characters, so the row now cites that and the NOTE in the past tense. |
| §7.9.2.2.1, NOTE 4 | 132 | #161 | **corrects a NOTE, and the NOTE is informative** | A strike at `[118.551 726.576 160.267 738.456]` over the `dieresis,` that `-bbox` puts at (121.70, 102.31)–(159.28, 114.68) in "precludes beginning a string using PDFDocEncoding with the three characters dieresis, guillemotright, questiondown", with a caret at `[152.841 726.655 161.395 733.625]` writing *idieresis*. The UTF-8 marker is EF BB BF and Table D.2 gives 357 octal to `idieresis`, so the published NOTE named the character for BFh's neighbour rather than for EFh. Nothing decodes from a NOTE. |

### What reading them made this round look at, which is the point of the rule

**Issue #276 tells a settled row which syntax its bytes may arrive in, and the row had evidence for
one of the two.** §7.9.2.4 is `implemented`; its whole test list was
`hex_strings_ignore_junk_and_pad_an_odd_digit`. The erratum makes the literal form the standard's
own alternative for *every* byte string — so the row's claim covers §7.3.4.2 as well, and §7.3.4.2
is where the question had to go.

**§7.3.4.2's end-of-line rule was not implemented and nothing asked for it.** The clause states:

> An end-of-line marker appearing within a literal string without a preceding REVERSE SOLIDUS shall
> be treated as a byte value of (0Ah), irrespective of whether the end-of-line marker was a CARRIAGE
> RETURN (0Dh), a LINE FEED (0Ah), or both.

`Lexer::read_literal_string` handled all eight of Table 3's escapes, the octal form and the line
continuation, and let an *unescaped* end-of-line marker through as itself: a bare CARRIAGE RETURN
became 0Dh, and a CARRIAGE RETURN with a LINE FEED behind it became two bytes where the clause
states one. §7.3.4.2's own row enumerated what the reader took and that `shall` was not on the list,
which is why neither the row nor the code could be read against the other and find it.

It is not a rule about display. A byte string's *bytes* are what §7.6's algorithms hash and compare,
so a `/U`, a `/Perms` or an `/OE` written as a literal string with an unescaped marker in it was one
byte longer than the file states; a `/ID` compared for equality could differ from itself under
§7.5.6; and one lexer serves the content streams, so inside a text-showing operator the same byte is
a glyph code and a different byte is a different glyph.
`an_unescaped_end_of_line_in_a_literal_string_is_one_line_feed` asks all four unescaped forms —
including a LINE FEED followed by a CARRIAGE RETURN, which is *two* markers and therefore two bytes
— and the two escaped controls that must not move. Calibrated per trap 13: each of the four fails
against the code that preceded it, and against a plant writing 0Dh in place of 0Ah.

### Two things about the instrument, and neither is new

**The settled tie-break was decided by a row whose annotations belong to its neighbour.**
`spec-errata emit` attributes an annotation by the *outline section for its page*, and page 541
opens §12.6.4.16 and reaches §12.6.4.17 before it ends — so all six of §12.6.4.16's annotations
print under §12.6.4.17. Here it changed nothing: both rows are `out-of-scope` under the same
exclusion, and the tie-break wanted a settled row. It is the same coarseness the four-hundred-and-
twenty-ninth session recorded, arriving from the ranking's side rather than from `check`'s.

**And a settled row's erratum found the evidence weaker than the row for a fourth time, by a fourth
mechanism.** 755 found a round trip that could not fail; 760 a sentence about a sibling row's
status; 765 a set with no closure check; this one a row whose claim covers two written forms and
whose evidence covered one. What the four share is only the status.

## The identifier that stopped naming its contents, and the precedence rule two class objects never had — the seven-hundred-and-seventy-fourth

The successor rule's ninth use. Over live rows the head is **§7.6.4.1 and §7.6.6 with six
annotations apiece**, standing since the seventh use; over **every** row it is the same two,
§12.6.4.17 having left when the eighth use read it and no settled row reaching six. The two
rankings agree at the head for the first time, so the fourth step's settled preference has nothing
to prefer and the third use's tie-break decides between the two live rows: §7.6.6's issue rewrites
a table's *type cell* where §7.6.4.1's substitutes a word in prose, so §7.6.6 leads. Both were
read whole; each falls under one `emit` heading. ADR 0712.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §7.6.6, Table 27's `/Recipients` | 107 | #16 | **corrects a type cell nothing here reads, and the row now says why not** | Three StrikeOut/Caret pairs, `Review/Completed`. Strikes at `[137.33 274.297 168.22 285.501]`, `[204.272 250.747 235.162 261.951]` and `[249.341 88.727 280.34 99.931]` — (557.0), (580.6) and (742.6) from the top — over the `string` of the type cell "string or array", of "where each string shall be a binary-encoded CMS object", and of "this entry shall be a string that shall be a binary-encoded CMS object", each caret writing *byte string*. The CMS objects are bytes, and §7.6.5.3's digest runs over each item's bytes, so decoding them as text would corrupt exactly what the key derivation hashes. Nothing here reads the entry: §7.6.5's family refuses the handler by name before any crypt filter dictionary is opened, which is where the debt is recorded — and §7.6.6's note now states that its own enumeration stops at Tables 25 and 26 for that reason. |
| §7.6.4.1, the revision-4-and-later paragraph | 92 | #89 | **corrects prose, and a tolerance is stated** | Three StrikeOut/Caret pairs, `Review/Completed`, at `[241.831 418.949 303.283 431.369]`, `[134.707 374.549 196.171 386.969]` and `[111.92 359.669 173.384 372.089]` — (411.2), (455.6) and (470.5) from the top — each over `crypt filters`, each caret writing *a crypt filter*: named StdCF, named DefaultCryptFilter, named DefEmbeddedFile. Each name denotes one filter, so the standard handler's stated support is Identity and one filter named StdCF with AuthEvent DocOpen. `crypt::crypt_filters` resolves whatever names `/CF` states — wider than the amended "limited to", and §7.6.4.1's row records that as a reader's tolerance now. The two public-key names are behind §7.6.5's refusal. |
| §13.6.7.3.3 | 705, 706 | #645 | **inside the exclusion** | A strike at `[315.48 275.759 327.214 288.698]` over a glyph the table printed as `a I`, with a caret writing *a 3D measurement dictionary*, and three carets writing *(digit 1)*. Clause 13's ground, which `CLAUDE.md`'s multimedia exclusion names; the row is confirmed. |
| §14.5, Table 350 | 734 | #69 | **corrects a producer's naming rule** | A strike at `[181.848 227.039 230.472 239.459]` — (602.9) from the top — over `keyed by`, with a caret writing *key should be a second-class name, or*; a caret at `[197.037 90.428 205.067 96.971]` widening Table 350's key column with *(recommended), any conforming product name, or well known data type*; and a Text note saying the widening exists so ISO 32000-1 documents upgrade. Who may write which keys, end to end; the `inapplicable` row's disposition survives it. |
| **§14.4**, both identifier sentences | 734 | #328 | **quotes — a gated blockquote and a ledger note stood on the struck words, and the outline filed the strike under §14.5** | Two StrikeOuts, `Review/Accepted`, no replacement: `[234.583 679.849 306.799 692.788]` — (149.1) from the top — over `contents of the` in "a permanent identifier based on the contents of the PDF file at the time it was originally created", and `[170.749 650.209 220.627 663.148]` — (178.8) — over `'s contents` in "based on the PDF file's contents at the time it was last updated". Both identifiers become ones *based on the PDF file at the time* — loosened, and rightly: the clause's own suggested computation names the time, the location and the size, none of which is the contents. `write.rs::identify`'s rustdoc blockquote — the one population with a gate — and §14.4's ledger note both quoted the retired wording as the writer's warrant, and both strikes are under `check`'s four-word floor: the third of this rule's uses to find quoted text on a strike below that floor, after #117 and #534 — #181 was the same blindness met by running `emit` before writing. The behaviour stands — the appended-to bytes *are* the file at the time it was last updated — and the warrant moved. `emit` files both strikes under §14.5, page 734 opening §14.4 and reaching §14.5, which is also how #691's verdict above came to be written against the wrong row. |
| §14.7.6.2 | 753 | #289 | **implements — the rule was satisfied by construction and evidenced by a fixture too small to fail it** | A caret at `[322.113 119.996 331.126 127.34]` — (714.6) from the top, after "along with those identified in the element's A entry." — inserting *Attribute objects included through a class and through an array of classes within the C entry may have the value of O and NS repeated. If a given attribute is specified more than once across the attribute objects, the later (in array order) shall take precedence.* The published clause ranked `/A`'s objects among themselves and `/A` against `/C`; two class objects disagreeing was stated nowhere. `Tree::attributes` walks `/C` in array order, each class's objects in theirs, and `Tree::attribute` takes the last match — but the row's one test attached a single class object, which no ordering of the class route can fail. `an_attribute_two_class_objects_state_goes_to_the_later_one` is the evidence now, calibrated per trap 13 against a plant that walks the classes in reverse: it passes the single-class test and fails the new one. |
| §14.7.6.2 | 753 | #305 | **cites** | A caret at `[112.003 149.45 121.016 156.794]` — (685.1) from the top — inserting *(deprecated in PDF 2.0)* after "revision numbers", and a strike at `[347.503 164.519 394.306 176.939]` — (665.0) — over `typically`, with a caret writing *possibly*. §14.7.6.3 already opens with the deprecation; reading the pairs stays necessary whatever their status, because an integer beside a class name is the only thing that says whether an array element is a name or a pair. |

### What reading them made this round look at, which is the point of the rule

**Issue #328's two strikes were the round's finding three times over.** A rustdoc blockquote in
`write.rs::identify` — the population `cargo test -p conformance` gates — quoted the first
identifier's sentence whole, its prose quoted "based on the file's contents" as the meaning of what
the function derives, and §14.4's ledger note quoted the second sentence's retired wording as the
reason deriving `/ID[1]` from the appended-to bytes conforms. No instrument could see any of the
three: the strikes are three words and two, under `check`'s floor. All three now carry the amended
wording with the erratum named, and the behaviour is unchanged in all three, because bytes of the
file are the file at the time it was last updated.

**And the record's own #691 verdict had judged the wrong clause's row**, corrected above in place:
the outline's page-straddle put §14.4's uniqueness paragraph under §14.5's heading, and the
four-hundred-and-eighteenth session's verdict inherited the filing — "a NOTE about detecting a
changed page" for body text about identifier uniqueness, and "names no digest" of a row one clause
away from the writer that names MD5. The eighth use met the same coarseness at a cost of nothing;
this is the first time it reached a recorded verdict.

### Three things about the rule itself, from running it

- **The two rankings agreeing at the head is the decay finishing its work, not a new regime.** The
  settled rows that out-ranked the live head on four consecutive uses are gone because those uses
  read them; what is left above five annotations is the live pair no round has taken. The tie-break
  between two live rows is the third use's cell-over-prose rule, and it earned its keep: the
  cell-correcting issue led to a row correction, and walking on downward is what found #328.
- **A settled row's evidence was weaker than its claim for a fifth time, by a fifth mechanism**:
  755 a round trip that could not fail, 760 a sentence about a sibling row, 765 a set with no
  closure check, 771 a claim of two written forms with a test of one — and now a rule satisfied by
  construction whose only fixture was too small to exercise it at all.
- **The eighth use's arithmetic was off by one, and the record is why it is checkable.** It wrote
  "110 after this round" of a population of 115; one of its five issues had carried a verdict in
  this file's tables since the four-hundred-and-eighteenth session, so only four newly left the
  population and the count at this round's base is 111. A round quoting the previous round's
  closing figure is quoting a derivation, and the greps are the instrument.

## The undefined cell in a normative matrix, and the floor a descent was already standing on — the seven-hundred-and-seventy-ninth

The successor rule's tenth use. Over live rows the head is **§9.8.1 with six annotations under
three issues** — §7.6.4.1 and §7.6.6 both left when the ninth use read them, which is the decay
working for a fifth consecutive use. Over **every** row the head is **Annex L with seven under
two issues, `writer-side`** — the fifth time the full ranking has out-ranked the live one — so
step 4 takes the settled head, and the eighth use's practice follows: the head to a verdict,
then downward until a row pays. The settled head confirmed its row and paid nothing, exactly as
the eighth use's did; the live head paid three times. The base population reproduces the ninth
use's closing arithmetic exactly: 302 issue numbers carry a strike or a caret under the
recipe's own single-issue line parse and 104 were named nowhere. Five gain verdicts this round.
ADR 0716.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| Annex L, the opening NOTE and Table L.2's P section | 962 | #83 | **amends the matrix, and confirms the row** | A Text note at `[31.686 490.426 51.686 508.426]` inserting *NOTE 2 Table is now permitted as a child of P. Table is now indicated as a valid child of P with a 0..n relationship and, in the Table section, P is listed as a valid parent also with a 0..n relationship.*, with a StrikeOut at `[68.875 642.576 101.52 654.456]` — (187.5) from the top, over the `NOTE` of "NOTE This annex was corrected (2020)." — and a caret writing *NOTE 1*. The published sections list the pair in neither direction: P's children table (p. 969) runs NonStruct to content item with no `Table`, and Table's parents table (p. 981) runs Document to Artifact with no `P`. Two FileAttachment annotations on the same page carry the corrected informative matrix, naming five issues in one field — which is the multi-issue line the recipe's single-issue parse skips. Nothing here reads a cell of Table L.2; the row is `writer-side` and says a checker of tagged PDF would, and that checker takes the amended matrix. |
| Annex L, Table L.2's WP and Figure sections | 979, 985 | #440 | **defines two cells the legend never could, and confirms the row** | Two StrikeOut/Caret pairs, `Review/Accepted`. A strike at `[147.98 594.46 152.193 604.42]` — (237.5) from the top — over the `c` in WP's *children* column beside `Figure`, and one at `[348.55 440.14 352.763 450.1]` — (391.8) — over the `c` in Figure's *parents* column beside `WP`, each caret writing *0..n*. Table L.1's legend defines ø, ø*, 0..n, 1..n, 0..1, ‡, [a] and [b] — **no `c`** — so the published matrix constrained the WP/Figure pair with a value the annex gives no meaning. The erratum makes the pair symmetric and defined. A normative table stating an undefined value is a defect in the published standard itself; the correction binds the checker the row names, and nothing here. |
| §9.8.1, Table 120's `/Descent` | 359 | #190 | **quotes — three blockquotes stood on a one-word strike, and the code's stated choice becomes the entry's own floor** | A strike at `[453.165 325.919 489.44 335.879]` — (506.0) from the top — over the `negative` of "The value shall be a negative number.", a caret at `[230.352 313.814 238.305 320.294]` — (521.6), after the `number.` that `-bbox` puts at (200.3, 516.1) — writing *less than or equal to zero*, and a Text note inserting *NOTE While different font programs may define descender metrics using either positive or negative numbers (e.g. OpenType usWinDescent …), PDF always expects negative values.* So the amended entry reads *[t]he value shall be a number less than or equal to zero*. Three rustdoc blockquotes quoted the struck sentence — `pdf-font/src/metrics.rs::measured_extent`, `pdf-model/src/variable_text.rs::Metrics::read` and `pdf-model/tests/variable_text.rs` — and the strike is one word, under `check`'s four-word floor: the fourth of this rule's uses to find quoted text on a strike below it. `measured_extent`'s acceptance of a zero descent was argued in place as this program's reading of a depth against a sign convention, and its own test credited Table 120 with a permission the published table did not state; the erratum makes both the entry's own words. And the inserted NOTE names the mechanism behind the corpus's 42 positive descents — a producer copying its font program's sign convention — corroborating the ADR 0216 repair without legalising the form, since PDF still *expects* negative values. All three blockquotes keep the published wording the gate verifies and carry the amendment beside it. |
| §9.8.1, Table 120's `/FontWeight` | 359 | #474 | **cites — the value set widens and the threshold reads both printings** | A caret at `[462.202 700.082 471.278 707.478]` — (134.4) from the top, between the `shall` and `be` of "If present, the value shall be one of 100, 200, …, 900" — writing *be between 1 and 1000 inclusive, and should*. The nine hundreds become a `should` inside a `shall` of 1..=1000, which is OpenType's usWeightClass range and what a variable font's instance states. `substitute.rs` thresholds the entry at 600 — the same line PANOSE's Demi draws — which reads every conforming value under either printing; no code here enumerates the nine. |
| §9.8.1, §12.8.2.2 and §14.8.5.4.4, one type cell each | 359, 589, 802 | #152 | **implements — and one of the three strikes closed a misread window** | Three StrikeOut/Caret pairs, each over a `number` and each caret writing *integer*. On p. 359 at `[125.69 712.706 165.504 723.911]` — (118.0) from the top — Table 120's `/FontWeight` type cell; `substitute.rs` reads it `as_number`, wider than the amended type, now stated as a reader's tolerance. On p. 589 at `[124.25 657.987 164.064 669.192]` — (172.7), the `number` at (127.2, 171.6) — **Table 257's `/P`**, which `signature::modification` has always read `as_integer`: under the published cell a conforming file could write `/P 1.0` and be read as stating nothing — the table's default, level 2, in place of the level 1 it wrote, a permission-widening misread of a conforming file. The amended cell makes that file malformed and the integer read exact; `a_docmdp_level_written_as_a_real_takes_the_tables_default` pins the recovery, calibrated per trap 13 against the numeric-read plant, which passes every pre-existing signature test and fails only the new one. On p. 802 at `[205.88 708.987 245.694 720.192]` — (121.7) — Table 380's `/GlyphOrientationVertical`, which nothing here reads. **Two of the three strikes are filed one clause late by the outline's page-straddle**: p. 589 prints under §12.8.2.3 for Table 257, which is §12.8.2.2's, and p. 802 under §14.8.5.4.5 for Table 380, which is §14.8.5.4.4's — the ninth use's coarseness met twice inside one issue, and both rows now say so. |

### What reading them made this round look at, which is the point of the rule

**Issue #190 is the round's finding, and it is the vindication shape three times over.** The
zero-descent acceptance was a documented choice standing on an argument about magnitudes and
conventions; the amended entry states the floor outright, and a clause that says a thing is a
stronger answer than one that implies it (`CLAUDE.md` principle 5). The positive-descent repair —
42 of the corpus's 1629 font dictionaries — gains the standard's own account of its mechanism in
the inserted NOTE. And the misread the published sentence invited — a test comment crediting
Table 120 with permitting zero — was true of the amended table before it was true of the
published one, which is the direction this rule exists to catch.

**Issue #152's Table 257 strike is the one that moved code evidence.** `as_integer` was the
amended clause's read before the amendment existed, and no test could see the difference: swap it
for a numeric read and every signature, restriction and forms test stays green, because every
fixture wrote the level as the integer it is. That is the settled-row mechanism's shape on a
*live* family — a read satisfied by construction with no fixture that could fail its alternative
— and the new test is the one place the two reads part.

### Three things about the rule itself, from running it

- **The two heads split the eighth use's way and the practice held.** A settled head that pays
  nothing is a legitimate outcome — the population decays by two, a claim is confirmed rather
  than moved — and the walk downward is where the work was, on the first live row under it.
- **The outline's page-straddle is now the expected case for a multi-clause issue, not the
  surprise.** Two of one issue's three strikes were filed one clause late; the ninth use's rule —
  a verdict written under a heading is a claim about a page, not about a clause, until the
  rectangle has been placed — was applied here before any verdict was written, which is what it
  is for.
- **A normative table can be wrong in a way no reader of this tree could ever meet**, and the
  ranking still surfaces it: Annex L's `c` constrained nobody because it meant nothing, and the
  only consumer it binds is the checker a `writer-side` row promises. Reading it cost minutes and
  the row's note now carries the amended cells for whoever builds that checker.

## The recovery one entry states, and the grammar a reason had confused with a registry — the seven-hundred-and-eighty-ninth

The successor rule's eleventh use. Over **every** row the head is a settled pair tied at five —
**§7.5.4, `implemented`, and §13.6.3.1, `out-of-scope`** — the sixth time the full ranking has
out-ranked or tied the live one; over live rows the head is a four-way tie at five, §7.5.5,
§12.5.6.5, §12.7.5.5 and §14.7.2. Step 4 took the settled pair, both were read to verdicts, and
both confirmed their rows and paid nothing — the eighth use's practice for the third time. The
walk downward then crossed the whole live plateau, all four confirming, and paid one rank
further down: **§7.7.2 at four annotations under four distinct issues**, one of which changed
what this reader answers. ADR 0724.

Every placement below is the strikeout's or caret's own `/Rect` against `pdftotext -bbox` over
`doc/ISO_32000-2_sponsored_EC3.pdf`, on pages 841.92 points tall, with `y` measured from the top.

| clause | p. | issue | verdict | what it turned out to be |
|---|---|---|---|---|
| §7.5.2, the binary-marker sentence | 70 | #272 | **cites — and the outline filed it under §7.5.4** | A StrikeOut at `[198.465 413.399 314.750 424.439]` — (417.5)–(428.5) from the top, exactly over the `comment line containing` that `-bbox` puts at (198.47, 415.58)–(314.75, 428.52) — with a Caret writing *line containing only a comment that starts with*. So §7.5.2's sentence becomes: the header line shall be immediately followed by a line containing only a comment that starts with at least four binary characters. Filed under §7.5.4 because page 70 opens §7.5.2 and reaches §7.5.4 — the straddle again, placed before any verdict was written. Every word binds whoever writes a whole file; this tree writes §7.5.6's appends and never a header, and the lexer skips a comment wherever it stands. §7.5.2's row records it. |
| §7.5.4, §7.5.5, §7.12.3, §13.2.1 | 72–629 | #109 | untouched | ×10. Example typography, end to end: Example 3's cross-reference fields respaced (`06` → *0 6 (separated by a single SPACE)*, twice more in the trailer example), `objectif` → *object if* in §7.12.3, and an EDITOR NOTE about §13.2.1's scrambled bullets. Nothing normative moves and nothing here quotes any of it. |
| §13.6.3.1, 3D stream colour spaces and Table 311's `/Resources` | 664 | #18, #362 | **inside the exclusion** | Five annotations. #18 inserts a sentence routing a 3D stream's `DeviceRGB` through the page's `DefaultRGB` (caret at `[321.066 197.081 329.193 203.703]`, (638.2)–(644.8) from the top) and corrects a plural; #362 appends *; Deprecated in PDF 2.0* to `/Resources` (caret at `[246.716 415.924 254.668 422.404]`, (419.5)–(426.0)) with a NOTE naming under-specification as the reason. Clause 13's ground, which `CLAUDE.md`'s multimedia exclusion names; the row is confirmed. |
| §12.7.5.5, Table 237's `/DigestMethod` | 561 | #159 | cites | Five annotations rewriting the entry's shape: a Caret at `[280.752 417.887 288.705 424.367]` — (417.6)–(424.0) from the top, between the `An` and `array` of "An array of names" — writing *unordered*; a StrikeOut over `The value` at (425.3)–(435.2) with *Array values*; a StrikeOut over `and` at (436.9)–(446.9) with *or*. The amended entry: an unordered array of names whose values shall each be one of the five, SHA512 **or** RIPEMD160. It sits in the seed value dictionary this row already declines whole — `/SV` is unread — so the amended cell binds nobody here. |
| §12.5.6.5, the QuadPoints NOTE and Table 176's `/BS`; §12.5.6.6, Table 177's `/BS` | 498, 500 | #17, #299 | **cites — the NOTE states the split the code already draws** | #17 inserts NOTE 1 — *When QuadPoints is used, the activation area and the visual appearance (including border) of the link annotation are not required to be the same.* — and renumbers the old NOTE to NOTE 2 (strike at (672.2)–(681.2) from the top). That is `link.rs`'s construction exactly: `/QuadPoints` bounds activation, the border stays §12.5.4's rectangle. #299 strikes `PDF 1.6` from both tables' `/BS` version markers and writes *PDF 1.3* — on p. 498 at (698.2)–(708.2) and p. 500 at (463.7)–(473.6), each exactly over the marker on its `/BS` row. A version marker binds a producer; `/BS` is read here whatever the header claims. |
| §7.5.5, Table 15's `/Size`; §7.5.8.2 | 73, 81 | #522 | cites | A Text note at `[478.855 132.486 498.855 150.486]` adding NOTE 2 — the value of Size does not decrease in incremental updates — a Caret at `[435.677 239.802 444.753 247.198]` on p. 81, (594.7)–(602.1) from the top, inserting *(see "Table 15 - Entries in the file trailer dictionary")*, and the same NOTE again there. Informative both times; this reader's `/Size` departure is about a value producers understate, and no two sections' values are compared at all. |
| §14.7.2, Table 354's `/Namespaces` and Table 355's `/NS` and `/R` | 739, 741 | #396, #93 | cites | #396's four carets: on p. 739 *at least all* at `[263.734 347.196 271.687 353.676]` — (488.2)–(494.7) from the top, between the `of` and `namespaces` of the `/Namespaces` cell — and *as referenced from structure elements in the structure hierarchy* after `document`; on p. 741 *that shall also be an element in the structure tree root Namespaces array (see "Table 354 - Entries in the structure tree root")* at `[469.917 177.013 477.869 183.493]` — (658.4)–(664.9) — and *NS is* making the next sentence "If NS is not present". Completeness rules on the writer; the role walk takes each element's own `/NS` and never enumerates the root's array. #93's caret at `[227.010 556.167 235.041 562.710]` — (279.2)–(285.8), at the end of `/R`'s `(Optional)` — marks the revision entry deprecated in PDF 2.0, as #305 did §14.7.6.2's pairs. |
| **§7.7.2, Table 29's `/Lang`** | 116 | #105 | **implements — the recovery the entry now states was stated nowhere before** | A single Caret at `[323.386 535.186 331.416 541.729]` — (300.2)–(306.7) from the top, at the end of the `absent,` that `-bbox` puts at (298.82, 294.55)–(329.38, 306.22) — writing *or invalid (see 14.9.2, "Natural language specification")*, so the entry's last sentence reads: if this entry is absent or invalid, the language shall be considered unknown. §14.9.2.2 is what invalid means — not empty, and not a BCP 47 `Language-Tag` — and this reader carried an invalid catalog tag to every consumer as if it named a language, prose like `(German, not a tag)` included, because the published entry stated the recovery for absence alone. `structure::document_language` answers `None` for a tag that fails RFC 5646 section 2.1's grammar now — well-formedness, which needs no registry, deliberately not the registry judgement — applied to this entry alone, because this entry is the only place the standard states the recovery. `an_invalid_catalog_language_is_unknown` pins it, calibrated per trap 13. §14.9.2.2's row retires its reason of record, which had conflated the grammar with the registry. |
| §7.7.2, Table 29's `/Extensions` and `/StructTreeRoot` | 114, 116 | #242, #348 | cites | #242's Caret at `[286.643 580.215 294.596 586.695]` — (255.2)–(261.7) from the top, inside the `/Extensions` parenthetical — writing *shall be a direct object;*. #348's at `[318.716 665.307 326.669 671.786]` — (170.1)–(176.6), at the end of `/StructTreeRoot`'s `(Optional; PDF 1.3)` — writing *; shall be an indirect reference*. Two shape requirements on the writer, running in opposite directions; `Document::get_key` resolves either shape, which the row records as a reader's tolerance. |
| §7.5.7's shall-not list; §7.7.2 | 77, 112 | #439 | cites | Two carets stating one rule twice: *The document catalog (see 7.7.2 Document catalog dictionary) in an encrypted document* appended to §7.5.7's list of what shall not be in an object stream, and the sentence form under §7.7.2. A writer's rule; the reader's tolerance is the same by construction — a catalog is reached through its cross-reference entry wherever it lives, and an object stream's members are parsed out of the already-decrypted stream data, which §7.5.7's row has said since the four-hundred-and-twenty-fourth session. |

### What reading them made this round look at, which is the point of the rule

**Issue #105 is the round's finding, and what it retired was a reason rather than a behaviour.**
§14.9.2.2's row had declined a BCP 47 parse in one breath — "a BCP 47 grammar would be a
judgement about a registry this program does not hold" — and the sentence conflates two
judgements BCP 47 itself keeps apart: well-formedness, which is RFC 5646 section 2.1's grammar and
needs nothing outside the tag, and validity, which is the registry. The erratum makes the first
judgement a reader's job on the catalog entry, the grammar answers it in eighty lines with no
data dependency, and the registry judgement is still deliberately not made — which is the half
the retired sentence was right about. A row's *reason* can overstate the cost of a requirement
just as it can deny a capability, and no sweep reads a reason's internal logic.

**And the paying row was one rank below a plateau that confirmed four times over.** The five
plateau rows' twenty-five annotations moved nothing — example typography, version markers, an
informative NOTE stating the code's own construction, writer-side completeness rules — while
§7.7.2's four annotations were four distinct issues, two of them requirements. The ranking's
unit is the annotation, and a substitution repeated five times weighs five; the third use's
tie-break already prices that inside a tie, and this round is the first where the *whole head
plateau* was the repeated-substitution shape and the work sat under it.

### Three things about the rule itself, from running it

- **The base count reproduced the closing arithmetic for the second consecutive use**: 302
  issues carrying a strike or a caret under the recipe's own single-issue line parse, 99 named
  nowhere — the tenth use's 104 less its five verdicts — and the multi-issue parse's 310 and
  101 are the tenth's figures less the same five. Two uses running is the record behaving as a
  record; the greps stay the instrument.
- **A settled head that ties is read whole on both sides, and it cost minutes.** §7.5.4's five
  were three of an example-typography issue and two of a strike that turned out to be §7.5.2's
  — so the settled head's real content was one sentence, in a different clause than the ranking
  named, found by placing the rectangle before writing (the ninth use's rule, applied for the
  third round running). §13.6.3.1's five were all inside `CLAUDE.md`'s exclusion.
- **Fourteen issues left the population in one round — the largest single decay this rule has
  produced — and most of them were cheap on purpose.** A plateau of editorial and writer-side
  issues is what the head looks like when the decayed rows are gone; reading it out is what
  moves the next real requirement to the top, and the one requirement in reach this round was
  sitting one rank below it.
