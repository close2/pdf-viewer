# 1309 — A linearised file packs object streams part by part, encrypts under final numbers, and derives its beads

Status: accepted and **built**. Amends ADR 1293 section 5, whose three refusals and one silence
this removes; ADR 1293's sections 1 to 4 and 6 stand.
Context: `crates/pdf-syntax/src/linearize.rs` (`Packer`, `lay_out`, `carrier`, `variable_parts`,
`thread_beads`, `serialize_linearized_encrypted`), `crates/pdf-syntax/src/serialize.rs` (hunks named
below), `crates/pdf-transform/src/optimize.rs` (`write_linearized`), `crates/pdf-transform/tests/linearize.rs`,
`crates/pdf-transform/tests/support/linearized.rs`, `crates/pdf-transform/tests/optimize_corpus.rs`.

## 1. Object streams: a carrier per run, numbered last in its section

F.3.1 states the conditions and F.4.1 the reading, and nothing is left to choose but where the
carriers go. **Each run a hint table describes is packed on its own** — part 4, the first page's own
objects, the outline where it is in part 6, each item of part 6's trailing run and of part 8, each
later page's section, each F.3.10 category, each thumbnail and embedded file group — and a carrier
stands where its first member stood. So every group any table states is still a run of whole items,
a member only moves earlier (F.3.7 (d)'s "each resource object shall precede the stream in which it
is first referenced" survives a move), and a carrier is one item and one shared object group, which
is F.3.1's "shared object references shall be made to the object stream containing a compressed
object". A page that reaches several objects of one carrier names its group once, at the earliest
numerator. Kept out: F.3.1's "the linearization dictionary, the document catalog dictionary, and page
objects", and §7.5.7's own list. No `/Extends`: it is optional, and a chain would be a reference from
one part into another that F.3.7's walk should not have to follow.

**Numbering.** Second group: its items from 1, then the main cross-reference stream, then its
compressed objects. First group: the parameter dictionary, the first-page cross-reference stream,
the items, the compressed objects, and the hint stream last — F.3.1's "highest range of object numbers
within the main and first-page cross-reference sections" and F.3.6's "after the object number for
the last object in the first page, including any objects stored within object streams".

**Both sections are cross-reference streams** wherever object streams are generated, because Table
18's type 2 entry exists only there; otherwise the serializer's `Options::form` decides, as it does
for the ordinary writer. The first-page stream's `/Index` is F.3.4's single subsection; `/T` is the
main stream's offset, Table F.1's sentence for "cross-reference streams exclusively". `/W [1 4 2]`,
uncompressed, so a row's length never moves and the fixed point is ADR 1293's unchanged.

## 2. Encryption: once per object, under its final number, before the layout

§7.6.2's rule runs over each object after numbering and before its references are renumbered (the
handler reads a stream's `/Type` and `/Filter` through the assembly). The dictionary is part 4's last
item, its values all direct. Revision 6's Algorithm 2.A takes nothing from `/ID` — §7.6.4.3.2's step
(e) is revision 4's and earlier — so the identifier stays the digest of the bytes written, and Table
15's "this array and the two byte-strings shall be direct objects and shall be unencrypted" holds by
construction. The hint stream is the one object rendered on every pass: one initialisation vector is
drawn before the first, and AES's length is a function of the plaintext's, so it settles with it.

## 3. F.3.7 (b): derived from the thread chain, and the order is a choice

Table 163 makes a later bead's `/T` optional and Table 31 makes `/B` optional while §12.4.3 says
"shall"; F.3.7 (b) requires both. So "carried as the producer wrote them" does not satisfy (b), and
the writer derives each from §12.4.3's chain from `/F` through `/N`, as Table 31's NOTE 2 says can be
done. A stated `/T` or `/B` is kept. **The synthesised `/B` order is thread by thread in `/Threads`
order, then chain order** — §12.4.3's "drawing order" and Table 31's "natural reading order" are
quantities a writer that reads no content stream does not have; this is a documented choice.

## 4. qpdf, as evidence

`qpdf --check` reads the object-stream and the encrypted shapes as linearised and, under the
password, as `R = 6`, AESv3. One new warning, "an uncompressed object after a compressed one in a
cross-reference stream": the hint stream, numbered last by F.3.6's sentence above. qpdf's own writer
numbers the hint stream before part 6, which F.3.6 does not permit; the annex stands. The page 0
count and packing warnings are ADR 1293's, unchanged. `Q131` is still open and this writer still
packs as F.4.1 says.
