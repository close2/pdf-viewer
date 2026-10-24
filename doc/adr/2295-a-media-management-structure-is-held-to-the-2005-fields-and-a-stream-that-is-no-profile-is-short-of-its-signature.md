# 2295 — A Media Management structure is held to the 2005 fields, and a stream that is no profile is short of its signature

Session 1730, the archive slot of batch one hundred and twelve. Status: implemented. Context:
`doc/history/1714` (veraPDF 1.30.3 failed `me450f12project10_report`'s 2b output on 6.6.2.3.1 and
6.2.4.2, which this validator passed); ADR 1991 (the September 2005 XMP edition); ADRs 1979, 1990
(the unreadable-profile route).

## 1. ISO 19005-2 section 6.6.2.3.1: veraPDF is right, and the validator moves

The packet veraPDF named is an Illustrator figure's object-level metadata. Its `xmpMM:History`
event states `stEvt:changed`, and its `xmpMM:DerivedFrom` states `stRef:originalDocumentID`. The
subclause admits properties of the predefined schemas of the XMP Specification, and ADR 1991 settled
which edition that is: the September 2005 one ISO 19005-2's bibliography names. Read again, fetched
from the bibliography's own address and kept in the round's scratch only: that edition's
`ResourceEvent` table defines five fields (`action`, `instanceID`, `parameters`, `softwareAgent`,
`when`), its `ResourceRef` table nine, its `Version` five. Neither `changed` nor `originalDocumentID`
is among them. The validator's own reading of the row is that *using* a schema means a value of the
type it gives, and a structure stating a field its type does not define is not a value of that
type — so the row was right in principle and short in what it checked: shape and lexical form,
never a structure's fields.

**Built** (`pdf_archive::table::metadata`): the three Media Management value types, with their field
namespaces and fields, and the five properties that hold them; a structure, or each item of an array
of them, is held to its type's fields, a nested `event` to `ResourceEvent`. One finding per property,
as the shape's finding is. veraPDF reads the same lists (its `XMPConstants`, which agrees and decided
nothing). The edition's other structured types are not yet held to their fields (`doc/todo/72`).

**The converter needed nothing new for the property**: `properties_outside_their_schema` is the same
judgement, so both profiles' `discard` takes the property out. **It needed one thing for the
population**: the packets it cuts were the row's kept findings, capped at 32, so a document whose
packets fail at more places than that left packets uncut and failed the second verdict
(`s41589-021-00892-6`, eighteen packets). Where the row failed at more places than it kept, the cut
now walks every metadata stream the validator walks, each asked again and passed over when clean.

## 2. ISO 19005-2 section 6.2.4.2: veraPDF is right, and the validator moves

The two `ICCBased` streams on pages 156 and 157 are Lab profiles whose header begins 24 bytes late:
bytes 36 to 39 are not `acsp`. Both editions this tree holds of the four the clause names,
ICC.1:1998-09 and ICC.1:2001-12, place the profile file signature there in their header table
(section 6.1 of each), so the stream conforms to neither — and `icc_tag_shortfall` answered `None`
for it, which the row read as a profile one held edition admits. The row's own reading reports a
profile neither held text admits and leaves the two texts not held to the unchecked row beside it;
this follows that reading. ICC.1:2010, which ISO 19005-2 names as the basis of ISO 15076-1, was fetched as
evidence and states the same field.

**Built**: a stream without the signature is short of it, and the converter's existing route for a
profile this reader cannot draw through writes the `/Alternate` where the array was (ADR 1979). The
pages were drawn before and after: identical to HEAD's output, within one level of the source.

## 3. What the census and veraPDF say

`me450f12project10_report` converts to 2b, veraPDF-compliant. At 2b, against HEAD, pdf.js is
unchanged under both profiles and `doc/corpora` goes 343/6/125/16 → 344/6/124/16 under
`as-if-printed` and 302/6/167/16 → 303/6/166/16 under `as-shown`: nothing is newly refused once the
population reaches past the cap, and `143424271191706722106594310729442158189`, which HEAD refused on
the second verdict of this row for exactly that cap, converts (veraPDF cannot open it or its source,
calling both encrypted; neither is). `s41589-021-00892-6` and `102952`, refused on the second verdict
by the first build of this change, convert and pass veraPDF, where HEAD's `s41589` output failed
veraPDF's 6.6.2.3.1. The validator over the veraPDF corpus is identical to HEAD's at every target,
missed 0 and over 0, so that corpus holds no witness of either construction. Part 4 reads ISO
16684-2, which this tree does not hold, and section 1 does not reach it.
