# 0948 — The two defaults a conforming file needs, and the decision class they wanted

Session 948. Status: **accepted**. The second slice of `quorra-transform archive`
(`doc/questions/A22`), taking sections 4.1 and 4.2 of `doc/pdf-a-conversion-limits.md`: the output
intent and the identification schema.

## Why these two and not something else

`doc/adr/0947` shipped the mechanical rewrites and refused everything else by name. Over the veraPDF
corpus that left the great majority of failing documents refused, and a tally of *which* requirement
stopped each one says why: colour. Almost every real document paints something in `DeviceRGB` or
`DeviceGray`, both parts license a device colour space only through a default space or a PDF/A
output intent, and this tree had shipped no profile to make one out of. The identification schema is
the second half of the same sentence — a file that meets every requirement and does not *say* it is
PDF/A has not made the claim the format exists to carry.

## 1. `Decision::Stated`, a fifth answer the middle stage needed

ADR 0947 gave a failed requirement one of four answers, from sections 2, 3 and 4 of the limits
document: refused, authorised, unauthorised, mechanical. **The output intent fits none of them**,
and finding that out is the most useful thing this slice did.

It is a section 4 default and it loses nothing, so `Mechanical` looks right — except that
`Mechanical`'s own documentation says "the requirement is met by a rewrite that loses nothing", and
`doc/questions/A18` attaches a condition precisely because something all the same is different
afterwards: **an output intent is what a conforming reader colour-manages device colours through**,
so adding one changes what every `DeviceGray`, `DeviceRGB` and `DeviceCMYK` value in the file means
to a reader that is not this one, and makes that profile the default blending space for transparency
as well.

So there is now a fifth variant:

```rust
Decision::Stated { rewrite: Rewrite, reinterprets: &'static str }
```

and the sentence saying what a different reader may now do differently **travels in the decision**
rather than being left to a caller to remember to print. That is `doc/adr/0927`'s condition made
structural: all four of the owner's permissions are of this class — state an interpretation the
standard defines, never fill in an absence — and each is conditional on the same thing, that what
was written is reported. `A21`'s constructed appearances and `A50`'s operator substitution will be
`Stated` too when they arrive; `A48`'s empty glyph is the one the owner already flagged as the
thinnest case, and it will be the test of whether this variant is the right home for it.

## 2. The family question, and why the converter does not answer it

ISO 19005 section 6.2.4.3 licenses `DeviceRGB` through an **RGB** destination profile and
`DeviceCMYK` through a **CMYK** one, so which requirements an intent answers depends on the profile
in hand. That much is in the decision table: a row carries the family it needs, `None` where any
PDF/A output intent answers it, and a row whose family does not match the profile is refused by name
with a sentence naming `--output-intent-profile` and section 10.1's DeviceN fallback.

But three families of rows — a transparency group's `CS`, an `Indexed` or `Pattern` base space, a
`Separation` alternate — are subject to the colour subclauses *through* whatever space the document
put there, and which family that is, is a fact about the document rather than about the requirement.
**The converter deliberately does not work it out.** Deciding it here would mean re-deriving section
6.2.4.3's licences inside the converter, and ADR 0947's first decision is that **the validator is
the reading and nothing in the converter re-reads ISO 19005**. Those rows are answered by the intent
and the *output's own verdict* settles whether it sufficed — which is exactly the case ADR 0947's
fourth decision built the net for. A conversion that did not reach the target writes no file and
names the requirements it still failed.

## 3. Page-level output intents: the clause read, and the feature not needed

PDF/A-4 permits a page-level output intent (ISO 19005-4 section 6.2.3) and PDF/A-2 has no such
thing, so the slice had to decide whether to use one. **The clause decides it.** Part 4's page-level
rule binds only where the document states no PDF/A output intent of its own, so a document-level
intent answers the requirement outright and there is nothing left for a page-level one to do. What a
page-level intent is *for* is a document mixing an RGB body with CMYK inserts — and choosing which
pages get which profile is a decision that cannot be taken from the file. That is a clause read
rather than a feature declined.

## 4. The XMP writer edits spans, and does not round-trip a reading

`pdf_model::xmp` had no writer. The obvious one — parse a packet to properties, add the schema,
print it again — would have been quietly lossy, because that reader keeps neither an `rdf:about`
subject nor a qualifier other than `xml:lang` (ISO 16684-1 section 7.7), and a producer's metadata
is something somebody wrote deliberately.

So `xmp::restate` **edits the producer's bytes**: it tokenises the packet, records the byte spans of
every element and every attribute whose resolved namespace is one it was told to restate, cuts them,
and inserts one `rdf:Description` before the closing `rdf:RDF`. Every other byte crosses unchanged —
the `<?xpacket>` wrapper, the padding, the producer's own properties, the spelling of everything.
The namespaces argument is a *list* because the identification schema is printed with an `http`
scheme in one part and an `https` one in the other, and a property left behind under the other
spelling would be a second claim standing beside the one just written.

A packet this cannot edit — not UTF-8, tags that do not nest, no single `rdf:RDF` with a close tag
of its own — is a **refusal by name**, not a replacement. Replacing it would throw away metadata
nobody authorised throwing away, and section 3 of the limits document is where that question
belongs.

`xmp::packet` is the other case: a whole packet for a document that states none. Its wrapper is ISO
32000-2 §14.3.2's own EXAMPLE rather than a convention read off another producer's file.

## 5. The profile the file already holds wins over the one we would ship

ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 require every entry of an `OutputIntents`
array that states a destination profile to state the **same** object. So a file that already carries
one — a PDF/X intent, say — decides what a new PDF/A entry may name: the entry shares that object,
and the report says the profile was not this conversion's choice. Embedding a second profile beside
it would break the requirement the conversion was trying to meet. Where the held profile is not one
ISO 19005 admits as a destination profile, no intent is added at all and the requirement is refused
by name.

`pdf_model::icc::Identification` is what makes that possible without parsing a transform: the
header's class and colour space at their fixed offsets, and the `desc` and `cprt` tags out of the
tag table. The `cprt` is the load-bearing one — section 10.1 of the limits document records the
ICC's own guidance that a profile's terms of use live there, and turns it into a rule, so a user
embedding somebody else's press profile is told whose it is. `--output-intent-profile` prints it
before the run as well.

## 6. Objects a conversion adds are numbered in the *source's* numbering

A new profile stream and a new metadata stream are objects the source does not have. They are built
with object numbers past the highest any cross-reference section names, checked one at a time
against the document, and entered into the assembly by `replace` exactly as a rewritten object is —
so the closure walk maps their references the same way it maps everything else, and ADR 0947's rule
that a rewritten object is built in the source's numbering holds for an *added* one too. A number
allocated in the output's numbering would have been renumbered to null the first time anything
referred to it.

## What it cost, and what it did not

`ArchivePlan` gains one field, `profile`. `pdf-model` gains two public entry points —
`icc::Identification` and the three `xmp` writer items — and neither is reachable from a rendering
path, so nothing on the launch path or in a page's draw got heavier. The converter runs one extra
XMP parse per document, and only where a failed requirement asks for the schema: a document that
already conforms has its packet left unread, which is what keeps ADR 0947's first rule true of this
slice as well.

## What the next slice owes

- **CMYK with no profile**: section 10.1's DeviceN `/DefaultCMYK` over the standard's own §10.4.2.5
  transform, which `doc/questions/A48` has already permitted, reported per document and recorded in
  `xmpMM:History`. It is the largest single remaining colour blocker.
- **`/Info`**: part 4 forbids the document information dictionary, section 4.2 says to copy it into
  XMP through Table 7's crosswalk and then delete it, and the deletion is a `Loss` needing a word to
  authorise it. The crosswalk needs a §7.9.4 date turned into an ISO 8601 one and `dc:title` written
  as the language alternative its schema defines.
- **Extension schemas** (ISO 19005-2 section 6.6.2.3): a producer's private XMP property with no
  description embedded is the largest single blocker left on the part 2 side, and section 4.2's
  default is to describe them rather than delete them.
