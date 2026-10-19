# 2020 — An `xmp:Rating` of `1.0` is not the 2005 edition's Integer, and the corpus's pass is settled against it

Session 1592, the archive slot of batch ninety-four. Status: **accepted**. Context: `t3-archive-val`
showed one `over` at ISO 19005-2 section 6.6.2.3.1 since ADR 1991; `crates/pdf-archive/tests/
corpus.rs` now prints the names behind `over` as it already did behind `missed`, and the one name
is veraPDF's `6-6-2-3-1-t07-pass-m.pdf`.

## 1. What the file states, and what the edition part 2 names says of it

The packet's one property past the identification is `xmp:Rating` with the value `1.0`. ADR 1991
reads the XMP Specification part 2 admits as its September 2005 edition, the one its bibliography
names. That edition's XMP Basic schema types `xmp:Rating` a closed choice of Integer, and its basic
value types define an Integer as a decimal numeric string with an optional leading sign, where its
Real is the type that admits a single decimal point. So `1.0` is a Real's spelling and not an
Integer's: the property is predefined, and the value is not of its type. TechNote 0010's A020, which
the working group resolved parts 1 to 3 are read with, holds an XMP value to its type alone — which
is exactly this judgement and nothing more.

**The brief's hypothesis did not hold**: no schema was missed. The 2005 edition does define the
property; it defines it with the type the validator holds it to.

## 2. Why the corpus passes it

The editions after 2005 retype `xmp:Rating` a closed choice of Real, which `1.0` is. The corpus
reads that later type here while reading the 2005 edition for the four properties ADR 1991 read
(none of which any edition defines, so they decided nothing about editions). A later edition is not
the one part 2 names, and the owner's rule of 2026-09-07 sends a conflict with the text to the text.

## 3. What changes

The validator does not: `metadata/properties-use-known-schemas` keeps judging the value. The
corpus comparison records the file as `Ruling::SpecAgainstTheCorpus` with this reading, so `over`
returns to 0 at 2b and the disagreement is a `settled` one. The converter is unchanged too: under
`--authorise metadata-property` it removes the property and names what it held (section 3.9 of
the limits document), and the sibling `-t07-pass-l`, stating `1`, passes as it did.
