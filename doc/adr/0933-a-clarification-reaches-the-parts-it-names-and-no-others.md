# 0933 — A clarification reaches the parts it names, and it can widen a row as well as narrow one

Session 942. Status: **accepted**. Continues ADR 0931, which established what a clarification is
and how a reader tells one this tree acts on from committee guidance it does not. This one is about
what happened when eight more of `TechNote 0010`'s items were taken.

## What was taken, and the one that mattered most

Session 941 left eight items owed and named a ninth concern: A026, `DeviceGray` in soft-mask
images, as the item most likely to be costing a conforming file a false failure. It is not, and the
reason is worth recording because it is the shape of several answers below: **`crate::survey`
records an image's colour space only for an image a content stream draws** — an `XObject` reached
through `Do`, or an inline image — and it never descends into the `SMask` entry of an image
dictionary, which it reads for Annex Q's transparency question alone. So no soft-mask image's
colour space has ever reached ISO 19005-2 section 6.2.4.3's rule, and there was nothing to exempt.
The resolution is now written at the rule *and* at the walk's boundary, because it binds any later
round that widens the walk: reaching a soft mask without bringing the exemption would turn today's
silence into a false failure.

Four items are taken as **confirmations** in the same sense — the row already read the sentence the
way the working group resolved it, and the record is now cited so a verdict says which reading it
applied. A003 (an explicitly associated resources dictionary is a page's, a pattern's, a form
`XObject`'s or a Type 3 font's own `Resources` entry, never one inherited through the page tree),
A004 (`q`/`Q` nesting counted within one content stream and not summed across a form `XObject`'s
invocation), A005 (a name's or string's length is its decoded internal representation), A007 (the
character-identifier limit is applied to a `CMap`'s syntax), A012 with A023 (a button field's
normal appearance is always a subdictionary, and the field type is read from the parent field
dictionary where the widget is not merged with it), and A024 (the overprint rule pairs each
painting side with its own overprint parameter).

**A confirmation is worth carrying, and that is a decision rather than a formality.** The
alternative — note it in prose and cite nothing — leaves a verdict that agrees with the committee
indistinguishable from one that agrees by luck. Every one of those six sentences has a second
available reading that a validator could hold, and four of them would fail conforming files:
summing nesting across an invocation, counting a name's escapes, forbidding an `ICCBased` CMYK
stroke because fill overprinting is on, requiring a push button's `/N` to be a stream. The
citation is how a reader of a disagreement can tell which of us read the resolution.

## The two findings, which is what this round is actually for

### A002 widens a row's reach, and the previous round had it as already true

`graphics/named-resources-are-defined` bound ISO 19005-4 alone, because part 4 states in its own
words that the associated resources dictionary shall define the names its content stream
references and **part 2 does not**. Session 941's list recorded A002 as already true on the
strength of the row existing, without reading which part it bound.

A002 is the working group resolving that parts 2 and 3 are read as if that sentence stood in their
section 6.2.2 too — the ambiguity it was asked about being exactly that the published sentence
requires the dictionary to exist and never says the names have to be defined in it. So the row now
binds both parts, and a PDF/A-2 file naming a resource its associated dictionary does not define is
reported where it used to pass.

**This is the first clarification in the tree that makes a rule bind more rather than less.** A021
put a requirement outside validation; A029 withdrew a rule; A020 narrows one. A002 goes the other
way, and it is why `clarification.rs`'s note about what a clarification does to a row now names
three shapes rather than one.

The corpus does not rank it: the witnesses that exercise an undefined named resource sit in the
`PDF_A-4` directory and there is no part 2 equivalent, so the sweep is byte-for-byte unchanged and
`over` stays zero on all six targets. That is `CLAUDE.md`'s two denominators in miniature — the
coverage question moved and the robustness question could not see it — and it is why the reach is
pinned by a test rather than left to the sweep.

### A citation has a reach, and printing it outside that reach is inference

Attaching these records to rows exposed something ADR 0931's fourth condition had implied without
enforcing. `Judgement::clarified_by` was looked up by requirement identifier alone, so a row that
states the same rule in both parts — the button appearance, the overprint mode, the explicit
resources dictionary — would print *as clarified by `TechNote 0010` A003 for ISO 19005-2 and
ISO 19005-3* underneath a **PDF/A-4** verdict. The parts are named in the line, so nothing said
was false; but a resolution printed beside a verdict reads as that verdict's ground, and this one
would not have been.

So `Clarification` now carries its reach as data — the parts of ISO 19005 this crate has targets
for — and `clarifying` takes the part and answers only where the resolution names it. Every item of
this note predates ISO 19005-4 and none of them names it, so the field is part 2 throughout today;
it exists because "this note has no part 4 items" is a fact about one note rather than a rule, and
the lookup should ask rather than assume. A test asserts the negative directly, which is the half
that would otherwise rot silently.

## A020, and a list that answers most of a question

A020 resolves that an XMP value is validated on its type alone, with anything else inferable from
the property's name or description disregarded — which is the approach
`metadata/properties-use-known-schemas` already took — and then lists the rule for each basic type.
One entry of that list changes a check: `Rational` is placed among the types admitting any string,
beside `Text` and `URI` rather than beside `Integer` and `Real`, so this crate no longer holds
`exif:XResolution` and its kind to a quotient. Sixty-odd properties across the EXIF, TIFF and
Dynamic Media schemas are affected and no corpus witness changes side.

Two points of the list are not settled by it and are `doc/questions/Q53`: `GPSCoordinate` is a
basic type the list does not mention at all, and `Date` is given as ISO 8601 where this crate
implements the XMP Specification's six profiles of ISO 8601 and this project does not hold the
dating standard. The tree keeps both checks meanwhile, and the reasons are written at `Lexical`.
A third point is a debt rather than a question: A020's `MimeType` rule is RFC 2046 and nothing is
judged against it.

## What is still owed, and one item nobody had written down

Two items remain, and both are predicates to write rather than rules to withdraw:

- **A010** — an unreferenced named resource is exempt from the part except its file-structure and
  implementation-limit subclauses. Half of it holds here by construction, because the rows that
  read the survey never see a resource nothing references; the other half does not, because the
  rows that walk every object judge an unreferenced image like any other. Taking it means the
  survey recording which names each resources dictionary had referenced. It is the one item of this
  note that could move `over` in either direction.
- **A028** — a default colour space shall itself be defined in the explicitly associated resources
  dictionary, and a processor ignores what that dictionary does not define. The survey reads
  §8.6.5.6's defaults from the resources in force, which for a form `XObject` with no `Resources`
  entry of its own is the page's, so a `DeviceGray` inside such a form is licensed here by a
  `DefaultGray` A028 says to ignore. Taking it adds failures, including to files that break nothing
  today, which is the `over` direction and wants a round with the corpus in front of it.

**A028 was in none of session 941's three lists.** The note was read in full and one of its
twenty-eight items reached no list at all, next to an item recorded as already true whose row bound
the wrong part. The lesson is not that the previous round was careless — it read the note in one
sitting and wrote down twenty-seven items — but that a list of what a document says is checked
against the document and never against the previous copy of the list. `clarification.rs` now says
so in the words above the lists.

## What it would take to change this

For A002's reach: an erratum to ISO 19005-2, or a later reading that the resolution does not carry
to the published part. For the reach field: an item of some future technical note that names ISO
19005-4, at which point the field starts doing visible work. For the confirmations: nothing — they
cost a line each in a report and buy a reader the difference between agreement and coincidence.
