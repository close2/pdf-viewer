# 2523 — A form reached from pages with differing resources is written once for each, and a rebuilt structure keeps the source's claim and signatures

Status: implemented. Session 1844, the R04 and R01 slot of batch one hundred and twenty-five (Sonnet,
high). The second and third items ADR 2500 and ADR 2502 left owed. Context: ADR 0255 (a form with
no `/Resources` is drawn with the page's), ADR 0816 (no content-stream byte is edited), ADR 1187
(the removed encryption's claim), ADR 2500 ("What remains"), ADR 2502 and ADR 2503 (the rebuild and
its outcome), ADR 2522 (the first item of this slot), ADR 2526 (a linearized file's later sections
reached); `crates/pdf-archive/src/survey.rs`, `crates/pdf-transform/src/archive/{reader_resources,recovered,signatures,mod}.rs`.
Q710 is unused.

## R04 — a copy of the form per page

ISO 32000-2 §7.8.3 hands a form that states no `/Resources` the resources of the page it is drawn on,
so one form object reached from two pages whose dictionaries differ resolves one name to two things.
ADR 2500 wrote one dictionary into the form and refused the form when the pages differed. The
remedy is the one ADR 2500 named: the form is written again for each differing dictionary, and the
`/XObject` table of that dictionary names the copy.

1. **The survey records the route** (`OpenedStream.through_page_table`): set by a `Do` made from a
   page's content or from a form that itself states no resources, cleared by any other way the same
   stream is reached on that page (a pattern, an annotation appearance, a form that states its own
   resources). A stream with the mark cleared is not copied, because the copy would be reached by a
   route the new `/XObject` entry does not govern.
2. **A copy is made only where it is complete**: the differing page value is an indirect object `R`
   whose `/XObject` entry is a direct dictionary naming the form, the stream was reached through the
   page table only, and `R` is not the `/Resources` of any other opened stream. Otherwise the form
   keeps the refusal it had (ADR 0947), naming the reason.
3. **The copy is built once and referenced**: the rewrite files an `Edit::Retarget { name, from, to }`
   on `R`, the added object is the form's own rewrites with `/Resources` set to `R`, and every page
   that reaches `R` names the one copy.
4. **Bounds** (`MAX_FORM_COPIES` = 256, `MAX_COPIED_BYTES` = 32 MiB of copied streams). The count alone
   would let one large form be written 256 times; the byte bound stops that. Past either, the form is
   refused by name. The test builds the bounds on both sides: 256 differing pages copy, 257 refuse;
   a total of 32 MiB copies, one byte more refuses. The memory probe is in the record
   (`doc/history/1844-*.md`).

## R01 — the claim, the signatures and the page, read from the source

The rebuild (ADR 2502) reopens a file this program wrote, which is unencrypted and holds no byte range
a signature covered. The conversion that goes on over it therefore asked the wrong document.

1. **Protection is the source's.** `SourceProtection::of` runs over the source before the rebuilt
   document shadows it, and `file-structure/no-encryption`, which the rebuilt file meets, is
   **raised** as a failed judgement of one place when the source is encrypted and the document is the
   rebuilt one (`raised_judgements`, the same mechanism as ADR 2522). It is decided as for any
   encrypted source: the loss `Loss::Encryption` authorised or the conversion stops (ADR 1187).
2. **Signatures are found over the source's bytes** and their positions in the rebuilt file
   (`signatures::find_over`), so each is reported against the bytes it signed while its value is
   removed from the objects written.
3. **A direct page is given an object of its own.** A page written in the `/Kids` array itself is a
   `Carried::Direct` dictionary. One that names its `/Parent` inherits through `split::inherited_from`;
   one that names none has the values the reader's descent resolved for it, which `pdf_model::Page`
   carries after inheritance, stated on the page (`recovered::with_resolved_entries`: `/Resources`,
   `/MediaBox`, `/CropBox` and `/Rotate`, each only where the file states it somewhere; ADR 2502 named
   this remedy).
4. The refusal `recovered::refuse_what_is_not_rebuilt`, which turned an encrypted or a signed source
   away from the rebuild, is removed: what it refused is now read. The stated-count refusal (ADR 2502
   decision 7) stands unchanged.

Tests (synthetic, because the tree's only encryption writer is the V5/R6 handler of a PDF 2.0
header, which part 2 refuses at `file-structure/file-header`, so the encrypted source is tested at part 4
alone): an encrypted source rebuilt behind the loss with the claim reported, and refused without it; a
signed source at both targets; a direct page rebuilt; a direct page without `/Parent` rebuilt with the media box and rotation the
reader applied stated on it.

## What the corpus shows, and the correction to ADR 2503

Over the eight census lines, nothing is freed: R04's differing-pages shape is in no corpus document, so
that item is carried by its tests. R01's `issue19484_1` and `issue19484_2` (encrypted) and
`poppler-395-0-fuzzed` (signed) now get past the rebuild and are refused by other rows of the table
(`graphics/only-operators-the-base-standard-defines` for the first two;
`signatures/signature-widgets-meet-the-annotation-rules` unauthorised and
`annotations/appearance-dictionary-present` for the third); `issue9105_other`, whose page 1 was not an
indirect object, now stops at `file-structure/file-header` (no version this tree reads).

**ADR 2503 stays as a record, and one sentence of it is no longer true.** It says `027613` is refused
because the reader reaches one of the 429 pages the file states. Round 1846 (ADR 2526) made the reader
reach all 429, so the stated-count check no longer refuses that file; its refusal is an ordinary
requirement's. The R01 text of `doc/pdfa-blocking-issues.md` is corrected accordingly.

## Consequences

R01's list of six is down to `poppler-742-0-fuzzed` (a page's dictionary does not parse whole) in the
census's terms, and `027613` and the four above leave it for other rows. R04's remaining shapes are the
Type 3 glyph procedure's resources and a font that moved (ADR 2500), unchanged.
