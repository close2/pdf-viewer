# 1025 — A page of the packet the producer wrote

Status: accepted. Session 1006.
Context: `crates/pdf-transform/src/archive/{preserve,config,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/tests/archive.rs`,
`doc/profiles/{only-metadata-loss,keep-everything,as-if-printed}.toml`.
Builds: `doc/rfc/0007` §2's `preserve`, in both of §4.6.1's mechanisms, under the permission
`doc/adr/1014` records and the owner's `A58` gave. `doc/adr/0954` §what-an-appending-remedy-owes is
the bill this pays; ADR 1012 is the configuration format it plugs into and ADR 1018 its companion.
Clauses: ISO 32000-2 §7.7.3.2 (Table 30), §7.7.3.3 (Table 31), §7.9.7, §8.3.2.3, §8.6 (Table 51),
§9.6.2, §9.8.1 (Table 122), §9.8.2 (Table 123), §9.9 (Tables 124, 125), §9.10.3, §12.3, §12.4.2
(Table 159); ISO 19005-2 sections 6.2.11.4.1, 6.2.11.8, 6.6.2.3.1, 6.7; ISO 16684-1 §7.

## 1. What was built

**`preserve` is two mechanisms, and the configuration has to say which** — `doc/rfc/0007` §4.6.1's
finding, which is the owner's: *what the six targets differ about is what may be attached, not what
may be a page*. So a `preserve` row takes a `placement`:

| `placement` | what happens | where it is built |
|---|---|---|
| `"append"` | the content is laid out on pages appended to the document | `metadata/properties-use-known-schemas` |
| `"attach"` | the content stays an attachment — unchanged where the target holds the original, derived into a conforming one where it will not | the two embedded-file sites |

**`attach` is wired to the derivation `derive` already runs rather than written again.** At PDF/A-4f
and PDF/A-4e the requirement does not bind at all: the target holds the original unchanged and the
row is inert because there is nothing to do. At every other target an embedded file is admitted only
where it conforms to a part of ISO 19005, so *keeping the attachment* means attaching one derived
from it — which is `derive`'s machinery under another word, and the report says **derived, not
original**, because that is what it is. A row asking for `attach` with no tool at a target that
binds the requirement is a configuration error naming both the site and the target, and saying which
targets hold the original.

**`append` is the new one**, and it is the first page this program has ever composed.

## 2. Why the metadata packet is the first content preserved

`doc/pdf-a-conversion-limits.md` §3.9 calls ISO 19005-2 section 6.6.2.3.1 *the largest single thing
between this converter and the corpus*, and its three routes are: correct the value (inventing
content, which `A48` forbids), describe a predefined schema as an extension (misrepresenting it in
the file), or remove the property — a loss. The loss is what this converter has had to ask a caller
to authorise, and what the archive then holds nothing of.

`preserve` adds the route that keeps it: **the property still comes out of the packet, because the
clause is about the packet, and the packet the producer wrote is set on pages appended to the
document.** Nothing else changes. What an operator gets is an archive whose metadata conforms and
whose body still carries what the metadata said.

The whole packet rather than the properties alone, and that is deliberate: `pdf_archive` cuts a
property's stated value down to what a report can print, so a page built from the findings would
carry a truncation and call it preservation. The packet's own bytes carry every value in full, and
`doc/rfc/0007` §2 names exactly this — *instead of losing metadata it could be appended or prefixed
as an extra page*.

The other content the catalogue lists for `preserve` — an embedded file's text, a signature's
statement — needs a rewrite this round did not build (§6), and the catalogue says so rather than
this ADR promising it.

## 3. The rule the page is bound by, and how the code keeps it

`doc/adr/1014`: *nothing may be put on such a page whose content did not come from the file.* Two
things follow in the code rather than in a comment:

- the **text** is the producer's own bytes, laid out in the order the packet states them, with no
  sentence of this program's added — not a heading, not a label, not an explanation. The report and
  the file's own `xmpMM:History` are where the explanation lives, because those are not the page;
- the **face** is one the *document* embeds wherever the document has one that can set the text:
  `LoadedFont::code_for` addresses it by character, its `/ToUnicode` is asked what each code means,
  and a font that cannot answer for one character of the content is passed over. A page set that way
  puts nothing whatever on the document from outside it.

**Where the document has no such face, the face is one this program ships**, which ADR 1014 §5
contemplates in as many words — *for text, a face, which must be embedded under every target, and
which `A47`'s permission and condition already govern* — and `A47`'s condition is carried: the
report names the face, per document, and says it is not the document's. The letters' *shapes* then
come from outside the file, exactly as the margin and the type size do; the words do not.

**One finding made that fallback buildable, and it is worth recording because this tree had read the
clause the other way.** `crates/pdf-transform/src/archive/fonts.rs` refuses to write a font
descriptor for a producer's font, "`/StemV` above all, which is a measurement of a face's stems that
nothing in this file states". Table 122 defines the entry — "[t]he thickness measured horizontally,
of the dominant vertical stems of glyphs in the font. Values shall be positive." — and then adds the
sentence that answers it: **"A value of 0 indicates an unknown stem thickness."** The standard has a
way of saying *not measured*, so a descriptor for a face this program chose can be written without
inventing a number; every other entry of it is read off the program's own `head`, `hhea`, `OS/2`,
`post` and `name` tables. That does not lift `fonts.rs`'s refusal, which is about describing
*somebody else's* font, and it is not changed here.

## 4. The placement choices, each one a choice

`CLAUDE.md`'s "one honest limit": where the standard defines nothing, done means a documented choice.
The standard defines none of this. Each is stated with what it costs.

1. **Page size — the size of this document's first page, as a reader sees it.** Its crop box, turned
   by that page's own `/Rotate` where it turns, reduced to a width and a height. A reader turning
   past the last page should not find the paper changing size. *Cost:* a document whose pages differ
   in size gets appended pages matching the first rather than the last. A document whose first page
   states no usable box is refused rather than given a paper size this program picked.
2. **Its own `/MediaBox`, `/CropBox` and `/Rotate 0`, stated rather than inherited.** §7.7.3.3 makes
   all three inheritable, and a page tree root stating a quarter turn would lay the preserved text
   on its side. *Cost:* three entries the producer's own pages may not state.
3. **Margin — one inch, which §8.3.2.3's 1/72-inch default user space unit makes 72 units, capped at
   an eighth of the page's own width and height.** The inch is the conventional margin and the only
   length the format itself defines; the cap is what keeps it a margin rather than the page on a
   document whose pages are 200 units square, where an inch a side leaves a seventh of the width and
   the packet would run to twenty times the pages it needs.
4. **Type size 9 on 12.** Roughly eighty characters of a text face across a letter-page measure,
   which is a line a person reads. *Cost:* a long packet is several pages; the alternative is a
   smaller face and a page nobody reads.
5. **Line breaks — the source's own, and a break at the measure where a line is longer.** Nothing is
   added to mark a broken line: a hyphen would be a character the producer did not write. *Cost:* a
   reader cannot tell a wrapped line from two lines. The alternative, refusing to set a long line at
   all, would lose the content this remedy exists to keep.
6. **A horizontal tab is set as a space**, because it is whitespace with no glyph of its own in any
   text face. **A byte-order mark is not set at all**: ISO 16684-1 §7 puts one at the head of every
   XMP packet to say how the packet is encoded, it says nothing *in* the packet, and no face has a
   glyph for it — setting it would draw `.notdef`, which ISO 19005-2 section 6.2.11.8 forbids.
7. **Whitespace-only lines at the end of the content are not set.** An XMP packet carries kilobytes
   of them as the padding ISO 16684-1 recommends; setting them would cost pages that preserve
   nothing. Blank lines *inside* the content are set, because a producer put them there.
8. **No colour operator is written.** Table 51 makes `DeviceGray` the initial value of the colour
   space parameter and §8.6.4.2's black its initial colour, so the text is black without this
   program choosing a colour. *Cost:* the page still paints in a device colour space, which is why a
   conversion that will carry no output intent refuses the remedy rather than writing a page that
   breaks a requirement its source met.
9. **The pages go at the end, in the order the content is stated**, as kids of the page tree's root
   node — §7.7.3.2's Table 30 makes a node's children "only be page objects or other page tree
   nodes", so a page is one of the two things the root may take, nothing else in the tree changes,
   and no other node's `/Count` does either.
10. **Sixty-four pages is the budget.** `CLAUDE.md` principle 3: an XMP packet is attacker-supplied
    and a bound that is a property of the code is worth more than one that is a property of the
    corpus. Past it the remedy refuses by name.

## 5. What the page owes, and where each is discharged

`doc/adr/0954` and ADR 1014 §5 priced this, and the bill is paid line by line:

| owed | how |
|---|---|
| **page labels (§12.4.2)** | a range is added at the first appended page stating *no* entries, which Table 159 admits — every entry of a page label dictionary is optional — so the pages carry no label. They are not part of the producer's numbering, and letting the last range run on would number them as though they were; a prefix would be this program writing words into a document. A document that labels no pages gets no range, because its own silence already says it. A label tree with `/Kids` rather than one flat `/Nums` refuses the remedy rather than leaving the labels saying something false |
| **the outline (§12.3)** | **not extended, and the omission is argued.** An outline item needs a `/Title`, which would be a string this program wrote — the far side of ADR 1014's line. Appending at the end moves no destination the outline already names, since a destination names its page by reference, so the outline is not left *stale*: it goes on describing exactly what the producer wrote |
| **the structure tree (ISO 19005-2 section 6.7)** | **refused rather than written.** A document carrying a `/StructTreeRoot`, or claiming `/MarkInfo /Marked true`, gets no appended page at all: the entries are inside the same permission (ADR 1014 §3) and are not built, and a page the tree does not describe would make a Level A file non-conforming and a `/MarkInfo` claim false at any level. §6 says what building them needs |
| **the report's sentence** | `Conversion::preserved`: what was preserved, from which object, on which pages, placed how, and — where one was embedded — which face. It is in the text report and in `--json` |
| **`xmpMM:History`** | one `converted` event carrying the same, and the permission is conditional on it: a packet that will not take the entry does not get the pages either, which is the construction `doc/questions/A48` already put on the `/DefaultCMYK` |
| **not the identity conversion** | a document that conforms is copied untouched (ADR 1006); a preserved page is a rewrite and is reported as one |

## 6. What is not built, and what each waits on

- **The structure entries.** A `/StructParents` on the appended page, marked content in its stream,
  a `/StructElem` per paragraph under the tree's root, and an entry in the `/ParentTree` number tree
  with `/ParentTreeNextKey` moved on. The tree edit is the work: a `/Nums` tree takes an entry, and
  one with `/Kids` needs the same reading §7.9.7 gives a name tree.
- **`preserve` for an embedded file's content by appended page.** Laying out an XML attachment's
  text is the same composer, but the requirement stays failed until the attachment *leaves* the
  document, and the rewrite that takes a file specification out of the `/EmbeddedFiles` name tree,
  the catalog's `/AF` and any file attachment annotation is not written. `doc/rfc/0007` §5's
  ZUGFeRD case therefore still reaches PDF/A-2 only through ADR 1018's departure.
- **A signature's statement as a page.** ADR 1007 computes the signer, the time and what verifying
  found; the words that would carry it are this program's, and ADR 1014 §3 admits them as *what the
  signature told a reader*. It needs its own argument about which words those are.
- **Characters outside `/WinAnsiEncoding`.** The shipped face is addressed through Annex D.2's
  encoding, so a packet stating a character outside it refuses unless the document's own font can
  set it. A composite font written for the appended page would lift it.

## 7. What it does, measured

Over the 974 pdf.js documents at PDF/A-2b with `doc/profiles/only-metadata-loss.toml`: nine fail
`metadata/properties-use-known-schemas`; **seven are now preserved on a page**, one is refused
because it carries a structure tree, and one because its packet states a character
`/WinAnsiEncoding` has no code for. Of the seven, two convert to a written file — the other five are
refused by *other* requirements this converter still owes, which is the same answer they gave before
this round. Both written files gained exactly one page, and both were re-opened and held to the
target again before being written, which is stage three doing what it always does.

The fixture in `tests/archive.rs` is the end-to-end proof and it is proved on the copy rather than
promised on the plan (session 971's rule): the output is re-opened, its pages counted, and the
producer's own sentence read back out of the appended page's content stream, code for code.

## 8. Three shipped profiles changed

`only-metadata-loss` answers the site with `preserve`/`append`, which is what its own text already
said it preferred — *the original packet is kept as content* — now that there is a mechanism for it.
`keep-everything` does the same and gains `placement = "attach"` on its two embedded-file rows, the
first two of the three preferences it lists. `as-if-printed` answers the site with `discard`, which
its yardstick decides without hesitating: a printed page carries no XMP at all.
