# 946 — The obligation that was withdrawn, and a converter's middle stage

Date: 2026-09-10.
ADR: 0947 (the converter's middle stage and the net under it).
Files: `crates/pdf-archive/src/table/metadata.rs`, `crates/pdf-model/src/xmp.rs`,
`crates/pdf-transform/src/archive.rs` (new), `crates/pdf-transform/tests/archive.rs` (new),
`crates/pdf-transform/tests/archive_corpus.rs` (new), `data/icc/sRGB2014.icc` (new), `/NOTICE`,
`doc/third-party-data.md`, `doc/pdf-a-conversion-limits.md`.

The owner downloaded ISO 16684-1 and ISO 15076-1 and asked for the validator finished and the
PDF/A converter built.

## Two previews, and the one that reached far enough

Both downloads are iTeh **STANDARD PREVIEW** files, as the ISO/IEC 15444 ones were in session 940,
and they are not equally useful. ISO 15076-1's is front matter alone — its own contents list puts
section 7.2 on page 19 and the preview ends in the introduction — so the profile header, the
`Profile ID` field and section 7.2.18's MD5 computation are all past its last page. The three ICC
rows keep their reasons unchanged, with the acquisition recorded in them so no later round buys
the same hope twice.

ISO 16684-1's preview carries real clause text from clause 1 to section 7.2. That was enough to
split one `Unchecked` row into five and implement two of them.

**And it withdrew an obligation rather than adding one, which is the finding.** The serialisation
row's reason had named an encoding requirement. Section 7.1 says the opposite: it names UTF-8,
UTF-16 and UTF-32 and puts the choice between them beyond its own scope, handing it to whichever
standard embeds the packet.

That closed the PDF/A-4 miss `6-7-2-1-t01-fail-e`, whose corpus outline expects failure because
the packet is not UTF-8. **No such rule exists, and three documents establish it**, all named in
the adjudication: ISO 16684-1 hands the choice on; ISO 32000-2 §14.3.2 is the standard it hands it
to and states no encoding, its §7.9.2.2 constraint having the *text string type* as its subject —
a string object rather than a stream's contents; and ISO 19005-4 section 6.7.2.1 forbids the
`encoding` **attribute** in a packet header, which is how a packet used to declare UTF-16, so
banning the attribute is not banning the encoding.

Two smaller things the held text bore on. `xml:lang` was compared case-sensitively, where section
6.4 by way of RFC 3066 makes every such comparison case-insensitive — so a packet writing
`X-Default` had no title this reader could find. And a citation that attributed the three
permitted encodings to a section saying considerably less.

The validator now stands at **`over` zero on all six targets** and four miss rows over three
files: two want ISO 15076-1's body or ICC.2, and one is a document that is non-conforming under a
clause this crate does not check, which TechNote 0010 A029 made the right answer in session 941.

## The converter, and where its argument lives

`quorra-transform archive` is three stages — validate, decide, apply — and **the middle one is the
product**. `Decision` has four variants because the limits document's §2, §3 and §4 are three
different answers to a failed requirement rather than three degrees of one, and `Because`
separates an edit the fence forbids from a gap this converter has from a target for which no
conforming file exists. A requirement in neither table is **refused by name**, never passed over
and never answered by a rewrite the converter invented.

Two rules underneath, both ADR 0947's. *Nothing is changed that no failed requirement asked for*,
which makes "an already conforming file is not rewritten" true by construction rather than by
test. And **a file leaves the verb only if it conforms**: stage three assembles in memory,
re-opens the result and holds it to the same target, refusing with the requirements named — and
naming separately any requirement the *source* had met, because a conversion that breaks something
that worked is the worse failure. The cost is stated rather than discovered: peak memory is the
whole output.

The report is a first-class output, not a log, because all four of the owner's permissions are
conditional on it (ADR 0927). It carries the validator's `Unchecked` reasons **word for word**,
which is `A20` applied one layer out.

## The profile, and a licence read from the file

`data/icc/sRGB2014.icc` is what `A18` said to ship. Which of the ICC's four sRGB profiles is
decided by the clause: ISO 19005-2 section 6.2.4.2 names ICC.1:1998-09, ICC.1:2001-12,
ICC.1:2003-09 and ISO 15076-1, and **version 4.2.0 is not among them** — which is exactly what
`sRGB_v4_ICC_preference.icc`, the ICC's headline download, is. The shipped file's own header says
2.0.0.0, conforming to the first text on that list.

The licence was read first-hand, and this project had it second-hand in two places. The owner
pointed at the ICC's registry, which also answered a question `doc/pdf-a-conversion-limits.md`
§10.1 had been carrying as an assumption since it was written: the registered CMYK profiles belong
to the ECI, Idealliance and APTEC rather than to the ICC, what the registry publishes for them is
characterization data rather than profiles, and no terms are stated for any of them.

**The generalisation is the useful part.** The ICC's guidance is that a profile's copyright owner
and terms of use live in its header's `Creator` field and its `cprt` tag. That turns "what may we
redistribute" from a question answered once per file by reading web pages into a field a program
reads — so a converter handed a press profile can tell the user whose it is.
