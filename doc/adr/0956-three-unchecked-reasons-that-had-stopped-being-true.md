# 0956 — Three unchecked reasons that had stopped being true, and one that was two rules

Session 958. Status: **accepted**. It closes one `Check::Unchecked` row, splits another in two,
and corrects three claims this crate made about what the project holds.

## Why a reason is read rather than trusted

`doc/questions/A20` forbids weakening the reason on an `Unchecked` row — a requirement nobody has
read is reported not-checked, by name, and a check is never implemented from a secondary source.
The mirror image has no rule against it and turns out to be the commoner failure: a reason that was
exactly true when it was written, and that nobody re-read after the thing it named arrived.

Every reason on every target was read against the tree this session. Seventeen of twenty were
accurate. The three below were not, and the shape of each miss is different.

## 1. The character-data row was waiting on a reader, not on a text

`metadata/xmp-character-data-only-in-simple-values` carried a reason saying `pdf_model::xmp`
accumulated an element's text wherever it sat and kept it only where the element turned out to be
a simple value — so character data under a description or a container left nothing behind to
report. That was true of the reader and it named no missing standard, because there is none:
ISO 16684-1 section 7.2 is inside the preview this project holds, and its closing sentence confines
non-white character data to the element content of the leaf elements standing for simple XMP
values.

**A reason that names a missing reader is a reason with a deadline.** `Xmp::stray_character_data`
is that reader: the same walk, one more `usize` on each frame and a vector that stays empty for a
conforming packet, so the viewer pays nothing and no ADR-sized cost arises. The row is now
`Check::Implemented`.

What it reports is narrower than the sentence, deliberately, and the narrowing is structural rather
than cautious:

- **An element with a child element is not a leaf.** That needs no clause at all.
- **RDF's own grammar elements** — the `rdf:RDF`, a description, one of clause 6.3.4's three
  arrays — stand for a description or an array rather than for a simple value, which clause 6 says.
- **Section 7.6's `rdf:parseType="Resource"`** is the packet declaring its own value a structure
  before any field arrives.

Two kinds are passed over: an element above the packet's `rdf:RDF`, whose subject is section 7.3,
and one whose content the reader does not interpret, which it therefore cannot call a leaf simple
value either. Both sit past the preview's last page. So a packet this reports on has broken the
sentence; a packet it is silent about has not been shown to keep it. `over` stayed zero on all six
targets and the one remaining corpus miss stayed one.

## 2. The word-boundary row named a walk that exists

`logical-structure/word-boundaries` said asking ISO 19005-2 section 6.7.3.2 needed *a walk of every
page's text-showing operators* and a per-run decision about the script — "neither of which this
crate reaches from a dictionary". The first half had been false for a long time: `crate::survey`
reads every page's text-showing operators and keeps, per font, the strings they drew.

The row stays `Unchecked` and its reason now says what actually blocks it, which is not a walk:

- the rule binds only a language or script system that normally separates words with space
  characters, and nothing in a file says which a run of text is in;
- where it does bind, the subclause's own NOTE puts word boundaries beyond any boundary in the
  file — a single word may span two or more show strings — so only the characters decide, and
  segmenting them needs a lexicon this project has no business carrying.

**Under-reporting is not available here**, which is worth stating because it is this crate's usual
escape: the failure mode is an *absent* space, so a conservative predicate reports nothing at all.

## 3. The single-resource row had a second blocker, and it is the harder one

`metadata/xmp-packets-describe-one-resource` said the reader keeps no `rdf:about`, "which is the
only thing in a packet that says which resource a description is about". The reader half is true.
The clause half was an assumption: ISO 16684-1 section 6.1 states the rule in the **data model**
and says nothing about how a serialised packet shows which resource a description is about. The
subclause that does is section 7.4, past the preview's last page — and the preview never spells
`rdf:about` anywhere. Reading a subject out of that attribute would be this crate supplying the
serialisation rule rather than applying it.

The reason now names both, in order of difficulty. The row is further from closable than it looked,
not nearer.

## 4. The JPEG 2000 device-colour row was two rules wearing one reason

Both parts close their JPEG 2000 subclause with a sentence naming **two routes** by which such an
image comes to use `DeviceGray`, `DeviceRGB` or `DeviceCMYK`: the `ColorSpace` entry of the image
`XObject`, and — in the absence thereof — the colour space defined in the JPEG 2000 data. The row's
reason applied the word *effectively* to the whole sentence. It belongs to the second route only:
an entry naming `/DeviceRGB` is a use of `DeviceRGB`, with nothing ambiguous in it.

So the row is two, the way session 941's amendment row and session 946's serialisation row were
split:

- `graphics/jpeg2000-device-colour-the-image-dictionary-states` — **delegated**, and the delegation
  is real: `crate::survey`'s `image_space` records the `ColorSpace` of every image `XObject` a
  content stream draws, and the six section 6.2.4.3 rows judge what it records. A predicate here
  would report the same failure twice under a clause number that adds nothing.
- `graphics/jpeg2000-device-colour-the-codestream-defines` — the word *effectively*, which neither
  part defines and both send to ISO/IEC 15444-2 in their NOTE 3. `doc/questions/A51` settles that
  the extensions part will not be bought, so this is settled rather than outstanding.

**ADR 0928 names the old row by its old id** and says it "must not be filed under" A51. That
warning survives the split and now attaches to the second of the two only: the first has nothing to
do with A51, and the second is settled *by* it rather than covered by 0928's argument about the
baseline feature set. This ADR amends 0928 to that extent and to no other.

## 5. Three doc comments that said the project holds no ICC text

`ICC.1:1998-09` and `ICC.1:2001-12` arrived in session 950 and the `Check::Unchecked` reason on
`graphics/icc-profiles-conform-to-a-permitted-edition` was updated then. Three doc comments in the
same file were not, and each still said this project holds **none** of the four editions
ISO 19005-2 section 6.2.4.2 names. All three are corrected. A stale doc comment is cheaper than a
stale reason and decays the same way; the checker watches neither.

## What did not move, and why that is the answer

Sixteen rows stay `Unchecked` with reasons this session read and confirmed, in four kinds:

- **A whole clause family**, asked by the rows beneath it: tagged PDF, adherence to the base
  standard, device-independent colour. Nothing would change it; a predicate would restate its
  children.
- **A delegation that is real**: appearance graphics, and now the JPX `ColorSpace` route. Nothing
  would change it either.
- **A text this project does not hold**: ISO/IEC 15444-2, ISO 14533-3, ISO 16684-2,
  ISO/IEC 14496-22, ICC.1:2003-09, and ISO 15076-1 and ISO 16684-1 past their previews. Buying it
  would, where the owner decides to.
- **A fact no document states**: which amendment a file was written against, whether an
  `xmpMM:History` entry was *added* rather than carried, whether a font may lawfully be embedded.
  Nothing a validator can reach.

**Only the third kind is a debt**, and only some of that is a debt anyone intends to pay.
